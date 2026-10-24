mod format_cli;
mod observation_bundle;
mod program_arguments;

use cerune_lang::{diagnostic::Diagnostic, modules::Compilation};
use std::{env, fs, path::PathBuf, process};

fn main() {
    if let Err(error) = run() {
        eprintln!("cerune: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);

    let Some(command) = args.next() else {
        print_help();
        return Ok(());
    };

    let command = command
        .into_string()
        .map_err(|_| "invalid Unicode in Cerune command")?;
    let (compiler_args, program_arguments, argument_limits) =
        program_arguments::parse(&command, args)?;
    let (rest, string_heap_limit, array_heap_limit) = parse_heap_limits(compiler_args, &command)?;
    let mut args = rest.into_iter();
    match command.as_str() {
        "fmt" => format_cli::run(args),
        // コードの構文チェック
        "check" => {
            let input = required_path(args.next(), "missing input file")?;

            reject_extra(args)?;

            let source = read_source(&input)?;

            render_compilation_result(cerune_lang::semantic::check(&source.program), &source)?;

            println!("OK {}", input.display());

            Ok(())
        }

        // 依存ファイルと正確な本文を明示的に出力します。
        "emit-sources" => {
            let input = required_path(args.next(), "missing input file")?;
            let rest: Vec<String> = args.collect();
            let output =
                parse_output_option(&rest, "cerune emit-sources <file> [-o <sources.json>]")?;
            let source = read_source(&input)?;
            ir_for(&source, string_heap_limit, array_heap_limit)?;
            write_or_print(output, source.source_manifest())
        }

        // 一回の共通frontendから生成した各段階をまとめて保存します。
        "observe" => {
            let input = required_path(args.next(), "missing input file")?;
            let (rest, ssa) = parse_ssa_option(args.collect())?;
            let (rest, lir) = parse_flag(rest, "--lir")?;
            let (output, target, _) = parse_native_options(&rest, "observe", "observation/")?;
            let output = output.ok_or("observe requires -o <new-directory>")?;
            let target = target.ok_or("observe requires an explicit --target")?;
            let target = cerune_lang::codegen::x86_64::Target::parse(&target)
                .ok_or("unsupported observation target; expected x86_64-unknown-linux-gnu or x86_64-pc-windows-msvc")?;
            let source = read_source(&input)?;
            let hir = ir_for(&source, string_heap_limit, array_heap_limit)?;
            observation_bundle::save(&source, &hir, target, &output, ssa, lir)
        }

        // Cerune IR 生成
        "emit-ir" => {
            let input = required_path(args.next(), "missing input file")?;

            let rest: Vec<String> = args.collect();

            let output = parse_output_option(&rest, "cerune emit-ir <file> [-o <output.ceir>]")?;

            let source = read_source(&input)?;

            let ir = render_compilation_result(
                Ok(cerune_lang::ir::text::emit(&ir_for(
                    &source,
                    string_heap_limit,
                    array_heap_limit,
                )?)),
                &source,
            )?;

            write_or_print(output, ir)
        }

        // 基準の非SSA MIR、または明示的に変換したSSAと元MIRを観測します。
        "emit-mir" => {
            let input = required_path(args.next(), "missing input file")?;
            let (rest, ssa) = parse_ssa_option(args.collect())?;
            let output =
                parse_output_option(&rest, "cerune emit-mir <file> [--ssa] [-o <output.txt>]")?;
            let source = read_source(&input)?;
            let hir = ir_for(&source, string_heap_limit, array_heap_limit)?;
            let mir = render_compilation_result(
                cerune_lang::mir::lower(&hir).map_err(|e| e.diagnostic()),
                &source,
            )?;
            let text = if ssa {
                let ssa = ssa_for(&mir, &source)?;
                render_compilation_result(
                    cerune_lang::mir::ssa::text::emit(&ssa).map_err(|e| e.diagnostic()),
                    &source,
                )?
            } else {
                cerune_lang::mir::text::emit(&mir)
            };
            write_or_print(output, text)
        }

        // C コード生成
        "emit-c" => {
            let input = required_path(args.next(), "missing input file")?;

            let rest: Vec<String> = args.collect();

            let output = parse_output_option(&rest, "cerune emit-c <file> [-o <output.c>]")?;

            let source = read_source(&input)?;

            let c = render_compilation_result(
                cerune_lang::codegen::emit_c(&ir_for(
                    &source,
                    string_heap_limit,
                    array_heap_limit,
                )?),
                &source,
            )?;

            write_or_print(output, c)
        }

        // LLVM コード生成
        "emit-llvm" => {
            let input = required_path(args.next(), "missing input file")?;

            let rest: Vec<String> = args.collect();

            let (output, target, annotate_origins) =
                parse_native_options(&rest, "emit-llvm", "ll")?;
            let target = match target.as_deref() {
                None => None,
                Some(value) => Some(cerune_lang::codegen::llvm::Target::parse(value).ok_or_else(|| {
                    format!("unsupported LLVM target `{value}`; expected x86_64-unknown-linux-gnu or x86_64-pc-windows-msvc")
                })?),
            };

            let source = read_source(&input)?;

            let llvm = render_compilation_result(
                cerune_lang::codegen::llvm::emit_llvm_with_argument_limits(
                    &ir_for(&source, string_heap_limit, array_heap_limit)?,
                    cerune_lang::codegen::llvm::Options {
                        target,
                        annotate_origins,
                    },
                    argument_limits,
                ),
                &source,
            )?;

            write_or_print(output, llvm)
        }

        // WAT コード生成
        "emit-wat" => {
            let input = required_path(args.next(), "missing input file")?;

            let rest: Vec<String> = args.collect();

            let output = parse_output_option(&rest, "cerune emit-wat <file> [-o <output.wat>]")?;

            let source = read_source(&input)?;

            let wat = render_compilation_result(
                cerune_lang::codegen::wat::emit_wat_with_argument_limits(
                    &ir_for(&source, string_heap_limit, array_heap_limit)?,
                    argument_limits,
                ),
                &source,
            )?;

            write_or_print(output, wat)
        }

        // QBE コード生成
        "emit-qbe" => {
            let input = required_path(args.next(), "missing input file")?;

            let rest: Vec<String> = args.collect();

            let (output, target, _) = parse_native_options(&rest, "emit-qbe", "ssa")?;
            let target = match target.as_deref() {
                None => None,
                Some(value) => Some(cerune_lang::codegen::qbe::Target::parse(value).ok_or_else(
                    || {
                        format!(
                            "unsupported QBE target `{value}`; expected x86_64-unknown-linux-gnu or x86_64-pc-windows-msvc"
                        )
                    },
                )?),
            };

            let source = read_source(&input)?;

            let qbe = render_compilation_result(
                cerune_lang::codegen::qbe::emit_qbe_with_argument_limits(
                    &ir_for(&source, string_heap_limit, array_heap_limit)?,
                    target,
                    argument_limits,
                ),
                &source,
            )?;

            write_or_print(output, qbe)
        }

        // Native LIRの構造と出典を観測します。
        "emit-lir" => {
            let input = required_path(args.next(), "missing input file")?;
            let (rest, ssa) = parse_ssa_option(args.collect())?;
            let (output, target, _) = parse_native_options(&rest, "emit-lir", "lir.txt")?;
            let target = target.ok_or("emit-lir requires an explicit --target")?;
            let target = cerune_lang::codegen::x86_64::Target::parse(&target)
                .ok_or("unsupported native LIR target")?;
            let source = read_source(&input)?;
            let hir = ir_for(&source, string_heap_limit, array_heap_limit)?;
            let mir = native_mir_for(&source, &hir, ssa)?;
            let lir = render_compilation_result(
                cerune_lang::codegen::x86_64::lower_mir_with_argument_limits(
                    &mir,
                    target,
                    argument_limits,
                ),
                &source,
            )?;
            let text =
                render_compilation_result(cerune_lang::codegen::x86_64::lir::emit(&lir), &source)?;
            write_or_print(output, text)
        }

        // Direct Assembly コード生成
        "emit-asm" => {
            let input = required_path(args.next(), "missing input file")?;

            let rest: Vec<String> = args.collect();

            let (rest, ssa) = parse_ssa_option(rest)?;
            let (output, target, annotate_origins) = parse_native_options(&rest, "emit-asm", "s")?;
            let target = match target.as_deref() {
                None => cerune_lang::codegen::x86_64::Target::X86_64PcWindowsMsvc,
                Some(value) => cerune_lang::codegen::x86_64::Target::parse(value).ok_or_else(|| format!("unsupported assembly target `{value}`; expected x86_64-unknown-linux-gnu or x86_64-pc-windows-msvc"))?,
            };

            let source = read_source(&input)?;

            let hir = ir_for(&source, string_heap_limit, array_heap_limit)?;
            let mir = native_mir_for(&source, &hir, ssa)?;
            let asm = render_compilation_result(
                cerune_lang::codegen::x86_64::lower_mir_with_argument_limits(
                    &mir,
                    target,
                    argument_limits,
                )
                .and_then(|lir| {
                    cerune_lang::codegen::x86_64::emit_asm_from_lir(&lir, annotate_origins)
                }),
                &source,
            )?;

            write_or_print(output, asm)
        }

        // バイナリは出力先とターゲットを必須にし、端末へ暗黙に書き出しません。
        "emit-obj" => {
            let input = required_path(args.next(), "missing input file")?;
            let rest: Vec<String> = args.collect();
            let (rest, ssa) = parse_ssa_option(rest)?;
            let (output, target, origins) = parse_native_options(&rest, "emit-obj", "o")?;
            let output = output.ok_or("emit-obj requires -o <output.o>")?;
            let target = target.ok_or("emit-obj requires an explicit --target")?;
            let target = cerune_lang::codegen::x86_64::Target::parse(&target)
                .ok_or("unsupported native object target")?;
            let source = read_source(&input)?;
            let hir = ir_for(&source, string_heap_limit, array_heap_limit)?;
            let mir = native_mir_for(&source, &hir, ssa)?;
            let bytes = render_compilation_result(
                cerune_lang::codegen::x86_64::lower_mir_with_argument_limits(
                    &mir,
                    target,
                    argument_limits,
                )
                .and_then(|lir| cerune_lang::codegen::x86_64::emit_object_from_lir(&lir, origins)),
                &source,
            )?;
            fs::write(&output, bytes)
                .map_err(|e| format!("failed to write {}: {e}", output.display()))
        }

        // Cerune Bytecode 生成
        "emit-bytecode" => {
            let input = required_path(args.next(), "missing input file")?;

            let rest: Vec<String> = args.collect();

            let output =
                parse_output_option(&rest, "cerune emit-bytecode <file> [-o <output.cebc>]")?;

            let source = read_source(&input)?;

            let bytecode = render_compilation_result(
                cerune_lang::bytecode::lower(&ir_for(
                    &source,
                    string_heap_limit,
                    array_heap_limit,
                )?)
                .map(|program| cerune_lang::bytecode::format_program(&program)),
                &source,
            )?;

            write_or_print(output, bytecode)
        }

        // 同じ完成済みIRから、明示された実行経路を選びます。
        "run" | "run-ir" | "run-vm" | "run-mir" => {
            let input = required_path(args.next(), "missing input file")?;
            let rest: Vec<_> = args.collect();
            let (rest, ssa) = if command == "run-mir" {
                parse_ssa_option(rest)?
            } else {
                (rest, false)
            };
            let runtime_format = match rest.as_slice() {
                [] => false,
                [flag, format] if flag == "--diagnostic-format" && format == "runtime-v1" => true,
                _ => {
                    return Err(format!(
                        "usage: cerune {command} <file> [--diagnostic-format runtime-v1]"
                    ));
                }
            };

            let source = read_source(&input)?;

            let ir = ir_for(&source, string_heap_limit, array_heap_limit)?;
            let stdout = std::io::stdout();
            let mut output = cerune_lang::runtime::output::Output::new(stdout.lock());
            let mut stdin = cerune_lang::runtime::input::stdin();
            let mut result = if command == "run-mir" {
                let mir = render_compilation_result(
                    cerune_lang::mir::lower(&ir).map_err(|e| e.diagnostic()),
                    &source,
                )?;
                if ssa {
                    let ssa = ssa_for(&mir, &source)?;
                    cerune_lang::ssa_executor::run_with_io_and_arguments(
                        &ssa,
                        &mut stdin,
                        &mut output,
                        &program_arguments,
                    )
                    .map_err(|error| {
                        if runtime_format
                            && let cerune_lang::ssa_executor::ErrorKind::Arguments(failure) =
                                error.kind()
                        {
                            return failure.record();
                        }
                        if runtime_format && let Some(failure) = error.runtime_failure() {
                            return failure.record();
                        }
                        source.render(&error.diagnostic())
                    })
                } else {
                    cerune_lang::mir_executor::run_with_io_and_arguments(
                        &mir,
                        &mut stdin,
                        &mut output,
                        &program_arguments,
                    )
                    .map_err(|error| {
                        if runtime_format
                            && let cerune_lang::mir_executor::ErrorKind::Arguments(failure) =
                                error.kind()
                        {
                            return failure.record();
                        }
                        if runtime_format && let Some(failure) = error.runtime_failure() {
                            return failure.record();
                        }
                        source.render(&error.diagnostic())
                    })
                }
            } else if command != "run-vm" {
                cerune_lang::ir_executor::run_with_io_and_arguments(
                    &ir,
                    &mut stdin,
                    &mut output,
                    &program_arguments,
                )
                .map_err(|error| {
                    if runtime_format
                        && let cerune_lang::ir_executor::ErrorKind::Arguments(failure) =
                            error.kind()
                    {
                        return failure.record();
                    }
                    if runtime_format && let Some(failure) = error.runtime_failure() {
                        return failure.record();
                    }
                    source.render(&error.diagnostic())
                })
            } else {
                let bytecode =
                    render_compilation_result(cerune_lang::bytecode::lower(&ir), &source)?;
                cerune_lang::run_bytecode_with_io_and_arguments(
                    &bytecode,
                    &mut stdin,
                    &mut output,
                    &program_arguments,
                )
                .map_err(|error| {
                    if runtime_format
                        && let cerune_lang::vm::VmErrorKind::Arguments(failure) =
                            error.vm_error().kind()
                    {
                        return failure.record();
                    }
                    if runtime_format && let Some(failure) = error.runtime_failure() {
                        return failure.record();
                    }
                    source.render_execution(&error)
                })
            };

            if runtime_format && let Some(failure) = stdin.failure() {
                result = Err(failure.record());
            }
            if runtime_format && let Some(failure) = output.failure() {
                result = Err(failure.record());
            }
            // 出力済みの内容は再送しません。既存の停止理由もflush失敗で上書きしません。
            let flush = if output.failure().is_none() {
                output.flush().map_err(|e| {
                    if runtime_format {
                        e.record()
                    } else {
                        e.to_string()
                    }
                })
            } else {
                Ok(())
            };
            match (result, flush) {
                (Err(error), Err(flush)) => Err(format!("{error}\n{flush}")),
                (Err(error), _) | (_, Err(error)) => Err(error),
                (Ok(()), Ok(())) => Ok(()),
            }
        }

        "--version" | "-V" | "version" => {
            println!("cerune {}", env!("CARGO_PKG_VERSION"));

            Ok(())
        }

        "--help" | "-h" | "help" => {
            print_help();
            Ok(())
        }

        other => Err(format!("unknown command `{other}`")),
    }
}

fn render_compilation_result<T>(
    result: Result<T, Diagnostic>,
    source: &Compilation,
) -> Result<T, String> {
    result.map_err(|diagnostic| source.render(&diagnostic))
}

fn native_mir_for(
    source: &Compilation,
    hir: &cerune_lang::ir::Program,
    ssa: bool,
) -> Result<cerune_lang::mir::Program, String> {
    let mir = render_compilation_result(
        cerune_lang::mir::lower(hir).map_err(|e| e.diagnostic()),
        source,
    )?;
    if ssa {
        let ssa = ssa_for(&mir, source)?;
        Ok(render_compilation_result(
            cerune_lang::mir::ssa::lower(&ssa).map_err(|e| e.diagnostic()),
            source,
        )?
        .program)
    } else {
        Ok(mir)
    }
}

fn ssa_for(
    mir: &cerune_lang::mir::Program,
    source: &Compilation,
) -> Result<cerune_lang::mir::ssa::Program, String> {
    render_compilation_result(
        cerune_lang::mir::ssa::construct(mir).map_err(|e| e.diagnostic()),
        source,
    )
}

// --ssaを受け付ける経路だけが使います。ほかのオプションの値は解釈しません。
fn parse_ssa_option(args: Vec<String>) -> Result<(Vec<String>, bool), String> {
    parse_flag(args, "--ssa")
}
fn parse_flag(args: Vec<String>, flag: &str) -> Result<(Vec<String>, bool), String> {
    let mut rest = Vec::new();
    let mut present = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == flag {
            if present {
                return Err(format!("duplicate {flag}"));
            }
            present = true;
        } else {
            let takes_value = matches!(
                arg.as_str(),
                "-o" | "--output" | "--target" | "--diagnostic-format"
            );
            rest.push(arg);
            if takes_value && let Some(value) = args.next() {
                rest.push(value);
            }
        }
    }
    Ok((rest, present))
}

fn ir_for(
    source: &Compilation,
    string_heap_limit: u64,
    array_heap_limit: u64,
) -> Result<cerune_lang::ir::Program, String> {
    let mut program = render_compilation_result(source.to_ir(), source)?;
    program.string_heap_limit = string_heap_limit;
    program.array_heap_limit = array_heap_limit;
    Ok(program)
}

fn parse_heap_limits(args: Vec<String>, command: &str) -> Result<(Vec<String>, u64, u64), String> {
    let mut rest = Vec::new();
    let mut limits = [None, None];
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let index = match arg.as_str() {
            "--string-heap-limit" => 0,
            "--array-heap-limit" => 1,
            _ => {
                rest.push(arg);
                continue;
            }
        };
        if !matches!(
            command,
            "run"
                | "run-ir"
                | "run-vm"
                | "run-mir"
                | "emit-ir"
                | "emit-mir"
                | "observe"
                | "emit-bytecode"
                | "emit-c"
                | "emit-llvm"
                | "emit-qbe"
                | "emit-wat"
                | "emit-lir"
                | "emit-asm"
                | "emit-obj"
        ) {
            return Err(format!("{arg} is not supported by {command}"));
        }
        if limits[index].is_some() {
            return Err(format!("duplicate {arg}"));
        }
        let text = args
            .next()
            .ok_or_else(|| format!("missing byte count after {arg}"))?;
        let value = text
            .parse::<u64>()
            .ok()
            .filter(|n| *n <= i64::MAX as u64)
            .filter(|_| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
            .ok_or_else(|| {
                format!("{arg} expects a decimal byte count from 0 to 9223372036854775807")
            })?;
        limits[index] = Some(value);
    }
    Ok((
        rest,
        limits[0].unwrap_or(cerune_lang::ir::DEFAULT_STRING_HEAP_LIMIT),
        limits[1].unwrap_or(cerune_lang::ir::DEFAULT_ARRAY_HEAP_LIMIT),
    ))
}

fn required_path(value: Option<String>, message: &str) -> Result<PathBuf, String> {
    value.map(PathBuf::from).ok_or_else(|| message.to_owned())
}

fn read_source(path: &std::path::Path) -> Result<Compilation, String> {
    cerune_lang::modules::load(path).map_err(|error| error.render())
}

fn reject_extra(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    if let Some(extra) = args.next() {
        Err(format!("unexpected argument `{extra}`"))
    } else {
        Ok(())
    }
}

fn parse_output_option(args: &[String], usage: &str) -> Result<Option<PathBuf>, String> {
    match args {
        [] => Ok(None),

        [flag, path] if flag == "-o" || flag == "--output" => Ok(Some(PathBuf::from(path))),

        _ => Err(format!("usage: {usage}")),
    }
}

fn parse_native_options(
    args: &[String],
    route: &str,
    extension: &str,
) -> Result<(Option<PathBuf>, Option<String>, bool), String> {
    let mut output = None;
    let mut target = None;
    let mut annotate_origins = false;
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        if flag == "--annotate-origins"
            && matches!(route, "emit-llvm" | "emit-asm" | "emit-obj")
            && !annotate_origins
        {
            annotate_origins = true;
            continue;
        }
        match (flag.as_str(), args.next()) {
            ("-o" | "--output", Some(path)) if output.is_none() => {
                output = Some(PathBuf::from(path));
            }
            ("--target", Some(value)) if target.is_none() => {
                target = Some(value.clone());
            }
            _ => {
                return Err(if route == "observe" {
                    "usage: cerune observe <file> [--ssa] [--lir] --target <triple> -o <new-directory>"
                        .to_owned()
                } else {
                    format!(
                        "usage: cerune {route} <file> [--target <triple>] [-o <output.{extension}>]"
                    )
                });
            }
        }
    }
    Ok((output, target, annotate_origins))
}

