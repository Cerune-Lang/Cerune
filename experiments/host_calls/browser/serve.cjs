const http = require('node:http');
const fs = require('node:fs/promises');
const path = require('node:path');

async function serve(artifacts, port = 0) {
  const routes = new Map();
  for (const file of ['index.html', 'index.en.html', 'browser.mjs', 'style.css']) {
    routes.set('/' + file, path.join(__dirname, file));
  }
  routes.set('/', path.join(__dirname, 'index.html'));
  for (const file of ['library.mjs','library.d.mts','library.wasm','library.wat','exports.json','sources.json','source.ceir','source.cebc','plain.wat']) {
    routes.set('/artifacts/' + file, path.resolve(artifacts, file));
  }
  const mime = {'.html':'text/html; charset=utf-8','.mjs':'text/javascript; charset=utf-8','.css':'text/css; charset=utf-8','.json':'application/json; charset=utf-8','.wasm':'application/wasm'};
  const server = http.createServer(async (req, res) => {
    const file = routes.get(new URL(req.url, 'http://localhost').pathname);
    if (!file || !['GET','HEAD'].includes(req.method)) { res.writeHead(404).end(); return; }
    try {
      const body = await fs.readFile(file);
      res.writeHead(200, {'Content-Type':mime[path.extname(file)] ?? 'text/plain; charset=utf-8','Cache-Control':'no-store'});
      res.end(req.method === 'HEAD' ? undefined : body);
    } catch {
      res.writeHead(404).end('Generate the library artifacts first.');
    }
  });
  await new Promise((resolve,reject) => { server.once('error',reject); server.listen(port,'127.0.0.1',resolve); });
  return server;
}
module.exports = {serve};
if (require.main === module) {
  const directory = process.argv[2];
  if (!directory) throw new Error('usage: serve.cjs artifact-directory [port]');
  serve(directory, Number(process.argv[3] ?? 0)).then(server => {
    console.log('http://127.0.0.1:' + server.address().port + '/');
  }).catch(error => {console.error(error); process.exitCode=1;});
}
