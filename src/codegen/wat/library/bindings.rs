use super::Library;
use std::fmt::Write;

pub(super) fn javascript(library: &Library) -> String {
    format!(
        "export const metadata = freezeTree({});\n{}",
        library.export_manifest().trim(),
        RUNTIME
    )
}

pub(super) fn typescript(library: &Library) -> String {
    let mut output = String::from(TYPES);
    output.push_str("\nexport interface LibraryFunctions {\n");
    for export in library.exports() {
        write!(
            output,
            "  readonly {}: (",
            crate::modules::json(export.name())
        )
        .unwrap();
        for (index, _) in export.parameters().iter().enumerate() {
            if index > 0 {
                output.push_str(", ");
            }
            write!(output, "arg{index}: number").unwrap();
        }
        output.push_str(") => number;\n");
    }
    output.push_str("}\n");
    output
}

const TYPES: &str = r#"export type HostErrorPhase = 'compile' | 'link' | 'instantiate' | 'arguments' | 'language' | 'trap' | 'host' | 'closed';
export interface RuntimeFailure {
  readonly code: string;
  readonly node: number;
  readonly file: number;
  readonly start: number;
  readonly end: number;
}
export class CeruneHostError extends Error {
  readonly phase: HostErrorPhase;
  readonly functionName: string | undefined;
  readonly record: string | undefined;
  readonly failure: RuntimeFailure | undefined;
  constructor(phase: HostErrorPhase, message: string, details?: {
    functionName?: string; record?: string; failure?: RuntimeFailure; cause?: unknown;
  });
}
export interface AbiType {
  readonly cerune: 'i32' | 'f64';
  readonly wasm: 'i32' | 'f64';
  readonly internal: 'i64' | 'f64';
}
export interface ExportDescriptor {
  readonly name: string;
  readonly function: number;
  readonly internal_name: string;
  readonly wrapper: string;
  readonly parameters: readonly AbiType[];
  readonly result: AbiType;
  readonly origin: {readonly file: number; readonly start: number; readonly end: number};
}
export const metadata: {
  readonly schema: 'cerune-wat-library-v1';
  readonly exports: readonly ExportDescriptor[];
};
export interface Library {
  readonly functions: LibraryFunctions;
}
export function createLibrary(bytes: Uint8Array): Promise<Library>;
"#;

const RUNTIME: &str = r#"// Generated checked binding for the accompanying Cerune WAT library.
// metadata is immutable observation data; it does not invoke the program.
function freezeTree(value) {
  if (value !== null && typeof value === 'object') {
    for (const child of Object.values(value)) freezeTree(child);
    Object.freeze(value);
  }
  return value;
}

export class CeruneHostError extends Error {
  constructor(phase, message, details = {}) {
    super(message, {cause: details.cause});
    this.name = 'CeruneHostError';
    this.phase = phase;
    this.functionName = details.functionName;
    this.record = details.record;
    this.failure = details.failure;
    Object.freeze(this);
  }
}

function runtimeFailure(diagnostic) {
  const record = diagnostic.trim().replace(/^cerune: /, '');
  const match = /^runtime-v1 code=([a-z][a-z0-9-]*) node=(\d+)(?: file=(\d+))? bytes=(\d+)\.\.(\d+)$/.exec(record);
  if (!match) return undefined;
  const [node, file, start, end] = [match[2], match[3] ?? '0', match[4], match[5]].map(Number);
  if (![node, file, start, end].every(Number.isSafeInteger) || start > end) return undefined;
  return {record, failure: Object.freeze({code: match[1], node, file, start, end})};
}

function valid(value, type) {
  return typeof value === 'number' && (type === 'f64' ||
    (Number.isInteger(value) && value >= -2147483648 && value <= 2147483647 && !Object.is(value, -0)));
}

export async function createLibrary(bytes) {
  let module;
  try {
    // Accept bytes only: callers explicitly fetch/read the accompanying artifact.
    if (!(bytes instanceof Uint8Array)) throw new TypeError('expected Uint8Array');
    module = await WebAssembly.compile(bytes);
  } catch (cause) {
    throw new CeruneHostError('compile', 'Wasm compilation failed', {cause});
  }
  const exports = WebAssembly.Module.exports(module);
  const imports = WebAssembly.Module.imports(module);
  if (exports.length !== metadata.exports.length ||
      metadata.exports.some(e => !exports.some(x => x.name === e.name && x.kind === 'function')) ||
      imports.length > 1 ||
      imports.some(i => i.module !== 'cerune' || i.name !== 'write_error_byte' || i.kind !== 'function')) {
    throw new CeruneHostError('link', 'Wasm exports or imports do not match the binding');
  }

  let diagnostic = '';
  let instance;
  try {
    instance = await WebAssembly.instantiate(module, {
      cerune: {write_error_byte(byte) {
        if (!Number.isInteger(byte) || byte < 0 || byte > 127 || diagnostic.length >= 4096) {
          throw new Error('invalid runtime diagnostic bytes');
        }
        diagnostic += String.fromCharCode(byte);
      }}
    });
    if (diagnostic) throw new Error('unexpected diagnostic during instantiation');
  } catch (cause) {
    throw new CeruneHostError('instantiate', 'Wasm instantiation failed', {cause});
  }

  let closed = false;
  const functions = Object.create(null);
  for (const entry of metadata.exports) {
    functions[entry.name] = (...args) => {
      const details = {functionName: entry.name};
      if (closed) throw new CeruneHostError('closed', 'library handle is closed', details);
      if (args.length !== entry.parameters.length) {
        throw new CeruneHostError('arguments', 'incorrect argument count', details);
      }
      for (let i = 0; i < args.length; i++) {
        if (!valid(args[i], entry.parameters[i].cerune)) {
          throw new CeruneHostError('arguments', 'incorrect argument type or range at index ' + i, details);
        }
      }
      diagnostic = '';
      let value;
      try {
        value = instance.exports[entry.name](...args);
      } catch (cause) {
        closed = true;
        const failure = runtimeFailure(diagnostic);
        if (cause instanceof WebAssembly.RuntimeError && failure) {
          throw new CeruneHostError('language', 'Cerune execution stopped', {...details, ...failure, cause});
        }
        const phase = diagnostic || !(cause instanceof WebAssembly.RuntimeError) ? 'host' : 'trap';
        throw new CeruneHostError(phase, 'Wasm execution failed', {...details, cause});
      }
      if (diagnostic || !valid(value, entry.result.cerune)) {
        closed = true;
        throw new CeruneHostError('host', 'Wasm result or diagnostic violates the binding contract', details);
      }
      return value;
    };
  }
  return Object.freeze({functions: Object.freeze(functions)});
}
"#;
