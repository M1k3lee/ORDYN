export interface PipelineStage {
  name: string;
  status: 'VERIFIED' | 'FAILED' | 'TRAPPED' | string;
  details: string;
}

export interface VerifyResult {
  success: boolean;
  totalDeclarations: number;
  entities: string[];
  capabilities: string[];
  boundaries: string[];
  contractsCount: number;
  tirFunctions: number;
  stages: PipelineStage[];
  errorMessage?: string;
  tirSummary?: string;
  elapsedUs: number;
}

export interface RunOptions {
  /** Maximum execution fuel budget (default: 5,000,000 units) */
  fuelLimit?: number;
  /** Maximum memory allocation in MB (default: 64) */
  memoryLimitMb?: number;
  /** Entrypoint function name to invoke (default: "main" or "deposit") */
  entryFunction?: string;
  /** Numeric integer argument passed to the entrypoint */
  argument?: number;
  /** Explicitly allowed capabilities (zero ambient authority by default) */
  allowedCapabilities?: string[];
}

export interface RunResult {
  success: boolean;
  trapped: boolean;
  output?: string;
  returnValue?: number | string;
  fuelConsumed: number;
  fuelRemaining: number;
  stages: PipelineStage[];
  errorMessage?: string;
  elapsedUs: number;
}

export interface AttackResult {
  success: boolean;
  trapped: boolean;
  vectorName: string;
  enforcementLayer: string;
  kernelVerdict: string;
  rustErrorType: string;
  diagnosticLines: string[];
  auditHash: string;
  bountyClaimed: boolean;
}
