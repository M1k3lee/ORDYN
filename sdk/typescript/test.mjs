import { verifyOrdyn, runOrdyn } from './dist/index.js';

const code = `
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
`;

async function main() {
    console.log("Running TypeScript SDK test...");
    const verifyRes = await verifyOrdyn(code);
    console.log("Verify Result:", verifyRes.success ? "SUCCESS" : "FAILED", `(${verifyRes.elapsedUs} us)`);
    console.log("Entities:", verifyRes.entities);
    console.log("Stages:", verifyRes.stages.map(s => `${s.name}: ${s.status}`).join(", "));

    const runRes = await runOrdyn(code, { fuelLimit: 50000 });
    console.log("Run Result:", runRes.success ? "SUCCESS" : "FAILED", `(${runRes.elapsedUs} us)`);
    console.log("Output:", runRes.output);

    // Now test a deliberately flawed contract to verify that SMT catches it!
    const badCode = `
    entity Vault {
        mut balance: Int;
        pub fn bad_withdraw(amount: Int) -> Result<Int, String> {
            // Missing require: could underflow!
            self.balance = self.balance - amount;
            return Ok(self.balance);
        }
    }
    `;
    const badVerify = await verifyOrdyn(badCode);
    console.log("Bad Code Verify (Should succeed parse/check):", badVerify.success ? "SUCCESS" : "FAILED");
}

main().catch(err => {
    console.error("Test Error:", err);
    process.exit(1);
});
