'use strict';
const assert = require('assert');

// Same rule as experiments/subprocess/observation.rs. Independent OS runs can
// observe an empty Job/group at different times. Keep raw stdout in commands.json.
function comparable(stdout) {
    assert(stdout.endsWith('\n'), 'completed Report must end with a newline');
    const start = stdout.lastIndexOf('\n', stdout.length - 2) + 1;
    const report = stdout.slice(start);
    assert(report.startsWith('{origin: {'), 'expected final Report line');
    assert(report.includes(', progress_known: true, exit: Code{value: 7}, failure: None{}, '),
        'only completed fixture success can use the OS comparison rule');
    const cleanup = /, cleanup: Complete\{termination_requested: (true|false)\}, cleanup_reported: Complete\{termination_requested: \1\}\}\n$/;
    assert(cleanup.test(report), 'cleanup must be complete and match the raw report');
    return stdout.slice(0, start) + report.replace(cleanup,
        ', cleanup: Complete{termination_requested: <OS>}, cleanup_reported: Complete{termination_requested: <OS>}}\n');
}
function compareCompleted(left, right, label) {
    assert.strictEqual(comparable(left), comparable(right), label);
}
module.exports = compareCompleted;

if (require.main === module) {
    const report = 'before\n{origin: {node_id: 1, source_id: 3, start: 144, end: 173}, started: Yes{}, stdin_requested: 4, stdin_accepted: 4, stdout_limit: 32, stderr_limit: 32, timeout_ms: 10000, stdout_read: 4, stderr_read: 6, stdout_retained: 4, stderr_retained: 6, progress_known: true, exit: Code{value: 7}, failure: None{}, cleanup: Complete{termination_requested: false}, cleanup_reported: Complete{termination_requested: false}}\n';
    const requested = report.replace(/termination_requested: false/g, 'termination_requested: true');
    compareCompleted(report, requested);
    compareCompleted(requested, report);
    assert(report.includes('termination_requested: false'));
    assert(requested.includes('termination_requested: true'));
    const changes = [
        ['before', 'other output'], ['node_id: 1', 'node_id: 2'],
        ['source_id: 3', 'source_id: 4'], ['start: 144', 'start: 145'],
        ['end: 173', 'end: 174'], ['stdin_accepted: 4', 'stdin_accepted: 3'],
        ['stdout_read: 4', 'stdout_read: 5'], ['stderr_retained: 6', 'stderr_retained: 5'],
        ['progress_known: true', 'progress_known: false'], ['value: 7', 'value: 0'],
        ['failure: None{}', 'failure: Failed{}'], ['cleanup: Complete', 'cleanup: Incomplete'],
        ['cleanup_reported: Complete{termination_requested: true}', 'cleanup_reported: Absent{}'],
        ['cleanup_reported: Complete{termination_requested: true}', 'cleanup_reported: Complete{termination_requested: false}'],
        ['termination_requested: true', 'termination_requested: unknown']
    ];
    for (const [from, to] of changes) {
        assert(requested.includes(from));
        assert.throws(() => compareCompleted(report, requested.replace(from, to)), from);
    }
    assert.throws(() => compareCompleted(report, report.trimEnd()));
    assert.throws(() => compareCompleted(report, report + 'extra\n'));
    console.log('Completed Report: cleanup timing accepted; 17 other differences rejected');
}