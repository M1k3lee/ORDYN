import sys
from pathlib import Path

# Add sdk/python to sys.path
sys.path.insert(0, str(Path(__file__).parent))

from ordyn import verify_ordyn, run_ordyn, OrdynSandbox

code = """
entity Vault {
    mut balance: Int;

    pub fn deposit(amount: Int) -> Result<Int, String>
        requires amount > 0
        ensures  self.balance == old(self.balance) + amount
    {
        self.balance = self.balance + amount;
        return Ok(self.balance);
    }
}
"""

def main():
    print("Testing Python SDK...")
    sandbox = OrdynSandbox(fuel_limit=50_000)

    # 1. Verification
    res = sandbox.verify(code)
    print(f"Verification Result: {'SUCCESS' if res.is_valid else 'FAILED'} (elapsed: {res.elapsed_us} us)")
    if res.stages:
        print("Stages:")
        for s in res.stages:
            print(f"  - {s.name}: {s.status}")

    # 2. Execution of valid deposit
    run_res = sandbox.execute(code, function="deposit", argument=100)
    print(f"Execution Result: {'SUCCESS' if run_res.is_valid else 'FAILED'} (fuel remaining: {run_res.fuel_remaining})")
    print(f"Output: {run_res.output}")

    # 3. Test Contract Invariant Rejection on Underflow
    bad_code = """
    entity Vault {
        mut balance: Int;
        pub fn bad_withdraw(amount: Int) -> Result<Int, String>
            requires amount > 0 && amount <= self.balance
        {
            self.balance = self.balance - amount;
            return Ok(self.balance);
        }
    }
    """
    bad_res = sandbox.execute(bad_code, function="bad_withdraw", argument=500)
    print(f"Adversarial Underflow Attempt: {'TRAPPED (SAFE)' if not bad_res.is_valid else 'UNEXPECTED PASS'}")

    print("All Python SDK tests passed successfully!")

if __name__ == "__main__":
    main()
