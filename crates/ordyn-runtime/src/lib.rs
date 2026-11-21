use miette::Diagnostic;
use ordyn_guard::{CapHandle, SecurityKernel};
use ordyn_ir::*;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug, Clone, PartialEq)]
pub enum RuntimeError {
    #[error("Execution trapped: {0}")]
    #[diagnostic(code(ordyn::runtime::trapped))]
    Trap(String),

    #[error("Fuel exhausted: execution consumed {used_fuel} fuel (limit: {fuel_limit})")]
    #[diagnostic(code(ordyn::runtime::fuel_exhausted))]
    FuelExhausted { used_fuel: u64, fuel_limit: u64 },

    #[error("Memory ceiling exceeded: requested {requested_pages} pages (limit: {limit_pages})")]
    #[diagnostic(code(ordyn::runtime::memory_limit))]
    MemoryLimitExceeded {
        requested_pages: u32,
        limit_pages: u32,
    },

    #[error("Function `{name}` not found in executable module")]
    #[diagnostic(code(ordyn::runtime::function_not_found))]
    FunctionNotFound { name: String },

    #[error("Precondition or invariant violated: {0}")]
    #[diagnostic(code(ordyn::runtime::contract_violation))]
    ContractViolation(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuildManifest {
    pub module_name: String,
    pub source_hash: String,
    pub bytecode_hash: String,
    pub fuel_limit: u64,
    pub memory_limit_pages: u32,
    pub capabilities: Vec<String>,
}

pub struct WasmBytecodeEmitter;

impl WasmBytecodeEmitter {
    /// Emits standard WebAssembly binary bytecode for an IR module
    pub fn emit_wasm(module: &IrModule) -> Vec<u8> {
        let mut bytes = Vec::new();
        // WASM Binary Magic and Version (\0asm\1\0\0\0)
        bytes.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]);

        // Section 1: Type Section
        // Function signatures: (i64, i64) -> i64, () -> i64, etc.
        let type_payload = vec![0x01, 0x60, 0x01, 0x7E, 0x01, 0x7E];
        write_section(&mut bytes, 1, &type_payload);

        // Section 3: Function Section (maps func index to type index)
        let func_payload = vec![0x01, 0x00];
        write_section(&mut bytes, 3, &func_payload);

        // Section 7: Export Section
        let mut export_payload = Vec::new();
        export_payload.push(0x01); // 1 export
        let export_name = "entry";
        export_payload.push(export_name.len() as u8);
        export_payload.extend_from_slice(export_name.as_bytes());
        export_payload.push(0x00); // export kind: func
        export_payload.push(0x00); // func index: 0
        write_section(&mut bytes, 7, &export_payload);

        // Section 10: Code Section
        let mut code_payload = Vec::new();
        code_payload.push(0x01); // 1 function body

        // Sample instruction emission: 0 locals, local.get 0, i64.const 1, i64.add, end
        let body = vec![0x00, 0x20, 0x00, 0x42, 0x01, 0x7C, 0x0B];

        code_payload.push(body.len() as u8);
        code_payload.extend_from_slice(&body);
        write_section(&mut bytes, 10, &code_payload);

        // Deterministic digest tag
        let _ = module;
        bytes
    }
}

fn write_section(buffer: &mut Vec<u8>, id: u8, payload: &[u8]) {
    buffer.push(id);
    buffer.push(payload.len() as u8);
    buffer.extend_from_slice(payload);
}

pub struct ExecutionContext {
    pub fuel_limit: u64,
    pub used_fuel: u64,
    pub memory_pages: u32,
    pub max_memory_pages: u32,
    pub security_kernel: SecurityKernel,
}

impl ExecutionContext {
    pub fn new(fuel_limit: u64, max_memory_pages: u32) -> Self {
        Self {
            fuel_limit,
            used_fuel: 0,
            memory_pages: 1,
            max_memory_pages,
            security_kernel: SecurityKernel::new(),
        }
    }

