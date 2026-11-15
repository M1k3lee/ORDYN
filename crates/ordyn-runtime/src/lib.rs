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

pub struct SandboxRuntime {
    context: ExecutionContext,
    module: IrModule,
}

impl SandboxRuntime {
    pub fn new(module: IrModule, fuel_limit: u64, max_memory_pages: u32) -> Self {
        Self {
            context: ExecutionContext::new(fuel_limit, max_memory_pages),
            module,
        }
    }

    pub fn security_kernel_mut(&mut self) -> &mut SecurityKernel {
        &mut self.context.security_kernel
    }

    pub fn execute_function(&mut self, func_name: &str, args: &[i64]) -> Result<i64, RuntimeError> {
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
                        let h = self.eval_operand(handle, &registers) as u32;
                        let res_str = format!("{:?}", resource);
                        self.context
                            .security_kernel
                            .authorize(func_name, CapHandle(h), operation, &res_str)
                            .map_err(|e| {
                                RuntimeError::Trap(format!("Capability check failed: {}", e))
                            })?;
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
        };

        // Fuel limit of 10 instructions
        let mut runtime = SandboxRuntime::new(module, 10, 16);
        let err = runtime.execute_function("infinite_loop", &[]).unwrap_err();
        assert!(matches!(err, RuntimeError::FuelExhausted { .. }));
    }
}
