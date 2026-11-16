use miette::Diagnostic;
use ordyn_ast::*;
use ordyn_diagnostics::Span;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug, Clone, PartialEq)]
pub enum TypeError {
    #[error("Type mismatch: expected `{expected}`, found `{found}`")]
    #[diagnostic(code(ordyn::types::mismatch))]
    TypeMismatch {
        expected: String,
        found: String,
        #[label("expected `{expected}` here")]
        span: Span,
    },

    #[error("Undefined symbol: `{name}`")]
    #[diagnostic(code(ordyn::types::undefined_symbol))]
    UndefinedSymbol {
        name: String,
        #[label("`{name}` is not defined in this scope")]
        span: Span,
    },

    #[error("Cannot assign to immutable variable `{name}`")]
    #[diagnostic(code(ordyn::types::immutable_assignment))]
    ImmutableAssignment {
        name: String,
        #[label("variable declared immutable; declare with `let mut` or `mut`")]
        span: Span,
    },

    #[error("Invalid binary operation `{op}` for types `{left}` and `{right}`")]
    #[diagnostic(code(ordyn::types::invalid_binary_op))]
    InvalidBinaryOp {
        op: String,
        left: String,
        right: String,
        #[label("cannot apply `{op}` to `{left}` and `{right}`")]
        span: Span,
    },

    #[error("Invalid unary operation `{op}` for type `{operand}`")]
    #[diagnostic(code(ordyn::types::invalid_unary_op))]
    InvalidUnaryOp {
        op: String,
        operand: String,
        #[label("cannot apply `{op}` to `{operand}`")]
        span: Span,
    },

    #[error("Type `{ty}` is not callable")]
    #[diagnostic(code(ordyn::types::not_callable))]
    NotCallable {
        ty: String,
        #[label("cannot call value of type `{ty}`")]
        span: Span,
    },

    #[error("Argument count mismatch: expected {expected}, found {found}")]
    #[diagnostic(code(ordyn::types::argument_count))]
    ArgumentCountMismatch {
        expected: usize,
        found: usize,
        #[label("expected {expected} arguments")]
        span: Span,
    },

    #[error("Unknown field `{field}` on type `{target_type}`")]
    #[diagnostic(code(ordyn::types::unknown_field))]
    UnknownField {
        target_type: String,
        field: String,
        #[label("field `{field}` not found on `{target_type}`")]
        span: Span,
    },

    #[error("Contract condition must evaluate to `Bool`, found `{found}`")]
    #[diagnostic(code(ordyn::types::contract_not_bool))]
    ContractMustBeBool {
        found: String,
        #[label("expected boolean invariant or condition")]
        span: Span,
    },

