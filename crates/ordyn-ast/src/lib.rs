// Re-export Span for convenience
pub use ordyn_diagnostics::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub declarations: Vec<Declaration>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    Entity(EntityDecl),
    Capability(CapabilityDecl),
    Contract(ContractDecl),
    Flow(FlowDecl),
    Boundary(BoundaryDecl),
    Interface(InterfaceDecl),
    Function(FunctionDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityDecl {
    pub name: Ident,
    pub members: Vec<EntityMember>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntityMember {
    Field(FieldDecl),
    Method(FunctionDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityDecl {
    pub name: Ident,
    pub fields: Vec<FieldDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContractDecl {
    pub name: Ident,
    pub clauses: Vec<ContractClause>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContractClause {
    Requires(Expr, Span),
    Ensures(Expr, Span),
    Invariant(Expr, Span),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlowDecl {
    pub name: Ident,
    pub params: Vec<Param>,
    pub return_type: Option<TypeRef>,
    pub effects: Vec<Ident>,
    pub body: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoundaryDecl {
    pub name: Ident,
    pub grants: Vec<GrantClause>,
    pub quota: Option<QuotaClause>,
    pub bindings: Vec<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantClause {
    pub capability_name: Ident,
    pub arguments: Vec<(Ident, Expr)>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuotaClause {
    pub memory_limit: Option<u64>,
    pub fuel_limit: Option<u64>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDecl {
    pub name: Ident,
    pub methods: Vec<InterfaceMethod>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceMethod {
    pub name: Ident,
    pub params: Vec<Param>,
    pub return_type: Option<TypeRef>,
    pub effects: Vec<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub is_pub: bool,
    pub name: Ident,
    pub params: Vec<Param>,
    pub return_type: Option<TypeRef>,
    pub effects: Vec<Ident>,
    pub contracts: Vec<ContractClause>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub is_mut: bool,
    pub name: Ident,
    pub ty: TypeRef,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub is_pub: bool,
    pub is_mut: bool,
    pub name: Ident,
    pub ty: TypeRef,
    pub init: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeRef {
    Primitive(PrimitiveType, Span),
    Named(Ident),
    Generic(Ident, Vec<TypeRef>, Span),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Int,
    Float,
    Bool,
    String,
    Bytes,
    Unit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        is_mut: bool,
        name: Ident,
        ty: Option<TypeRef>,
        init: Option<Expr>,
        span: Span,
    },
    Assign {
        target: Expr,
        value: Expr,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    If {
        condition: Expr,
        then_branch: Block,
        else_branch: Option<Block>,
        span: Span,
    },
    Expr(Expr, Span),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal, Span),
    Ident(Ident),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    FieldAccess {
        target: Box<Expr>,
        field: Ident,
        span: Span,
    },
    Old {
        expr: Box<Expr>,
        span: Span,
    },
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
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
pub enum UnaryOp {
    Neg,
    Not,
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. }
            | Stmt::Assign { span, .. }
            | Stmt::Return { span, .. }
            | Stmt::If { span, .. }
            | Stmt::Expr(_, span) => *span,
        }
    }
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal(_, s) => *s,
            Expr::Ident(id) => id.span,
            Expr::Binary { span, .. } => *span,
            Expr::Unary { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::FieldAccess { span, .. } => *span,
            Expr::Old { span, .. } => *span,
            Expr::Block(b) => b.span,
        }
    }
}

impl TypeRef {
    pub fn span(&self) -> Span {
        match self {
            TypeRef::Primitive(_, s) => *s,
            TypeRef::Named(id) => id.span,
            TypeRef::Generic(_, _, s) => *s,
        }
    }
}
