use miette::Diagnostic;
use ordyn_ast::*;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug, Clone, PartialEq)]
pub enum IrError {
    #[error("IR lowering error: {0}")]
    #[diagnostic(code(ordyn::ir::lowering_failed))]
    LoweringFailed(String),

    #[error("Malformed basic block: block {block_id} has no terminator")]
    #[diagnostic(code(ordyn::ir::unterminated_block))]
    UnterminatedBlock { block_id: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Reg(pub u32);

#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    Reg(Reg),
    ConstInt(i64),
    ConstFloat(f64),
    ConstBool(bool),
    ConstString(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrType {
    Int,
    Float,
    Bool,
    String,
    Unit,
    OpaqueHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrUnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Assign(Reg, Operand),
    Binary(Reg, IrBinaryOp, Operand, Operand),
    Unary(Reg, IrUnaryOp, Operand),
    Call(Option<Reg>, String, Vec<Operand>),
    CapabilityCheck {
        handle: Operand,
        operation: String,
        resource: Operand,
    },
    ContractAssert {
        condition: Operand,
        kind: String, // "Requires", "Ensures", "Invariant"
        message: String,
    },
    LoadField {
        dest: Reg,
        target: Operand,
        field: String,
    },
    StoreField {
        target: Operand,
        field: String,
        value: Operand,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Terminator {
    Return(Option<Operand>),
    Branch {
        condition: Operand,
        then_block: usize,
        else_block: usize,
    },
    Jump(usize),
    Trap(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicBlock {
    pub id: usize,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<(String, IrType)>,
    pub return_type: IrType,
    pub blocks: Vec<BasicBlock>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrEntity {
    pub name: String,
    pub fields: Vec<(String, IrType)>,
    pub methods: Vec<IrFunction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrBoundary {
    pub name: String,
    pub enclave: Option<String>,
    pub grants: Vec<String>,
    pub bindings: Vec<String>,
    pub fuel_limit: Option<u64>,
    pub memory_limit: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IrModule {
    pub functions: Vec<IrFunction>,
    pub entities: Vec<IrEntity>,
    pub boundaries: Vec<IrBoundary>,
}

pub struct IrLowerer {
    next_reg: u32,
    blocks: Vec<BasicBlock>,
    current_instructions: Vec<Instruction>,
    var_map: HashMap<String, Reg>,
}

impl Default for IrLowerer {
    fn default() -> Self {
        Self::new()
    }
}

impl IrLowerer {
    pub fn new() -> Self {
        Self {
            next_reg: 0,
            blocks: Vec::new(),
            current_instructions: Vec::new(),
            var_map: HashMap::new(),
        }
    }

    pub fn lower_program(&mut self, program: &Program) -> Result<IrModule, IrError> {
        let mut module = IrModule::default();

        for decl in &program.declarations {
            match decl {
                Declaration::Function(func) => {
                    let ir_func = self.lower_function(func, None)?;
                    module.functions.push(ir_func);
                }
                Declaration::Entity(entity) => {
                    let mut fields = Vec::new();
                    let mut methods = Vec::new();

                    for member in &entity.members {
                        match member {
                            EntityMember::Field(f) => {
                                fields.push((f.name.name.clone(), map_type_ref(&f.ty)));
                            }
                            EntityMember::Method(m) => {
                                let ir_method = self.lower_function(m, Some(&entity.name.name))?;
                                methods.push(ir_method);
                            }
                        }
                    }

                    module.entities.push(IrEntity {
                        name: entity.name.name.clone(),
                        fields,
                        methods,
                    });
                }
                Declaration::Boundary(boundary) => {
                    let grants = boundary
                        .grants
                        .iter()
                        .map(|g| g.capability_name.name.clone())
                        .collect();
                    let bindings = boundary.bindings.iter().map(|b| b.name.clone()).collect();
                    let (fuel_limit, memory_limit) = if let Some(ref q) = boundary.quota {
                        (q.fuel_limit, q.memory_limit)
                    } else {
                        (None, None)
                    };
                    let enclave = boundary.enclave.as_ref().map(|e| e.target.clone());
                    module.boundaries.push(IrBoundary {
                        name: boundary.name.name.clone(),
                        enclave,
                        grants,
                        bindings,
                        fuel_limit,
                        memory_limit,
                    });
                }
                _ => {}
            }
        }

        Ok(module)
    }

    fn lower_function(
        &mut self,
        func: &FunctionDecl,
        entity_name: Option<&str>,
    ) -> Result<IrFunction, IrError> {
        self.next_reg = 0;
        self.blocks.clear();
        self.current_instructions.clear();
        self.var_map.clear();

        let mut params = Vec::new();
        if let Some(_e_name) = entity_name {
            let self_reg = self.alloc_reg();
            self.var_map.insert("self".to_string(), self_reg);
            params.push(("self".to_string(), IrType::OpaqueHandle));
        }

        for p in &func.params {
            let reg = self.alloc_reg();
            self.var_map.insert(p.name.name.clone(), reg);
            params.push((p.name.name.clone(), map_type_ref(&p.ty)));
        }

        let return_type = if let Some(r) = &func.return_type {
            map_type_ref(r)
        } else {
            IrType::Unit
        };

        // Lower contracts (Preconditions / requires)
        for contract in &func.contracts {
            if let ContractClause::Requires(expr, _) = contract {
                let cond_op = self.lower_expr(expr)?;
                self.current_instructions.push(Instruction::ContractAssert {
                    condition: cond_op,
                    kind: "Requires".to_string(),
                    message: "Precondition failed".to_string(),
                });
            }
        }

        // Lower body statements
        for stmt in &func.body.statements {
            self.lower_stmt(stmt)?;
        }

        // Lower contracts (Postconditions / ensures)
        for contract in &func.contracts {
            if let ContractClause::Ensures(expr, _) = contract {
                let cond_op = self.lower_expr(expr)?;
                self.current_instructions.push(Instruction::ContractAssert {
                    condition: cond_op,
                    kind: "Ensures".to_string(),
                    message: "Postcondition failed".to_string(),
                });
            }
        }

        // Finalize default terminator if block not terminated
        let terminator = Terminator::Return(None);
        self.finish_block(terminator);

        Ok(IrFunction {
            name: if let Some(e_name) = entity_name {
                format!("{}::{}", e_name, func.name.name)
            } else {
                func.name.name.clone()
            },
            params,
            return_type,
            blocks: std::mem::take(&mut self.blocks),
        })
    }

    fn lower_stmt(&mut self, stmt: &Stmt) -> Result<(), IrError> {
        match stmt {
            Stmt::Let { name, init, .. } => {
                let reg = self.alloc_reg();
                if let Some(init_expr) = init {
                    let op = self.lower_expr(init_expr)?;
                    self.current_instructions.push(Instruction::Assign(reg, op));
                }
                self.var_map.insert(name.name.clone(), reg);
            }
            Stmt::Assign { target, value, .. } => {
                let val_op = self.lower_expr(value)?;
                match target {
                    Expr::Ident(id) => {
                        if let Some(&reg) = self.var_map.get(&id.name) {
                            self.current_instructions
                                .push(Instruction::Assign(reg, val_op));
                        }
                    }
                    Expr::FieldAccess {
                        target: inner_target,
                        field,
                        ..
                    } => {
                        let target_op = self.lower_expr(inner_target)?;
                        self.current_instructions.push(Instruction::StoreField {
                            target: target_op,
                            field: field.name.clone(),
                            value: val_op,
                        });
                    }
                    _ => {}
                }
            }
            Stmt::Return { value, .. } => {
                let ret_op = if let Some(expr) = value {
                    Some(self.lower_expr(expr)?)
                } else {
                    None
                };
                self.finish_block(Terminator::Return(ret_op));
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                let _cond_op = self.lower_expr(condition)?;
                for s in &then_branch.statements {
                    self.lower_stmt(s)?;
                }
                if let Some(eb) = else_branch {
                    for s in &eb.statements {
                        self.lower_stmt(s)?;
                    }
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                let _cond_op = self.lower_expr(condition)?;
                for s in &body.statements {
                    self.lower_stmt(s)?;
                }
            }
            Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::Expr(expr, _) => {
                self.lower_expr(expr)?;
            }
        }
        Ok(())
    }

    fn lower_expr(&mut self, expr: &Expr) -> Result<Operand, IrError> {
        match expr {
            Expr::Literal(lit, _) => Ok(match lit {
                Literal::Integer(i) => Operand::ConstInt(*i),
                Literal::Float(f) => Operand::ConstFloat(*f),
                Literal::Bool(b) => Operand::ConstBool(*b),
                Literal::String(s) => Operand::ConstString(s.clone()),
                Literal::None => Operand::ConstInt(0),
            }),
            Expr::Ident(id) => {
                if let Some(&reg) = self.var_map.get(&id.name) {
                    Ok(Operand::Reg(reg))
                } else {
                    Ok(Operand::ConstString(id.name.clone()))
                }
            }
            Expr::Binary {
                left, op, right, ..
            } => {
                let left_op = self.lower_expr(left)?;
                let right_op = self.lower_expr(right)?;
                let dest = self.alloc_reg();
                let ir_op = map_binary_op(*op);
                self.current_instructions
                    .push(Instruction::Binary(dest, ir_op, left_op, right_op));
                Ok(Operand::Reg(dest))
            }
            Expr::Unary { op, operand, .. } => {
                let op_val = self.lower_expr(operand)?;
                let dest = self.alloc_reg();
                let ir_op = map_unary_op(*op);
                self.current_instructions
                    .push(Instruction::Unary(dest, ir_op, op_val));
                Ok(Operand::Reg(dest))
            }
            Expr::Call { callee, args, .. } => {
                let mut arg_ops = Vec::new();
                for arg in args {
                    arg_ops.push(self.lower_expr(arg)?);
                }

                let callee_name = if let Expr::Ident(id) = &**callee {
                    id.name.clone()
                } else {
                    "indirect_call".to_string()
                };

                let dest = self.alloc_reg();
                self.current_instructions
                    .push(Instruction::Call(Some(dest), callee_name, arg_ops));
                Ok(Operand::Reg(dest))
            }
            Expr::FieldAccess { target, field, .. } => {
                let target_op = self.lower_expr(target)?;
                let dest = self.alloc_reg();
                self.current_instructions.push(Instruction::LoadField {
                    dest,
                    target: target_op,
                    field: field.name.clone(),
                });
                Ok(Operand::Reg(dest))
            }
            Expr::Old { expr, .. } => self.lower_expr(expr),
            Expr::Match { target, arms, .. } => {
                let _target_op = self.lower_expr(target)?;
                let dest = self.alloc_reg();
                for arm in arms {
                    let _arm_op = self.lower_expr(&arm.body)?;
                }
                Ok(Operand::Reg(dest))
            }
            Expr::Block(block) => {
                for stmt in &block.statements {
                    self.lower_stmt(stmt)?;
                }
                Ok(Operand::ConstInt(0))
            }
        }
    }

    fn alloc_reg(&mut self) -> Reg {
        let r = Reg(self.next_reg);
        self.next_reg += 1;
        r
    }

    fn finish_block(&mut self, terminator: Terminator) {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock {
            id,
            instructions: std::mem::take(&mut self.current_instructions),
            terminator,
        });
    }
}

fn map_type_ref(ty: &TypeRef) -> IrType {
    match ty {
        TypeRef::Primitive(prim, _) => match prim {
            PrimitiveType::Int => IrType::Int,
            PrimitiveType::Float => IrType::Float,
            PrimitiveType::Bool => IrType::Bool,
            PrimitiveType::String => IrType::String,
            PrimitiveType::Bytes => IrType::OpaqueHandle,
            PrimitiveType::Unit => IrType::Unit,
        },
        TypeRef::Named(_) => IrType::OpaqueHandle,
        TypeRef::Generic(..) => IrType::OpaqueHandle,
    }
}

fn map_binary_op(op: BinaryOp) -> IrBinaryOp {
    match op {
        BinaryOp::Add => IrBinaryOp::Add,
        BinaryOp::Sub => IrBinaryOp::Sub,
        BinaryOp::Mul => IrBinaryOp::Mul,
        BinaryOp::Div => IrBinaryOp::Div,
        BinaryOp::Mod => IrBinaryOp::Mod,
        BinaryOp::Eq => IrBinaryOp::Eq,
        BinaryOp::Neq => IrBinaryOp::Neq,
        BinaryOp::Lt => IrBinaryOp::Lt,
        BinaryOp::Lte => IrBinaryOp::Lte,
        BinaryOp::Gt => IrBinaryOp::Gt,
        BinaryOp::Gte => IrBinaryOp::Gte,
        BinaryOp::And => IrBinaryOp::And,
        BinaryOp::Or => IrBinaryOp::Or,
    }
}

fn map_unary_op(op: UnaryOp) -> IrUnaryOp {
    match op {
        UnaryOp::Neg => IrUnaryOp::Neg,
        UnaryOp::Not => IrUnaryOp::Not,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ordyn_parser::Parser;

    #[test]
    fn test_lower_function_with_contracts() {
        let source = r#"
            entity Vault {
                mut balance: Int;

                pub fn deposit(amount: Int) -> Int
                    requires amount > 0
                    ensures self.balance >= old(self.balance)
                {
                    self.balance = self.balance + amount;
                    return self.balance;
                }
            }
        "#;

        let mut parser = Parser::from_source(source).unwrap();
        let program = parser.parse_program().unwrap();

        let mut lowerer = IrLowerer::new();
        let module = lowerer
            .lower_program(&program)
            .expect("IR lowering should succeed");

        assert_eq!(module.entities.len(), 1);
        let entity = &module.entities[0];
        assert_eq!(entity.name, "Vault");
        assert_eq!(entity.methods.len(), 1);

        let method = &entity.methods[0];
        assert_eq!(method.name, "Vault::deposit");

        // Verify that blocks contain instructions and ContractAssert
        let has_contract_assert = method.blocks.iter().any(|b| {
            b.instructions
                .iter()
                .any(|inst| matches!(inst, Instruction::ContractAssert { .. }))
        });
        assert!(
            has_contract_assert,
            "IR must emit ContractAssert instructions for preconditions/postconditions"
        );
    }
}
