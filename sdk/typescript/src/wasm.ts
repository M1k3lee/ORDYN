import { VerifyResult, AttackResult } from './types.js';

interface WasmExports {
  memory: WebAssembly.Memory;
  ordyn_alloc: (size: number) => number;
  ordyn_dealloc: (ptr: number, size: number) => void;
  ordyn_verify_source: (ptr: number, len: number) => number;
  ordyn_execute_attack: (vectorId: number, argVal: number | bigint) => number;
}

let cachedExports: WasmExports | null = null;

export async function initWasm(wasmBinaryOrBuffer?: ArrayBuffer | Uint8Array): Promise<WasmExports> {
  if (cachedExports) return cachedExports;

  let bytes: ArrayBuffer;

  if (wasmBinaryOrBuffer) {
    bytes = wasmBinaryOrBuffer instanceof Uint8Array ? wasmBinaryOrBuffer.buffer as ArrayBuffer : wasmBinaryOrBuffer;
  } else if (typeof process !== 'undefined' && process.versions && process.versions.node) {
    const fs = await import('fs');
    const path = await import('path');
    const { fileURLToPath } = await import('url');
    const dir = path.dirname(fileURLToPath(import.meta.url));
    const wasmPath = path.resolve(dir, '../ordyn_wasm.wasm');
    const buf = fs.readFileSync(wasmPath);
    bytes = buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength);
  } else if (typeof window !== 'undefined' || typeof fetch !== 'undefined') {
    const res = await fetch('ordyn_wasm.wasm');
    bytes = await res.arrayBuffer();
  } else {
    throw new Error('No WASM binary provided and environment is unsupported.');
  }

  const wasmModule = await WebAssembly.instantiate(bytes, {});
  cachedExports = wasmModule.instance.exports as unknown as WasmExports;
  return cachedExports;
}

export async function verifySourceWasm(source: string, wasmBinary?: ArrayBuffer): Promise<VerifyResult> {
  const exports = await initWasm(wasmBinary);
  const t0 = performance.now();

  const encoder = new TextEncoder();
  const encoded = encoder.encode(source);
  const ptr = exports.ordyn_alloc(encoded.length);

  new Uint8Array(exports.memory.buffer, ptr, encoded.length).set(encoded);
  const resPtr = exports.ordyn_verify_source(ptr, encoded.length);

  const rawMem = new Uint8Array(exports.memory.buffer);
  let end = resPtr;
  while (rawMem[end] !== 0) end++;

  const decoder = new TextDecoder();
  const jsonStr = decoder.decode(rawMem.subarray(resPtr, end));
  exports.ordyn_dealloc(ptr, encoded.length);

  const parsed = JSON.parse(jsonStr);
  const elapsedUs = Math.max(10, Math.round((performance.now() - t0) * 1000));

  return {
    success: parsed.success,
    totalDeclarations: parsed.total_declarations || 0,
    entities: parsed.entities || [],
    capabilities: parsed.capabilities || [],
    boundaries: parsed.boundaries || [],
    contractsCount: parsed.contracts_count || 0,
    tirFunctions: parsed.tir_functions || 0,
    stages: parsed.stages || [],
    errorMessage: parsed.error_message,
    tirSummary: parsed.tir_summary,
    elapsedUs,
  };
}

export async function executeAttackWasm(vectorId: number, argVal = 0): Promise<AttackResult> {
  const exports = await initWasm();
  const resPtr = exports.ordyn_execute_attack(vectorId, BigInt(argVal));

  const rawMem = new Uint8Array(exports.memory.buffer);
  let end = resPtr;
  while (rawMem[end] !== 0) end++;

  const decoder = new TextDecoder();
  const jsonStr = decoder.decode(rawMem.subarray(resPtr, end));
  const parsed = JSON.parse(jsonStr);

  return {
    success: parsed.success,
    trapped: parsed.trapped,
    vectorName: parsed.vector_name,
    enforcementLayer: parsed.enforcement_layer,
    kernelVerdict: parsed.kernel_verdict,
    rustErrorType: parsed.rust_error_type,
    diagnosticLines: parsed.diagnostic_lines || [],
    auditHash: parsed.audit_hash,
    bountyClaimed: parsed.bounty_claimed,
  };
}