fn write_or_print(output: Option<PathBuf>, content: String) -> Result<(), String> {
    match output {
        Some(path) => fs::write(&path, content)
            .map_err(|e| format!("failed to write {}: {e}", path.display())),

        None => {
            print!("{content}");
            Ok(())
        }
    }
}

fn print_help() {
    println!(
        "Cerune {}\n\n\
         A small experimental language with observable code generation.\n\n\
         USAGE:\n\
           cerune check <file>\n\
           cerune fmt <file> [--let preserve|show|hide] [--check | --write | -o <file>]\n\
           cerune emit-sources <file> [-o <sources.json>]\n\
           cerune emit-ir <file> [-o <output.ceir>]\n\
           cerune emit-c <file> [-o <output.c>]\n\
           cerune emit-llvm <file> [--target <triple>] [--annotate-origins] [-o <output.ll>]\n\
           cerune emit-wat <file> [-o <output.wat>]\n\
           cerune emit-qbe <file> [--target <triple>] [-o <output.ssa>]\n\
           cerune emit-lir <file> [--ssa] --target <triple> [-o <output.lir.txt>]\n\
           cerune emit-asm <file> [--ssa] [--target <triple>] [--annotate-origins] [-o <output.s>]\n\
           cerune emit-obj <file> [--ssa] --target <triple> [--annotate-origins] -o <output.o>\n\
           cerune observe <file> [--ssa] [--lir] --target <triple> -o <new-directory>\n\
           cerune emit-mir <file> [--ssa] [-o <output.txt>]\n\
           cerune emit-bytecode <file> [-o <output.cebc>]\n\
           cerune run <file> [--diagnostic-format runtime-v1] [-- <arguments...>]\n\
           cerune run-ir <file> [--diagnostic-format runtime-v1] [-- <arguments...>]\n\
           cerune run-mir <file> [--ssa] [--diagnostic-format runtime-v1] [-- <arguments...>]\n\
           cerune run-vm <file> [--diagnostic-format runtime-v1] [-- <arguments...>]\n\
           cerune --version\n\n\
         run / run-ir: direct Cerune IR execution; run-mir: HIR -> MIR execution; run-vm: Bytecode -> VM.\n\
         run / run-ir / run-mir / run-vm / observe / emit-* (except emit-sources): --string-heap-limit <bytes> / --array-heap-limit <bytes>\n\
         --ssa: explicit scalar SSA construction for emit-mir / run-mir / emit-lir / emit-asm / emit-obj / observe; no optimization.\n\
         run commands / emit-llvm / emit-qbe / emit-wat / emit-lir / emit-asm / emit-obj: --argument-count-limit <count> / --argument-byte-limit <bytes>; defaults: 4096 / 1048576.\n\
         First -- separates all program arguments from Cerune options.\n\
         Default: 67108864 bytes per heap; compile-time string budget is independent.\n",
        env!("CARGO_PKG_VERSION")
    );
}
