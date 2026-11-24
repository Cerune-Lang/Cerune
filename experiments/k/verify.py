"""Compare one completed HIR function with concrete K execution and Lean's proved table."""
import json
import os
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODEL = ROOT / "experiments/k/choose.k"
FIXTURE = ROOT / "experiments/lean/branch.ceru"
ROUTES = {"hir": ["run"], "mir": ["run-mir"], "ssa": ["run-mir", "--ssa"], "vm": ["run-vm"]}


def execute(args, cwd=ROOT, expected=0):
    p = subprocess.run(args, cwd=cwd, capture_output=True, text=True, encoding="utf8")
    if p.returncode != expected:
        raise RuntimeError(f"{args}: exit {p.returncode}, expected {expected}\n{p.stdout}\n{p.stderr}")
    return p


def k_execution(directory, program, definition):
    directory.mkdir()
    model = directory / "choose.k"
    model.write_text(definition, encoding="utf8")
    execute(["kompile", "choose.k", "--backend", "llvm"], cwd=directory)
    raw = execute(["krun", program, "--definition", "choose-kompiled", "--output", "json"], cwd=directory).stdout
    (directory / "result.json").write_text(raw, encoding="utf8")
    return read_events(raw)


def read_events(raw):
    result = json.loads(raw)
    assert (result["format"], result["version"]) == ("KAST", 4)
    state, = [x for x in result["term"]["args"] if x["label"]["name"] == "<state>"]
    cells = {cell["label"]["name"]: cell["args"][0] for cell in state["args"]}
    assert cells["<input>"]["token"] == "256" and cells["<k>"]["items"] == []
    pending = [cells["<events>"]]
    observations = [[] for _ in range(256)]
    while pending:
        term = pending.pop()
        args = term.get("args", [])
        label = term["label"]["name"]
        if label in ("returned", "decided", "integerOverflow"):
            i = int(args[0]["token"])
            assert 0 <= i < 256
            pos = args[-1]
            assert pos["label"]["name"] == "site"
            site = tuple(int(x["token"]) for x in pos["args"])
            if label == "returned":
                event = ("return", int(args[1]["token"]), site)
            elif label == "decided":
                assert args[1]["token"] in ("true", "false")
                event = ("branch", args[1]["token"] == "true", site)
            else:
                event = ("stop", "integer-overflow", site)
            observations[i].append(event)
        else:
            assert label in ("_List_", ".List", "ListItem"), label
            pending.extend(reversed(args))
    return observations


def lean_table(directory):
    execute(["cargo", "run", "--locked", "--quiet", "--example", "lean_verification"])
    generated = ROOT / "target/lean-verification/BranchVerified.lean"
    source = directory / "Table.lean"
    source.write_text(generated.read_text(encoding="utf8") + """
#eval do
  for i in List.range 256 do
    IO.println s!"row {i} {CeruneProof.render (CeruneProof.branchExpected i)}"
""", encoding="utf8")
    output = execute([os.environ.get("CERUNE_TEST_LEAN", "lean"), source]).stdout
    (directory / "lean.txt").write_text(output, encoding="utf8")
    rows = [line for line in output.splitlines() if line.startswith("row ")]
    assert len(rows) == 256
    return rows


def route_observations(directory, command, name):
    binary = ROOT / "target/debug" / ("cerune.exe" if os.name == "nt" else "cerune")
    observations = [[] for _ in range(256)]
    for inputs in (range(254), [254], [255]):
        # Functions precede the fixture's top-level print calls. Preserve the definition bytes.
        src = directory / f"inputs-{inputs[0]}.ceru"
        prefix = FIXTURE.read_bytes()
        calls = "".join(f"print(choose({i}));\n" for i in inputs)
        src.write_bytes(prefix + b"\n" + calls.encode())
        trace = directory / f"{name}-{inputs[0]}.jsonl"
        command_name, *options = command
        p = execute([binary, command_name, src, *options, "--trace", trace,
                     "--diagnostic-format", "runtime-v1"], expected=int(inputs[0] >= 254))
        events = [json.loads(line) for line in trace.read_text(encoding="utf8").splitlines()]
        assert events.pop() == {"schema": "trace-v1", "event": "end", "status": "complete"}
        outputs = iter(map(int, p.stdout.splitlines()))
        # First run the five calls already in branch.ceru, then the inputs for this batch.
        all_inputs = [0, 126, 127, 128, 253, *inputs]
        assert len(events) == 3 * len(all_inputs)
        for n, i in enumerate(all_inputs):
            call, branch, end = events[n*3:n*3+3]
            assert (call["event"], call["function"], branch["event"]) == ("call", "choose", "branch")
            def site(row):
                return tuple(row[k] for k in ("node", "file", "start", "end"))
            if end["event"] == "return":
                assert end["function"] == "choose"
                outcome = ("return", next(outputs), site(end))
            else:
                assert end["event"] == "stop"
                outcome = ("stop", end["code"], site(end))
            if n >= 5:
                observations[i] = [("branch", branch["taken"], site(branch)), outcome]
        assert next(outputs, None) is None
        if inputs[0] >= 254:
            assert "runtime-v1 code=integer-overflow" in p.stderr
        else:
            assert not p.stderr
    return observations


def main():
    (ROOT / "target").mkdir(exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="k-choose-", dir=ROOT / "target"))
    print(f"Artifacts: {directory}", flush=True)
    program = directory / "choose.term"
    program.write_text(execute(["cargo", "run", "--locked", "--quiet", "--example", "k_choose"]).stdout, encoding="utf8")
    execute(["cargo", "build", "--locked", "--quiet", "--bin", "cerune"])
    lean = lean_table(directory)
    rules = MODEL.read_text(encoding="utf8")
    observed = k_execution(directory / "reference", program, rules)
    for i, row in enumerate(observed):
        assert len(row) == 2 and row[0][0] == "branch", (i, row)
        kind, value, (node, file, start, end) = row[1]
        render = f"ok {value}" if kind == "return" else f"{value} node={node} source={file} bytes={start}..{end}"
        assert lean[i] == f"row {i} {render}", (lean[i], row)
    for name, command in ROUTES.items():
        actual = route_observations(directory, command, name)
        for i, (left, right) in enumerate(zip(observed, actual, strict=True)):
            assert left == right, (name, i, left, right)
    print("K, Lean and four executors agree on 256 outcomes; all branch, return and stop sites match the executors.", flush=True)
    before = "andThen(truth(false), _) => truth(false)"
    assert rules.count(before) == 1
    mutant = k_execution(directory / "eager", program, rules.replace(before, "andThen(truth(false), R) => R"))
    assert mutant[128][0][:2] == ("stop", "integer-overflow")
    stop_site = mutant[128][0][2]
    assert FIXTURE.read_bytes()[stop_site[2]:stop_site[3]] == b"value + 128"
    assert observed[128][1][:2] == ("return", 130)
    print("Input 128 detects the eager right operand at value + 128.")


if __name__ == "__main__":
    main()
