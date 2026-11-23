'use strict';

function checkResults(jobs) {
    const entries = Object.entries(jobs);
    if (!entries.length) throw new Error('CI reported no prerequisite jobs');
    const problems = entries.filter(([, job]) => job.result !== 'success');
    if (problems.length) {
        throw new Error(problems.map(([id, job]) => id + ': ' + job.result).join(', '));
    }
}

if (require.main === module) {
    checkResults(JSON.parse(process.env.CI_JOB_RESULTS));
}

module.exports = {checkResults};