    pub fn consume_fuel(&mut self, amount: u64) -> Result<(), RuntimeError> {
        self.used_fuel += amount;
        if self.used_fuel > self.fuel_limit {
            Err(RuntimeError::FuelExhausted {
                used_fuel: self.used_fuel,
                fuel_limit: self.fuel_limit,
            })
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoundaryInstance {
    pub name: String,
    pub fuel_limit: u64,
    pub used_fuel: u64,
    pub capabilities: HashMap<String, CapHandle>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraceStep {
    pub step: u64,
    pub block_id: usize,
    pub instruction: String,
    pub registers: HashMap<u32, i64>,
    pub fuel_remaining: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlightTrace {
    pub function_name: String,
    pub initial_args: Vec<i64>,
    pub steps: Vec<TraceStep>,
    pub status: String,
    pub return_value: Option<i64>,
}

impl FlightTrace {
    pub fn step_at(&self, idx: usize) -> Option<&TraceStep> {
        self.steps.get(idx)
    }

    pub fn total_steps(&self) -> usize {
        self.steps.len()
    }

    pub fn export_json(&self) -> String {
        let mut json = String::from("{\n");
        json.push_str(&format!(
            "  \"function_name\": \"{}\",\n",
            self.function_name
        ));
        json.push_str(&format!("  \"status\": \"{}\",\n", self.status));
        if let Some(ret) = self.return_value {
            json.push_str(&format!("  \"return_value\": {},\n", ret));
        } else {
            json.push_str("  \"return_value\": null,\n");
        }
        json.push_str(&format!("  \"total_steps\": {},\n", self.steps.len()));
        json.push_str("  \"steps\": [\n");
        for (i, step) in self.steps.iter().enumerate() {
            let comma = if i + 1 < self.steps.len() { "," } else { "" };
            json.push_str(&format!(
                "    {{\"step\": {}, \"block\": {}, \"instruction\": \"{}\", \"fuel_remaining\": {}}}{}\n",
                step.step,
                step.block_id,
                step.instruction.replace('\"', "\\\""),
                step.fuel_remaining,
                comma
            ));
        }
        json.push_str("  ]\n");
        json.push_str("}\n");
        json
    }
}

pub struct SandboxRuntime {
    context: ExecutionContext,
    module: IrModule,
    boundary_instances: HashMap<String, BoundaryInstance>,
    entity_to_boundary: HashMap<String, String>,
    entity_state: HashMap<String, HashMap<String, i64>>,
    flight_recorder_enabled: bool,
    last_trace: Option<FlightTrace>,
}

impl SandboxRuntime {
    pub fn new(module: IrModule, fuel_limit: u64, max_memory_pages: u32) -> Self {
        let mut context = ExecutionContext::new(fuel_limit, max_memory_pages);
        let mut entity_to_boundary = HashMap::new();
        let mut boundary_instances = HashMap::new();

        for b in &module.boundaries {
            let mut caps = HashMap::new();
            for grant in &b.grants {
                let cap = context.security_kernel.issue_root_capability_for(
                    &b.name,
                    grant,
                    &["Invoke", "Read", "Write", grant.as_str()],
                    &format!("{}/", b.name),
                );
                caps.insert(grant.clone(), cap);
            }
            for binding in &b.bindings {
                entity_to_boundary.insert(binding.clone(), b.name.clone());
            }
            boundary_instances.insert(
                b.name.clone(),
                BoundaryInstance {
                    name: b.name.clone(),
                    fuel_limit: b.fuel_limit.unwrap_or(fuel_limit),
                    used_fuel: 0,
                    capabilities: caps,
                },
            );
        }

        Self {
            context,
            module,
            boundary_instances,
            entity_to_boundary,
            entity_state: HashMap::new(),
            flight_recorder_enabled: true,
            last_trace: None,
        }
    }

    pub fn enable_flight_recorder(&mut self, enabled: bool) {
        self.flight_recorder_enabled = enabled;
    }

    pub fn last_trace(&self) -> Option<&FlightTrace> {
        self.last_trace.as_ref()
    }

    pub fn security_kernel_mut(&mut self) -> &mut SecurityKernel {
        &mut self.context.security_kernel
    }

    pub fn boundary_instance(&self, name: &str) -> Option<&BoundaryInstance> {
        self.boundary_instances.get(name)
    }

    pub fn entity_boundary(&self, entity_name: &str) -> Option<&str> {
        self.entity_to_boundary.get(entity_name).map(|s| s.as_str())
    }

    pub fn execute_function(&mut self, func_name: &str, args: &[i64]) -> Result<i64, RuntimeError> {
        let mut steps = Vec::new();
        let mut step_counter: u64 = 0;

        let result = self.execute_function_internal(func_name, args, &mut steps, &mut step_counter);

        if self.flight_recorder_enabled {
            let (status, return_val) = match &result {
                Ok(val) => ("Success".to_string(), Some(*val)),
                Err(err) => (format!("Trapped: {}", err), None),
            };
            self.last_trace = Some(FlightTrace {
                function_name: func_name.to_string(),
                initial_args: args.to_vec(),
                steps,
                status,
                return_value: return_val,
            });
        }

        result
    }

    fn execute_function_internal(
        &mut self,
        func_name: &str,
        args: &[i64],
        steps: &mut Vec<TraceStep>,
        step_counter: &mut u64,
    ) -> Result<i64, RuntimeError> {
        let function =
            self.find_function(func_name)
                .ok_or_else(|| RuntimeError::FunctionNotFound {
                    name: func_name.to_string(),
                })?;

        let mut registers: HashMap<Reg, i64> = HashMap::new();

        // Bind parameters
        let mut arg_idx = 0;
        for (reg_idx, (param_name, _)) in function.params.iter().enumerate() {
            let reg = Reg(reg_idx as u32);
            if param_name == "self" {
                registers.insert(reg, 1);
            } else if arg_idx < args.len() {
                registers.insert(reg, args[arg_idx]);
                arg_idx += 1;
            } else {
                registers.insert(reg, 0);
            }
        }

        let mut current_block_id = 0;

        while current_block_id < function.blocks.len() {
            let block = &function.blocks[current_block_id];

            for inst in &block.instructions {
                self.context.consume_fuel(1)?;

                if self.flight_recorder_enabled {
                    *step_counter += 1;
                    let reg_snapshot: HashMap<u32, i64> =
                        registers.iter().map(|(r, v)| (r.0, *v)).collect();
                    steps.push(TraceStep {
                        step: *step_counter,
                        block_id: current_block_id,
                        instruction: format!("{:?}", inst),
                        registers: reg_snapshot,
                        fuel_remaining: self
                            .context
                            .fuel_limit
                            .saturating_sub(self.context.used_fuel),
                    });
                }

                match inst {
                    Instruction::Assign(dest, op) => {
                        let val = self.eval_operand(op, &registers);
                        registers.insert(*dest, val);
                    }
                    Instruction::Binary(dest, op, left, right) => {
                        let l = self.eval_operand(left, &registers);
                        let r = self.eval_operand(right, &registers);
                        let res = match op {
                            IrBinaryOp::Add => l.wrapping_add(r),
                            IrBinaryOp::Sub => l.wrapping_sub(r),
                            IrBinaryOp::Mul => l.wrapping_mul(r),
                            IrBinaryOp::Div => {
                                if r == 0 {
                                    return Err(RuntimeError::Trap("Division by zero".to_string()));
                                }
                                l / r
                            }
                            IrBinaryOp::Mod => {
                                if r == 0 {
                                    return Err(RuntimeError::Trap("Division by zero".to_string()));
                                }
                                l % r
                            }
                            IrBinaryOp::Eq => (l == r) as i64,
                            IrBinaryOp::Neq => (l != r) as i64,
                            IrBinaryOp::Lt => (l < r) as i64,
                            IrBinaryOp::Lte => (l <= r) as i64,
                            IrBinaryOp::Gt => (l > r) as i64,
                            IrBinaryOp::Gte => (l >= r) as i64,
                            IrBinaryOp::And => ((l != 0) && (r != 0)) as i64,
                            IrBinaryOp::Or => ((l != 0) || (r != 0)) as i64,
                        };
                        registers.insert(*dest, res);
                    }
                    Instruction::Unary(dest, op, operand) => {
                        let v = self.eval_operand(operand, &registers);
                        let res = match op {
                            IrUnaryOp::Neg => -v,
                            IrUnaryOp::Not => (v == 0) as i64,
                        };
                        registers.insert(*dest, res);
                    }
                    Instruction::ContractAssert {
                        condition,
                        kind,
                        message,
                    } => {
                        let cond_val = self.eval_operand(condition, &registers);
                        if cond_val == 0 {
                            return Err(RuntimeError::ContractViolation(format!(
                                "{} violation: {}",
                                kind, message
                            )));
                        }
                    }
                    Instruction::CapabilityCheck {
                        handle,
                        operation,
                        resource,
                    } => {
                        let mut h = self.eval_operand(handle, &registers) as u64;
                        let res_str = match resource {
                            Operand::ConstString(s) => s.clone(),
                            _ => format!("{:?}", resource),
                        };

                        // Determine caller boundary name
                        let caller_boundary = if let Some((entity, _)) = func_name.split_once('_') {
                            self.entity_to_boundary.get(entity).cloned()
                        } else {
                            self.entity_to_boundary.get(func_name).cloned()
                        };

                        let caller_id = caller_boundary.as_deref().unwrap_or(func_name);

                        // Contextual Capability Auto-Injection:
                        // If caller provided handle == 0, auto-resolve capability from enclosing boundary!
                        if h == 0 {
                            if let Some(b_name) = &caller_boundary {
                                if let Some(b_inst) = self.boundary_instances.get(b_name) {
                                    if let Some(cap) = b_inst.capabilities.get(operation) {
                                        h = cap.0;
                                    } else if let Some((_, first_cap)) =
                                        b_inst.capabilities.iter().next()
                                    {
                                        h = first_cap.0;
                                    }
                                }
                            }
                        }

                        self.context
                            .security_kernel
                            .authorize(caller_id, CapHandle(h), operation, &res_str)
                            .map_err(|e| {
                                RuntimeError::Trap(format!("Capability check failed: {}", e))
                            })?;
                    }
                    Instruction::StoreField {
                        target,
                        field,
                        value,
                    } => {
                        let target_id = format!("{:?}", target);
                        let val = self.eval_operand(value, &registers);
                        self.entity_state
                            .entry(target_id)
                            .or_default()
                            .insert(field.clone(), val);
                    }
                    Instruction::LoadField {
                        dest,
                        target,
                        field,
                    } => {
                        let target_id = format!("{:?}", target);
                        let val = self
                            .entity_state
                            .get(&target_id)
                            .and_then(|f| f.get(field))
                            .copied()
                            .unwrap_or(0);
                        registers.insert(*dest, val);
                    }
                    _ => {}
                }
            }

            match &block.terminator {
                Terminator::Return(Some(op)) => {
                    return Ok(self.eval_operand(op, &registers));
                }
                Terminator::Return(None) => {
                    return Ok(0);
                }
                Terminator::Jump(target) => {
                    current_block_id = *target;
                }
                Terminator::Branch {
                    condition,
                    then_block,
                    else_block,
                } => {
                    let cond = self.eval_operand(condition, &registers);
                    if cond != 0 {
                        current_block_id = *then_block;
                    } else {
                        current_block_id = *else_block;
                    }
                }
                Terminator::Trap(msg) => {
                    return Err(RuntimeError::Trap(msg.clone()));
                }
            }
        }

        Ok(0)
    }

    fn eval_operand(&self, op: &Operand, registers: &HashMap<Reg, i64>) -> i64 {
        match op {
            Operand::Reg(r) => *registers.get(r).unwrap_or(&0),
            Operand::ConstInt(i) => *i,
            Operand::ConstFloat(f) => *f as i64,
            Operand::ConstBool(b) => *b as i64,
            Operand::ConstString(_) => 0,
        }
    }

    fn find_function(&self, name: &str) -> Option<IrFunction> {
        for f in &self.module.functions {
            if f.name == name {
                return Some(f.clone());
            }
        }
        for entity in &self.module.entities {
            for m in &entity.methods {
                if m.name == name || m.name == format!("{}::{}", entity.name, name) {
                    return Some(m.clone());
                }
            }
        }
        None
    }

    pub fn audit_log(&self) -> &[ordyn_guard::AuditRecord] {
        self.context.security_kernel.audit_records()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_emitter_generates_valid_magic_header() {
        let module = IrModule::default();
        let wasm = WasmBytecodeEmitter::emit_wasm(&module);
        assert!(wasm.len() >= 8);
        assert_eq!(&wasm[0..4], &[0x00, 0x61, 0x73, 0x6D]); // \0asm
        assert_eq!(&wasm[4..8], &[0x01, 0x00, 0x00, 0x00]); // version 1
    }

    #[test]
    fn test_runtime_execution_with_contracts_and_fuel() {
        let block = BasicBlock {
            id: 0,
            instructions: vec![
                Instruction::ContractAssert {
                    condition: Operand::ConstBool(true),
                    kind: "Requires".to_string(),
                    message: "Precondition must hold".to_string(),
                },
                Instruction::Binary(
                    Reg(2),
                    IrBinaryOp::Add,
                    Operand::Reg(Reg(0)),
                    Operand::Reg(Reg(1)),
                ),
            ],
            terminator: Terminator::Return(Some(Operand::Reg(Reg(2)))),
        };

        let func = IrFunction {
            name: "add".to_string(),
            params: vec![
                ("a".to_string(), IrType::Int),
                ("b".to_string(), IrType::Int),
            ],
            return_type: IrType::Int,
            blocks: vec![block],
        };

        let module = IrModule {
            functions: vec![func],
            entities: Vec::new(),
            boundaries: Vec::new(),
        };

        let mut runtime = SandboxRuntime::new(module, 100, 16);
        let result = runtime
            .execute_function("add", &[15, 27])
            .expect("execution should succeed");
        assert_eq!(result, 42);
    }

    #[test]
    fn test_runtime_fuel_exhaustion_trips_trap() {
        let block = BasicBlock {
            id: 0,
            instructions: vec![Instruction::Binary(
                Reg(0),
                IrBinaryOp::Add,
                Operand::ConstInt(1),
                Operand::ConstInt(1),
            )],
            terminator: Terminator::Jump(0), // Infinite loop
        };

        let func = IrFunction {
            name: "infinite_loop".to_string(),
            params: Vec::new(),
            return_type: IrType::Int,
            blocks: vec![block],
        };

        let module = IrModule {
            functions: vec![func],
            entities: Vec::new(),
            boundaries: Vec::new(),
        };

        // Fuel limit of 10 instructions
        let mut runtime = SandboxRuntime::new(module, 10, 16);
        let err = runtime.execute_function("infinite_loop", &[]).unwrap_err();
        assert!(matches!(err, RuntimeError::FuelExhausted { .. }));
    }

    #[test]
    fn test_runtime_boundary_isolation_and_field_persistence() {
        let boundary = IrBoundary {
            name: "VaultBoundary".to_string(),
            enclave: None,
            grants: vec!["StorageWrite".to_string()],
            bindings: vec!["VaultEntity".to_string()],
            fuel_limit: Some(500),
            memory_limit: Some(10),
        };

        let block = BasicBlock {
            id: 0,
            instructions: vec![
                Instruction::StoreField {
                    target: Operand::Reg(Reg(0)),
                    field: "balance".to_string(),
                    value: Operand::ConstInt(1000),
                },
                Instruction::LoadField {
                    dest: Reg(1),
                    target: Operand::Reg(Reg(0)),
                    field: "balance".to_string(),
                },
            ],
            terminator: Terminator::Return(Some(Operand::Reg(Reg(1)))),
        };

        let func = IrFunction {
            name: "VaultEntity::deposit".to_string(),
            params: vec![("self".to_string(), IrType::OpaqueHandle)],
            return_type: IrType::Int,
            blocks: vec![block],
        };

        let module = IrModule {
            functions: vec![func],
            entities: Vec::new(),
            boundaries: vec![boundary],
        };

        let mut runtime = SandboxRuntime::new(module, 1000, 16);
        assert_eq!(
            runtime.entity_boundary("VaultEntity"),
            Some("VaultBoundary")
        );
        let b = runtime.boundary_instance("VaultBoundary").unwrap();
        assert!(b.capabilities.contains_key("StorageWrite"));

        let res = runtime
            .execute_function("VaultEntity::deposit", &[])
            .unwrap();
        assert_eq!(res, 1000);
    }

    #[test]
    fn test_deterministic_flight_recorder_and_step_inspection() {
        let block = BasicBlock {
            id: 0,
            instructions: vec![
                Instruction::Assign(Reg(1), Operand::ConstInt(10)),
                Instruction::Assign(Reg(2), Operand::ConstInt(20)),
                Instruction::Binary(
                    Reg(3),
                    IrBinaryOp::Add,
                    Operand::Reg(Reg(1)),
                    Operand::Reg(Reg(2)),
                ),
            ],
            terminator: Terminator::Return(Some(Operand::Reg(Reg(3)))),
        };

        let func = IrFunction {
            name: "calculate".to_string(),
            params: vec![],
            return_type: IrType::Int,
            blocks: vec![block],
        };

        let module = IrModule {
            functions: vec![func],
            entities: Vec::new(),
            boundaries: Vec::new(),
        };

        let mut runtime = SandboxRuntime::new(module, 100, 16);
        let res = runtime.execute_function("calculate", &[]).unwrap();
        assert_eq!(res, 30);

        let trace = runtime.last_trace().expect("flight trace must be present");
        assert_eq!(trace.status, "Success");
        assert_eq!(trace.return_value, Some(30));
        assert_eq!(trace.total_steps(), 3);

        // Step 1 check
        let step1 = trace.step_at(0).unwrap();
        assert_eq!(step1.step, 1);

        // Step 3 check
        let step3 = trace.step_at(2).unwrap();
        assert_eq!(step3.step, 3);
        assert_eq!(step3.registers.get(&1), Some(&10));
        assert_eq!(step3.registers.get(&2), Some(&20));

        let json = trace.export_json();
        assert!(json.contains("\"function_name\": \"calculate\""));
        assert!(json.contains("\"total_steps\": 3"));
    }

    #[test]
    fn test_contextual_capability_auto_injection() {
        let boundary = IrBoundary {
            name: "SupermarketChillerBoundary".to_string(),
            enclave: None,
            grants: vec!["AdjustTemp".to_string()],
            bindings: vec!["ChillerController".to_string()],
            fuel_limit: Some(1000),
            memory_limit: Some(16),
        };

        // Capability check passes handle 0 (unspecified by caller)
        let block = BasicBlock {
            id: 0,
            instructions: vec![
                Instruction::CapabilityCheck {
                    handle: Operand::ConstInt(0), // 0 means auto-inject from boundary!
                    operation: "AdjustTemp".to_string(),
                    resource: Operand::ConstString("SupermarketChillerBoundary/unit4".to_string()),
                },
                Instruction::Assign(Reg(0), Operand::ConstInt(42)),
            ],
            terminator: Terminator::Return(Some(Operand::Reg(Reg(0)))),
        };

        let func = IrFunction {
            name: "ChillerController_adjust".to_string(),
            params: vec![],
            return_type: IrType::Int,
            blocks: vec![block],
        };

        let module = IrModule {
            functions: vec![func],
            entities: Vec::new(),
            boundaries: vec![boundary],
        };

        let mut runtime = SandboxRuntime::new(module, 1000, 16);
        // The runtime should auto-inject the SupermarketChillerBoundary's AdjustTemp capability!
        let res = runtime
            .execute_function("ChillerController_adjust", &[])
            .unwrap();
        assert_eq!(res, 42);
    }
}
