# @ordyn/sandbox

Zero-ambient-authority execution and formal SMT verification sandbox for **AI agents, LLM tool-calling, and untrusted code**.

Powered by the **ORDYN Sovereign WebAssembly Core**. Runs in Node.js, Deno, Bun, and browser environments with zero external dependencies.

---

## ⚡ Quickstart in 10 Lines

```typescript
import { runOrdyn } from '@ordyn/sandbox';

const result = await runOrdyn(`
    entity Vault {
        mut balance: Int;
        pub fn withdraw(amount: Int) -> Result<Int, String>
            requires amount > 0 && amount <= self.balance
            ensures  self.balance == old(self.balance) - amount
        {
            self.balance = self.balance - amount;
            return Ok(self.balance);
        }
    }
`, { fuelLimit: 50_000 });

if (result.success) {
  console.log(`[PASS in ${result.elapsedUs}µs] Contracts verified sound, zero ambient leaks!`);
} else {
  console.error(`[TRAPPED] ${result.errorMessage}`);
}
```

---

## 🤖 Why AI Agents Need ORDYN

When building autonomous agents with **LangChain, Cursor, Claude Code, or AutoGen**, letting the LLM execute arbitrary Python or Bash code is risky:
- The model can hallucinate infinite loops (`while True: ...`), crashing host workers.
- Prompt injection attacks can trick the LLM into exfiltrating API keys or reading files.
- Concurrency bugs and arithmetic underflows bypass simple runtime unit tests.

### How ORDYN Protects Agent Workflows:
1. **Zero Ambient Authority**: Third-party libraries and agent code cannot touch sockets, disk, or environment keys unless an explicit unforgeable `capability` is passed down.
2. **Presburger SMT Prover**: If the LLM writes invalid logic (e.g., balance arithmetic allowing negative numbers), the SMT solver rejects it at compile time and outputs the exact counterexample.
3. **Deterministic Fuel Governors**: Execution halts safely when the instruction fuel quota is exhausted.

---

## 🛠️ AI Tool Integration (LangChain / Vercel AI SDK Example)

```typescript
import { tool } from 'ai'; // Vercel AI SDK
import { z } from 'zod';
import { OrdynSandbox } from '@ordyn/sandbox';

const sandbox = new OrdynSandbox({ fuelLimit: 100_000 });

export const ordynExecuteTool = tool({
  description: 'Execute untrusted code in a mathematically proven, zero-ambient-authority sandbox.',
  parameters: z.object({
    sourceCode: z.string().describe('ORDYN code with entity contracts and boundaries'),
  }),
  execute: async ({ sourceCode }) => {
    const res = await sandbox.execute(sourceCode);
    if (!res.success) {
      // The LLM receives the exact SMT counterexample or effect error to self-correct:
      return { status: 'TRAPPED', error: res.errorMessage };
    }
    return { status: 'VERIFIED_SOUND', output: res.output, fuelConsumed: res.fuelConsumed };
  },
});
```

---

## 📦 Installation

```bash
npm install @ordyn/sandbox
# or
pnpm add @ordyn/sandbox
# or
yarn add @ordyn/sandbox
```

---

## 📄 License

Apache-2.0 OR MIT
