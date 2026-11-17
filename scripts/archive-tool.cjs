'use strict';
const path = require('node:path');
// Select the host's archive utility independently of the program's target.
function archiveTool(platform = process.platform, environment = process.env) {
    if (platform !== 'win32') return 'tar';
    const systemRoot = environment.SystemRoot;
    if (!systemRoot || !path.win32.isAbsolute(systemRoot)) throw Error('absolute SystemRoot is required for Windows tar');
    return path.win32.join(systemRoot, 'System32', 'tar.exe');
}
module.exports = {archiveTool};