    #[error("Return type mismatch: expected `{expected}`, found `{found}`")]
    #[diagnostic(code(ordyn::types::return_mismatch))]
    ReturnMismatch {
        expected: String,
        found: String,
        #[label("expected return type `{expected}`")]
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Bytes,
    Unit,
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Entity(String),
    Capability(String),
    Interface(String),
    Named(String),
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
        effects: Vec<String>,
    },
    SelfType(String),
    Unknown,
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Bool => write!(f, "Bool"),
            Type::String => write!(f, "String"),
            Type::Bytes => write!(f, "Bytes"),
            Type::Unit => write!(f, "Unit"),
            Type::Option(inner) => write!(f, "Option<{}>", inner),
            Type::Result(ok, err) => write!(f, "Result<{}, {}>", ok, err),
            Type::Entity(name) => write!(f, "Entity({})", name),
            Type::Capability(name) => write!(f, "Capability({})", name),
            Type::Interface(name) => write!(f, "Interface({})", name),
            Type::Named(name) => write!(f, "{}", name),
            Type::Function {
                params,
                return_type,
                ..
            } => {
                let p_strs: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "fn({}) -> {}", p_strs.join(", "), return_type)
            }
            Type::SelfType(name) => write!(f, "self({})", name),
            Type::Unknown => write!(f, "<unknown>"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VarInfo {
    pub ty: Type,
    pub is_mut: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MethodSig {
    pub params: Vec<(String, Type, bool)>,
    pub return_type: Type,
    pub effects: Vec<String>,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct EntityInfo {
    pub name: String,
    pub fields: HashMap<String, (Type, bool)>, // (Type, is_mut)
    pub methods: HashMap<String, MethodSig>,
}

#[derive(Debug, Clone)]
pub struct CapabilityInfo {
    pub name: String,
    pub fields: HashMap<String, Type>,
}

#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    pub name: String,
    pub methods: HashMap<String, MethodSig>,
}

pub struct TypeChecker {
    entities: HashMap<String, EntityInfo>,
    capabilities: HashMap<String, CapabilityInfo>,
    interfaces: HashMap<String, InterfaceInfo>,
    functions: HashMap<String, MethodSig>,
    scopes: Vec<HashMap<String, VarInfo>>,
    current_return_type: Option<Type>,
    current_entity: Option<String>,
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            capabilities: HashMap::new(),
            interfaces: HashMap::new(),
            functions: HashMap::new(),
            scopes: vec![HashMap::new()],
            current_return_type: None,
            current_entity: None,
        }
    }

    pub fn check_program(&mut self, program: &Program) -> Result<(), TypeError> {
        // Pass 1: Register top-level nominal types and signatures
        for decl in &program.declarations {
            self.register_declaration_signature(decl)?;
        }

        // Pass 2: Typecheck bodies, methods, contracts, and boundaries
        for decl in &program.declarations {
            self.check_declaration_body(decl)?;
        }

        Ok(())
    }

    fn register_declaration_signature(&mut self, decl: &Declaration) -> Result<(), TypeError> {
        match decl {
            Declaration::Capability(cap) => {
                let mut fields = HashMap::new();
                for f in &cap.fields {
                    let ty = self.resolve_type_ref(&f.ty)?;
                    fields.insert(f.name.name.clone(), ty);
                }
                self.capabilities.insert(
                    cap.name.name.clone(),
                    CapabilityInfo {
                        name: cap.name.name.clone(),
                        fields,
                    },
                );
            }
            Declaration::Interface(iface) => {
                let mut methods = HashMap::new();
                for m in &iface.methods {
                    let mut params = Vec::new();
                    for p in &m.params {
                        let ty = self.resolve_type_ref(&p.ty)?;
                        params.push((p.name.name.clone(), ty, p.is_mut));
                    }
                    let return_type = if let Some(r) = &m.return_type {
                        self.resolve_type_ref(r)?
                    } else {
                        Type::Unit
                    };
                    let effects = m.effects.iter().map(|e| e.name.clone()).collect();
                    methods.insert(
                        m.name.name.clone(),
                        MethodSig {
                            params,
                            return_type,
                            effects,
                            is_pub: true,
                        },
                    );
                }
                self.interfaces.insert(
                    iface.name.name.clone(),
                    InterfaceInfo {
                        name: iface.name.name.clone(),
                        methods,
                    },
                );
            }
            Declaration::Entity(entity) => {
                let mut fields = HashMap::new();
                let mut methods = HashMap::new();

                for member in &entity.members {
                    match member {
                        EntityMember::Field(f) => {
                            let ty = self.resolve_type_ref(&f.ty)?;
                            fields.insert(f.name.name.clone(), (ty, f.is_mut));
                        }
                        EntityMember::Method(m) => {
                            let mut params = Vec::new();
                            for p in &m.params {
                                let ty = self.resolve_type_ref(&p.ty)?;
                                params.push((p.name.name.clone(), ty, p.is_mut));
                            }
                            let return_type = if let Some(r) = &m.return_type {
                                self.resolve_type_ref(r)?
                            } else {
                                Type::Unit
                            };
                            let effects = m.effects.iter().map(|e| e.name.clone()).collect();
                            methods.insert(
                                m.name.name.clone(),
                                MethodSig {
                                    params,
                                    return_type,
                                    effects,
                                    is_pub: m.is_pub,
                                },
                            );
                        }
                    }
                }

                self.entities.insert(
                    entity.name.name.clone(),
                    EntityInfo {
                        name: entity.name.name.clone(),
                        fields,
                        methods,
                    },
                );
            }
            Declaration::Function(func) => {
                let mut params = Vec::new();
                for p in &func.params {
                    let ty = self.resolve_type_ref(&p.ty)?;
                    params.push((p.name.name.clone(), ty, p.is_mut));
                }
                let return_type = if let Some(r) = &func.return_type {
                    self.resolve_type_ref(r)?
                } else {
                    Type::Unit
                };
                let effects = func.effects.iter().map(|e| e.name.clone()).collect();
                self.functions.insert(
                    func.name.name.clone(),
                    MethodSig {
                        params,
                        return_type,
                        effects,
                        is_pub: func.is_pub,
                    },
                );
            }
            _ => {}
        }
        Ok(())
    }

    fn check_declaration_body(&mut self, decl: &Declaration) -> Result<(), TypeError> {
        match decl {
            Declaration::Entity(entity) => {
                self.current_entity = Some(entity.name.name.clone());
                for member in &entity.members {
                    if let EntityMember::Method(m) = member {
                        self.check_function_decl(m)?;
                    }
                }
                self.current_entity = None;
            }
            Declaration::Function(func) => {
                self.check_function_decl(func)?;
            }
            Declaration::Contract(contract) => {
                self.enter_scope();
                for clause in &contract.clauses {
                    match clause {
                        ContractClause::Requires(expr, _)
                        | ContractClause::Ensures(expr, _)
                        | ContractClause::Invariant(expr, _) => {
                            let ty = self.check_expr(expr)?;
                            if ty != Type::Bool {
                                return Err(TypeError::ContractMustBeBool {
                                    found: ty.to_string(),
                                    span: expr.span(),
                                });
                            }
                        }
                    }
                }
                self.exit_scope();
            }
            Declaration::Boundary(boundary) => {
                // Verify granted capabilities exist and args match
                for grant in &boundary.grants {
                    let cap_info = self
                        .capabilities
                        .get(&grant.capability_name.name)
                        .cloned()
                        .ok_or_else(|| TypeError::UndefinedSymbol {
                            name: grant.capability_name.name.clone(),
                            span: grant.capability_name.span,
                        })?;

                    for (arg_name, arg_expr) in &grant.arguments {
                        let field_ty =
                            cap_info
                                .fields
                                .get(&arg_name.name)
                                .cloned()
                                .ok_or_else(|| TypeError::UnknownField {
                                    target_type: cap_info.name.clone(),
                                    field: arg_name.name.clone(),
                                    span: arg_name.span,
                                })?;
                        let expr_ty = self.check_expr(arg_expr)?;
                        if expr_ty != field_ty {
                            return Err(TypeError::TypeMismatch {
                                expected: field_ty.to_string(),
                                found: expr_ty.to_string(),
                                span: arg_expr.span(),
                            });
                        }
                    }
                }

                // Verify bound entities exist
                for binding in &boundary.bindings {
                    if !self.entities.contains_key(&binding.name) {
                        return Err(TypeError::UndefinedSymbol {
                            name: binding.name.clone(),
                            span: binding.span,
                        });
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn check_function_decl(&mut self, func: &FunctionDecl) -> Result<(), TypeError> {
        self.enter_scope();

        let return_type = if let Some(r) = &func.return_type {
            self.resolve_type_ref(r)?
        } else {
            Type::Unit
        };
        self.current_return_type = Some(return_type.clone());

        // Define `self` if inside an entity method
        if let Some(entity_name) = &self.current_entity {
            self.define_var(
                "self",
                Type::SelfType(entity_name.clone()),
                true,
                func.name.span,
            );
        }

        // Define parameters
        for p in &func.params {
            let ty = self.resolve_type_ref(&p.ty)?;
            self.define_var(&p.name.name, ty, p.is_mut, p.name.span);
        }

        // Check contracts (must be bool)
        for contract in &func.contracts {
            let (expr, _span) = match contract {
                ContractClause::Requires(e, s) => (e, s),
                ContractClause::Ensures(e, s) => (e, s),
                ContractClause::Invariant(e, s) => (e, s),
            };
            let c_ty = self.check_expr(expr)?;
            if c_ty != Type::Bool {
                return Err(TypeError::ContractMustBeBool {
                    found: c_ty.to_string(),
                    span: expr.span(),
                });
            }
        }

        // Check block statements
        for stmt in &func.body.statements {
            self.check_stmt(stmt)?;
        }

        self.exit_scope();
        self.current_return_type = None;
        Ok(())
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), TypeError> {
        match stmt {
            Stmt::Let {
                is_mut,
                name,
                ty,
                init,
                span,
            } => {
                let inferred_ty = if let Some(init_expr) = init {
                    let expr_ty = self.check_expr(init_expr)?;
                    if let Some(annotated) = ty {
                        let expected = self.resolve_type_ref(annotated)?;
                        if expr_ty != expected {
                            return Err(TypeError::TypeMismatch {
                                expected: expected.to_string(),
                                found: expr_ty.to_string(),
                                span: init_expr.span(),
                            });
                        }
                    }
                    expr_ty
                } else if let Some(annotated) = ty {
                    self.resolve_type_ref(annotated)?
                } else {
                    Type::Unknown
                };

                self.define_var(&name.name, inferred_ty, *is_mut, *span);
            }
            Stmt::Assign {
                target,
                value,
                span,
            } => {
                let target_ty = self.check_expr(target)?;
                let value_ty = self.check_expr(value)?;

                if target_ty != value_ty {
                    return Err(TypeError::TypeMismatch {
                        expected: target_ty.to_string(),
                        found: value_ty.to_string(),
                        span: value.span(),
                    });
                }

                // Verify mutability
                if let Expr::Ident(id) = target {
                    if let Some(var) = self.lookup_var(&id.name) {
                        if !var.is_mut {
                            return Err(TypeError::ImmutableAssignment {
                                name: id.name.clone(),
                                span: *span,
                            });
                        }
                    }
                } else if let Expr::FieldAccess {
                    target: inner_target,
                    field,
                    ..
                } = target
                {
                    if let Expr::Ident(id) = &**inner_target {
                        if id.name == "self" {
                            if let Some(entity_name) = &self.current_entity {
                                if let Some(entity_info) = self.entities.get(entity_name) {
                                    if let Some((_, is_mut)) = entity_info.fields.get(&field.name) {
                                        if !is_mut {
                                            return Err(TypeError::ImmutableAssignment {
                                                name: format!("self.{}", field.name),
                                                span: *span,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Stmt::Return { value, span } => {
                let expected = self.current_return_type.clone().unwrap_or(Type::Unit);
                let actual = if let Some(ret_expr) = value {
                    self.check_expr(ret_expr)?
                } else {
                    Type::Unit
                };

                // Allow Ok(...) / Err(...) matching Result<T, E>
                if !types_compatible(&expected, &actual) {
                    return Err(TypeError::ReturnMismatch {
                        expected: expected.to_string(),
                        found: actual.to_string(),
                        span: *span,
                    });
                }
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                span: _,
            } => {
                let cond_ty = self.check_expr(condition)?;
                if cond_ty != Type::Bool {
                    return Err(TypeError::TypeMismatch {
                        expected: "Bool".to_string(),
                        found: cond_ty.to_string(),
                        span: condition.span(),
                    });
                }
                self.enter_scope();
                for s in &then_branch.statements {
                    self.check_stmt(s)?;
                }
                self.exit_scope();

                if let Some(eb) = else_branch {
                    self.enter_scope();
                    for s in &eb.statements {
                        self.check_stmt(s)?;
                    }
                    self.exit_scope();
                }
            }
            Stmt::Expr(expr, _) => {
                self.check_expr(expr)?;
            }
        }
        Ok(())
    }

    fn check_expr(&mut self, expr: &Expr) -> Result<Type, TypeError> {
        match expr {
            Expr::Literal(lit, _) => Ok(match lit {
                Literal::Integer(_) => Type::Int,
                Literal::Float(_) => Type::Float,
                Literal::String(_) => Type::String,
                Literal::Bool(_) => Type::Bool,
                Literal::None => Type::Option(Box::new(Type::Unknown)),
            }),
            Expr::Ident(id) => {
                if let Some(var) = self.lookup_var(&id.name) {
                    Ok(var.ty.clone())
                } else if self.entities.contains_key(&id.name) {
                    Ok(Type::Entity(id.name.clone()))
                } else if self.capabilities.contains_key(&id.name) {
                    Ok(Type::Capability(id.name.clone()))
                } else if id.name == "Ok" || id.name == "Some" {
                    // Constructor helper
                    Ok(Type::Unknown)
                } else {
                    Err(TypeError::UndefinedSymbol {
                        name: id.name.clone(),
                        span: id.span,
                    })
                }
            }
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => {
                let left_ty = self.check_expr(left)?;
                let right_ty = self.check_expr(right)?;

                match op {
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Mod => {
                        if left_ty == Type::Int && right_ty == Type::Int {
                            Ok(Type::Int)
                        } else if left_ty == Type::Float && right_ty == Type::Float {
                            Ok(Type::Float)
                        } else if *op == BinaryOp::Add
                            && left_ty == Type::String
                            && right_ty == Type::String
                        {
                            Ok(Type::String)
                        } else {
                            Err(TypeError::InvalidBinaryOp {
                                op: format!("{:?}", op),
                                left: left_ty.to_string(),
                                right: right_ty.to_string(),
                                span: *span,
                            })
                        }
                    }
                    BinaryOp::Eq | BinaryOp::Neq => {
                        if left_ty == right_ty {
                            Ok(Type::Bool)
                        } else {
                            Err(TypeError::InvalidBinaryOp {
                                op: format!("{:?}", op),
                                left: left_ty.to_string(),
                                right: right_ty.to_string(),
                                span: *span,
                            })
                        }
                    }
                    BinaryOp::Lt | BinaryOp::Lte | BinaryOp::Gt | BinaryOp::Gte => {
                        if (left_ty == Type::Int && right_ty == Type::Int)
                            || (left_ty == Type::Float && right_ty == Type::Float)
                        {
                            Ok(Type::Bool)
                        } else {
                            Err(TypeError::InvalidBinaryOp {
                                op: format!("{:?}", op),
                                left: left_ty.to_string(),
                                right: right_ty.to_string(),
                                span: *span,
                            })
                        }
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        if left_ty == Type::Bool && right_ty == Type::Bool {
                            Ok(Type::Bool)
                        } else {
                            Err(TypeError::InvalidBinaryOp {
                                op: format!("{:?}", op),
                                left: left_ty.to_string(),
                                right: right_ty.to_string(),
                                span: *span,
                            })
                        }
                    }
                }
            }
            Expr::Unary { op, operand, span } => {
                let operand_ty = self.check_expr(operand)?;
                match op {
                    UnaryOp::Not => {
                        if operand_ty == Type::Bool {
                            Ok(Type::Bool)
                        } else {
                            Err(TypeError::InvalidUnaryOp {
                                op: "!".to_string(),
                                operand: operand_ty.to_string(),
                                span: *span,
                            })
                        }
                    }
                    UnaryOp::Neg => {
                        if operand_ty == Type::Int {
                            Ok(Type::Int)
                        } else if operand_ty == Type::Float {
                            Ok(Type::Float)
                        } else {
                            Err(TypeError::InvalidUnaryOp {
                                op: "-".to_string(),
                                operand: operand_ty.to_string(),
                                span: *span,
                            })
                        }
                    }
                }
            }
            Expr::Call { callee, args, span } => {
                if let Expr::Ident(id) = &**callee {
                    if id.name == "Ok" {
                        let inner_ty = if let Some(arg) = args.first() {
                            self.check_expr(arg)?
                        } else {
                            Type::Unit
                        };
                        return Ok(Type::Result(Box::new(inner_ty), Box::new(Type::Unknown)));
                    } else if id.name == "Err" {
                        let inner_ty = if let Some(arg) = args.first() {
                            self.check_expr(arg)?
                        } else {
                            Type::Unit
                        };
                        return Ok(Type::Result(Box::new(Type::Unknown), Box::new(inner_ty)));
                    } else if id.name == "Some" {
                        let inner_ty = if let Some(arg) = args.first() {
                            self.check_expr(arg)?
                        } else {
                            Type::Unit
                        };
                        return Ok(Type::Option(Box::new(inner_ty)));
                    }
                }

                let callee_ty = self.check_expr(callee)?;
                match callee_ty {
                    Type::Function {
                        params,
                        return_type,
                        ..
                    } => {
                        if args.len() != params.len() {
                            return Err(TypeError::ArgumentCountMismatch {
                                expected: params.len(),
                                found: args.len(),
                                span: *span,
                            });
                        }
                        for (arg, param_ty) in args.iter().zip(params.iter()) {
                            let arg_ty = self.check_expr(arg)?;
                            if &arg_ty != param_ty {
                                return Err(TypeError::TypeMismatch {
                                    expected: param_ty.to_string(),
                                    found: arg_ty.to_string(),
                                    span: arg.span(),
                                });
                            }
                        }
                        Ok(*return_type)
                    }
                    _ => Err(TypeError::NotCallable {
                        ty: callee_ty.to_string(),
                        span: *span,
                    }),
                }
            }
            Expr::FieldAccess {
                target,
                field,
                span,
            } => {
                let target_ty = self.check_expr(target)?;
                match target_ty {
                    Type::SelfType(entity_name) | Type::Entity(entity_name) => {
                        let entity_info = self.entities.get(&entity_name).ok_or_else(|| {
                            TypeError::UndefinedSymbol {
                                name: entity_name.clone(),
                                span: *span,
                            }
                        })?;
                        if let Some((field_ty, _)) = entity_info.fields.get(&field.name) {
                            Ok(field_ty.clone())
                        } else if let Some(method_sig) = entity_info.methods.get(&field.name) {
                            Ok(Type::Function {
                                params: method_sig
                                    .params
                                    .iter()
                                    .map(|(_, t, _)| t.clone())
                                    .collect(),
                                return_type: Box::new(method_sig.return_type.clone()),
                                effects: method_sig.effects.clone(),
                            })
                        } else {
                            Err(TypeError::UnknownField {
                                target_type: entity_name,
                                field: field.name.clone(),
                                span: field.span,
                            })
                        }
                    }
                    Type::Capability(cap_name) => {
                        let cap_info = self.capabilities.get(&cap_name).ok_or_else(|| {
                            TypeError::UndefinedSymbol {
                                name: cap_name.clone(),
                                span: *span,
                            }
                        })?;
                        if let Some(field_ty) = cap_info.fields.get(&field.name) {
                            Ok(field_ty.clone())
                        } else {
                            Err(TypeError::UnknownField {
                                target_type: cap_name,
                                field: field.name.clone(),
                                span: field.span,
                            })
                        }
                    }
                    _ => Err(TypeError::UnknownField {
                        target_type: target_ty.to_string(),
                        field: field.name.clone(),
                        span: field.span,
                    }),
                }
            }
            Expr::Old { expr, .. } => self.check_expr(expr),
            Expr::Block(block) => {
                self.enter_scope();
                for stmt in &block.statements {
                    self.check_stmt(stmt)?;
                }
                self.exit_scope();
                Ok(Type::Unit)
            }
        }
    }

    fn resolve_type_ref(&self, type_ref: &TypeRef) -> Result<Type, TypeError> {
        match type_ref {
            TypeRef::Primitive(prim, _) => Ok(match prim {
                PrimitiveType::Int => Type::Int,
                PrimitiveType::Float => Type::Float,
                PrimitiveType::Bool => Type::Bool,
                PrimitiveType::String => Type::String,
                PrimitiveType::Bytes => Type::Bytes,
                PrimitiveType::Unit => Type::Unit,
            }),
            TypeRef::Named(id) => match id.name.as_str() {
                "Int" => Ok(Type::Int),
                "Float" => Ok(Type::Float),
                "Bool" => Ok(Type::Bool),
                "String" => Ok(Type::String),
                "Bytes" => Ok(Type::Bytes),
                "Unit" => Ok(Type::Unit),
                name if self.entities.contains_key(name) => Ok(Type::Entity(name.to_string())),
                name if self.capabilities.contains_key(name) => {
                    Ok(Type::Capability(name.to_string()))
                }
                name if self.interfaces.contains_key(name) => Ok(Type::Interface(name.to_string())),
                _ => Ok(Type::Named(id.name.clone())),
            },
            TypeRef::Generic(id, args, span) => {
                if id.name == "Option" && args.len() == 1 {
                    let inner = self.resolve_type_ref(&args[0])?;
                    Ok(Type::Option(Box::new(inner)))
                } else if id.name == "Result" && args.len() == 2 {
                    let ok_ty = self.resolve_type_ref(&args[0])?;
                    let err_ty = self.resolve_type_ref(&args[1])?;
                    Ok(Type::Result(Box::new(ok_ty), Box::new(err_ty)))
                } else if id.name == "Result" && args.is_empty() {
                    Ok(Type::Result(
                        Box::new(Type::Unknown),
                        Box::new(Type::Unknown),
                    ))
                } else {
                    Err(TypeError::UndefinedSymbol {
                        name: format!("{}<{}>", id.name, args.len()),
                        span: *span,
                    })
                }
            }
        }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn define_var(&mut self, name: &str, ty: Type, is_mut: bool, span: Span) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), VarInfo { ty, is_mut, span });
        }
    }

    fn lookup_var(&self, name: &str) -> Option<&VarInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info);
            }
        }
        None
    }
}

fn types_compatible(expected: &Type, actual: &Type) -> bool {
    if expected == actual || actual == &Type::Unknown || expected == &Type::Unknown {
        return true;
    }

    match (expected, actual) {
        (Type::Result(e_ok, e_err), Type::Result(a_ok, a_err)) => {
            types_compatible(e_ok, a_ok) && types_compatible(e_err, a_err)
        }
        (Type::Option(e_inner), Type::Option(a_inner)) => types_compatible(e_inner, a_inner),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ordyn_parser::Parser;

    #[test]
    fn test_valid_program_typechecks() {
        let source = r#"
            capability NetConnect {
                host: String;
                port: Int;
            }

            entity Account {
                let id: String;
                mut balance: Int;

                pub fn deposit(amount: Int) -> Result<Int, String>
                    requires amount > 0
                    ensures self.balance == old(self.balance) + amount
                {
                    self.balance = self.balance + amount;
                    return Ok(self.balance);
                }
            }

            boundary MainBoundary {
                grant NetConnect { host: "localhost", port: 8080 };
                bind Account;
            }
        "#;

        let mut parser = Parser::from_source(source).unwrap();
        let program = parser.parse_program().unwrap();

        let mut checker = TypeChecker::new();
        assert!(checker.check_program(&program).is_ok());
    }

    #[test]
    fn test_immutable_assignment_rejected() {
        let source = r#"
            entity Account {
                let id: String;
                pub fn bad_assign() -> Unit {
                    self.id = "new_id";
                    return;
                }
            }
        "#;

        let mut parser = Parser::from_source(source).unwrap();
        let program = parser.parse_program().unwrap();

        let mut checker = TypeChecker::new();
        let err = checker.check_program(&program).unwrap_err();
        assert!(matches!(err, TypeError::ImmutableAssignment { .. }));
    }

    #[test]
    fn test_contract_non_boolean_rejected() {
        let source = r#"
            entity Account {
                mut balance: Int;
                pub fn bad_contract(amount: Int) -> Unit
                    requires amount + 10 // Invalid: Int expression instead of Bool
                {
                    return;
                }
            }
        "#;

        let mut parser = Parser::from_source(source).unwrap();
        let program = parser.parse_program().unwrap();

        let mut checker = TypeChecker::new();
        let err = checker.check_program(&program).unwrap_err();
        assert!(matches!(err, TypeError::ContractMustBeBool { .. }));
    }

    #[test]
    fn test_type_mismatch_rejected() {
        let source = r#"
            entity Calculator {
                pub fn calculate() -> Int {
                    let x: Int = "not_an_int";
                    return x;
                }
            }
        "#;

        let mut parser = Parser::from_source(source).unwrap();
        let program = parser.parse_program().unwrap();

        let mut checker = TypeChecker::new();
        let err = checker.check_program(&program).unwrap_err();
        assert!(matches!(err, TypeError::TypeMismatch { .. }));
    }
}
