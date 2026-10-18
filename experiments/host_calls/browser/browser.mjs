import {createLibrary, metadata, CeruneHostError} from '/artifacts/library.mjs';

const $ = id => document.getElementById(id);
const messages = document.documentElement.lang === 'ja'
  ? {ready: '呼出し可能', result: '結果', stopped: '停止済み', rejected: '実行前に拒否', loading: '読込み中', loadFailed: '読込み失敗'}
  : {ready: 'Ready', result: 'Result', stopped: 'Stopped', rejected: 'Rejected before execution', loading: 'Loading', loadFailed: 'Load failed'};
let bytes;
let handle;
let position = 10;

function state(name, text) {
  $('status').dataset.state = name;
  $('status').textContent = text;
}
function log(text) {
  $('log').textContent += text + '\n';
}
function reset() {
  position = 10;
  $('position').textContent = String(position);
  $('result').textContent = '—';
  $('diagnostic').textContent = '';
}
async function load() {
  state('loading', messages.loading);
  $('controls').disabled = true;
  try {
    if (!bytes) {
      const response = await fetch('/artifacts/library.wasm');
      if (!response.ok) throw new Error('HTTP ' + response.status);
      bytes = new Uint8Array(await response.arrayBuffer());
    }
    handle = await createLibrary(bytes);
    reset();
    state('ready', messages.ready);
    log(messages.ready);
    $('controls').disabled = false;
  } catch (error) {
    state('load-failed', messages.loadFailed);
    $('diagnostic').textContent = error.message;
  }
}
function call(name, args, accept = () => {}) {
  try {
    const value = handle.functions[name](...args);
    accept(value);
    $('result').textContent = String(value);
    $('diagnostic').textContent = '';
    state('ready', messages.ready);
    log(name + '(' + args.map(String).join(', ') + ') → ' + String(value));
  } catch (error) {
    if (!(error instanceof CeruneHostError)) throw error;
    const rejected = error.phase === 'arguments';
    state(rejected ? 'rejected' : 'stopped', rejected ? messages.rejected : messages.stopped);
    $('diagnostic').textContent = error.phase + '\n' + (error.record ?? error.message);
    log(name + ' → ' + error.phase);
  }
}
$('step').addEventListener('click', () => call('step', [position, 2, 0.5], value => {
  position = value;
  $('position').textContent = String(value);
}));
$('sum').addEventListener('click', () => call('sum_to', [$('limit').valueAsNumber]));
$('bad-argument').addEventListener('click', () => call('checked', ['1.5']));
$('fail').addEventListener('click', () => call('checked', [1.5]));
$('reload').addEventListener('click', load);

// Observation reads generated metadata; it never invokes a Cerune function.
$('metadata').textContent = JSON.stringify(metadata, null, 2);
await load();
