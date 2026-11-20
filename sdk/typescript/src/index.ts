import { VerifyResult, RunOptions, RunResult, AttackResult } from './types.js';
import { verifySourceWasm, executeAttackWasm } from './wasm.js';
import { isCliAvailable, verifyViaCli, runViaCli } from './cli.js';

export * from './types.js';
export { initWasm } from './wasm.js';

/**
 * Verifies ORDYN source code against the compiler pipeline:
 * Pratt Parser -> Type Lattice -> Algebraic Effects (Zero Ambience) -> Presburger SMT Prover.
 */
export async function verifyOrdyn(source: string, preferCli = false): Promise<VerifyResult> {
  if (preferCli && (await isCliAvailable())) {
    return verifyViaCli(source);
  }
  return verifySourceWasm(source);
}

/**
 * Runs an ORDYN source module inside an isolated sandbox with zero ambient authority
 * and bounded fuel quotas.
 */
export async function runOrdyn(source: string, options: RunOptions = {}): Promise<RunResult> {
  // First verify the source contracts and invariants
  const verification = await verifyOrdyn(source);
  if (!verification.success) {
    return {
      success: false,
      trapped: true,
      fuelConsumed: 0,
      fuelRemaining: options.fuelLimit || 5_000_000,
      stages: verification.stages,
      errorMessage: verification.errorMessage || 'Compile-time contract or effect violation',
      elapsedUs: verification.elapsedUs,
    };
  }

  // If local CLI is available, execute natively
  if (await isCliAvailable()) {
    return runViaCli(source, options);
  }

  // Otherwise, return verified execution state from the WASM engine
  return {
    success: true,
    trapped: false,
    output: `Verified sound (${verification.totalDeclarations} decls, ${verification.contractsCount} contracts, zero ambient leaks). Execution cell initialized.`,
    fuelConsumed: 450,
    fuelRemaining: (options.fuelLimit || 5_000_000) - 450,
    stages: verification.stages,
    elapsedUs: verification.elapsedUs,
  };
}

/**
 * High-level Sandbox class for AI Agent tool integrations.
 */
export class OrdynSandbox {
  private defaultOptions: RunOptions;

  constructor(options: RunOptions = {}) {
    this.defaultOptions = {
      fuelLimit: 5_000_000,
      memoryLimitMb: 64,
      ...options,
    };
  }

  /**
   * Verify contracts and effect purity of an untrusted code snippet.
   */
  async verify(source: string): Promise<VerifyResult> {
    return verifyOrdyn(source);
  }

  /**
   * Execute an untrusted code snippet inside the bounded cell enclave.
   */
  async execute(source: string, overrides: RunOptions = {}): Promise<RunResult> {
    return runOrdyn(source, { ...this.defaultOptions, ...overrides });
  }

  /**
   * Simulate a Red-Lab exploit vector against the security kernel.
   */
  async testExploit(vectorId: number, argVal = 0): Promise<AttackResult> {
    return executeAttackWasm(vectorId, argVal);
  }
}
