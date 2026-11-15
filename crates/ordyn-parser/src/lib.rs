use miette::Diagnostic;
use ordyn_ast::*;
use ordyn_diagnostics::Span;
use ordyn_lexer::{LexError, Lexer, Token, TokenKind};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug, Clone, PartialEq)]
pub enum ParseError {
    #[error("Lexer error: {0}")]
    #[diagnostic(transparent)]
    Lex(#[from] LexError),

    #[error("Unexpected token: expected {expected}, found {found:?}")]
    #[diagnostic(code(ordyn::parser::unexpected_token))]
    UnexpectedToken {
        expected: String,
        found: TokenKind,
        #[label("expected {expected} here")]
        span: Span,
    },

    #[error("Unexpected end of file: expected {expected}")]
    #[diagnostic(code(ordyn::parser::unexpected_eof))]
    UnexpectedEof {
        expected: String,
        #[label("expected {expected} before EOF")]
        span: Span,
    },
}

pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0 }
    }

    pub fn from_source(source: &str) -> Result<Self, ParseError> {
        let lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        Ok(Self::new(tokens))
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let start_span = self.current_span();
        let mut declarations = Vec::new();

        while !self.is_at_end() {
            let decl = self.parse_declaration()?;
            declarations.push(decl);
        }

        let end_span = self.previous_span();
        let span = start_span.merge(end_span);

        Ok(Program { declarations, span })
    }

    fn parse_declaration(&mut self) -> Result<Declaration, ParseError> {
        match self.peek() {
            TokenKind::Entity => self.parse_entity_decl().map(Declaration::Entity),
            TokenKind::Capability => self.parse_capability_decl().map(Declaration::Capability),
            TokenKind::Contract => self.parse_contract_decl().map(Declaration::Contract),
            TokenKind::Flow => self.parse_flow_decl().map(Declaration::Flow),
            TokenKind::Boundary => self.parse_boundary_decl().map(Declaration::Boundary),
            TokenKind::Interface => self.parse_interface_decl().map(Declaration::Interface),
            TokenKind::Fn | TokenKind::Pub => self.parse_function_decl().map(Declaration::Function),
            _ => {
                let token = self.advance();
                Err(ParseError::UnexpectedToken {
                    expected:
                        "declaration (entity, capability, contract, flow, boundary, interface, fn)"
                            .to_string(),
                    found: token.kind,
                    span: token.span,
                })
            }
        }
    }

    fn parse_entity_decl(&mut self) -> Result<EntityDecl, ParseError> {
        let start = self.expect(TokenKind::Entity)?.span;
        let name = self.parse_ident()?;
        self.expect(TokenKind::LBrace)?;

        let mut members = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let member = self.parse_entity_member()?;
            members.push(member);
        }

        let end = self.expect(TokenKind::RBrace)?.span;
        Ok(EntityDecl {
            name,
            members,
            span: start.merge(end),
        })
    }

    fn parse_entity_member(&mut self) -> Result<EntityMember, ParseError> {
        let is_pub = self.match_token(&TokenKind::Pub);
        if self.check(&TokenKind::Fn) {
            let func = self.parse_function_decl_internal(is_pub)?;
            Ok(EntityMember::Method(func))
        } else {
            let is_mut = self.match_token(&TokenKind::Mut);
            if !is_mut && self.check(&TokenKind::Let) {
                self.advance();
            }
            let name = self.parse_ident()?;
            self.expect(TokenKind::Colon)?;
            let ty = self.parse_type()?;

            let init = if self.match_token(&TokenKind::Eq) {
                Some(self.parse_expr()?)
            } else {
                None
            };

            let end = self.expect(TokenKind::Semicolon)?.span;
            Ok(EntityMember::Field(FieldDecl {
                is_pub,
                is_mut,
                name: name.clone(),
                ty,
                init,
                span: name.span.merge(end),
            }))
        }
    }

    fn parse_capability_decl(&mut self) -> Result<CapabilityDecl, ParseError> {
        let start = self.expect(TokenKind::Capability)?.span;
        let name = self.parse_ident()?;
        self.expect(TokenKind::LBrace)?;

        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let field_name = self.parse_ident()?;
            self.expect(TokenKind::Colon)?;
            let ty = self.parse_type()?;
            let semi = self.expect(TokenKind::Semicolon)?.span;

            fields.push(FieldDecl {
                is_pub: true,
                is_mut: false,
                name: field_name.clone(),
                ty,
                init: None,
                span: field_name.span.merge(semi),
            });
        }

        let end = self.expect(TokenKind::RBrace)?.span;
        Ok(CapabilityDecl {
            name,
            fields,
            span: start.merge(end),
        })
    }

    fn parse_contract_decl(&mut self) -> Result<ContractDecl, ParseError> {
        let start = self.expect(TokenKind::Contract)?.span;
        let name = self.parse_ident()?;
        self.expect(TokenKind::LBrace)?;

        let mut clauses = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            clauses.push(self.parse_contract_clause()?);
        }

        let end = self.expect(TokenKind::RBrace)?.span;
        Ok(ContractDecl {
            name,
            clauses,
            span: start.merge(end),
        })
    }

    fn parse_contract_clause(&mut self) -> Result<ContractClause, ParseError> {
        if self.match_token(&TokenKind::Requires) {
            let expr = self.parse_expr()?;
            let end = self.expect(TokenKind::Semicolon)?.span;
            let span = expr.span().merge(end);
            Ok(ContractClause::Requires(expr, span))
        } else if self.match_token(&TokenKind::Ensures) {
            let expr = self.parse_expr()?;
            let end = self.expect(TokenKind::Semicolon)?.span;
            let span = expr.span().merge(end);
            Ok(ContractClause::Ensures(expr, span))
        } else if self.match_token(&TokenKind::Invariant) {
            let expr = self.parse_expr()?;
            let end = self.expect(TokenKind::Semicolon)?.span;
            let span = expr.span().merge(end);
            Ok(ContractClause::Invariant(expr, span))
        } else {
            let token = self.advance();
            Err(ParseError::UnexpectedToken {
                expected: "contract clause (requires, ensures, invariant)".to_string(),
                found: token.kind,
                span: token.span,
            })
        }
    }

    fn parse_flow_decl(&mut self) -> Result<FlowDecl, ParseError> {
        let start = self.expect(TokenKind::Flow)?.span;
        let name = self.parse_ident()?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(TokenKind::RParen)?;

        let return_type = if self.match_token(&TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let effects = self.parse_effect_clause()?;
        self.expect(TokenKind::LBrace)?;

        let mut body = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_expr()?);
            self.match_token(&TokenKind::Arrow);
        }

        let end = self.expect(TokenKind::RBrace)?.span;
        Ok(FlowDecl {
            name,
            params,
            return_type,
            effects,
            body,
            span: start.merge(end),
        })
    }

    fn parse_boundary_decl(&mut self) -> Result<BoundaryDecl, ParseError> {
        let start = self.expect(TokenKind::Boundary)?.span;
        let name = self.parse_ident()?;
        self.expect(TokenKind::LBrace)?;

        let mut grants = Vec::new();
        let mut quota = None;
        let mut bindings = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.match_token(&TokenKind::Grant) {
                let cap_name = self.parse_ident()?;
                self.expect(TokenKind::LBrace)?;
                let mut args = Vec::new();
                while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                    let key = self.parse_ident()?;
                    self.expect(TokenKind::Colon)?;
                    let val = self.parse_expr()?;
                    args.push((key, val));
                    self.match_token(&TokenKind::Comma);
                }
                self.expect(TokenKind::RBrace)?;
                let end = self.expect(TokenKind::Semicolon)?.span;
                grants.push(GrantClause {
                    capability_name: cap_name.clone(),
                    arguments: args,
                    span: cap_name.span.merge(end),
                });
            } else if self.match_token(&TokenKind::Quota) {
                let q_start = self.previous_span();
                self.expect(TokenKind::LBrace)?;
                let mut mem = None;
                let mut fuel = None;
                while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                    let key = self.parse_ident()?;
                    self.expect(TokenKind::Colon)?;
                    let val_expr = self.parse_expr()?;
                    self.expect(TokenKind::Semicolon)?;

                    if key.name == "memory_limit" {
                        if let Expr::Literal(Literal::Integer(i), _) = val_expr {
                            mem = Some(i as u64);
                        }
                    } else if key.name == "fuel_limit" {
                        if let Expr::Literal(Literal::Integer(i), _) = val_expr {
                            fuel = Some(i as u64);
                        }
                    }
                }
                let q_end = self.expect(TokenKind::RBrace)?.span;
                quota = Some(QuotaClause {
                    memory_limit: mem,
                    fuel_limit: fuel,
                    span: q_start.merge(q_end),
                });
            } else if self.match_token(&TokenKind::Bind) {
                let entity_name = self.parse_ident()?;
                self.expect(TokenKind::Semicolon)?;
                bindings.push(entity_name);
            } else {
                let token = self.advance();
                return Err(ParseError::UnexpectedToken {
                    expected: "boundary member (grant, quota, bind)".to_string(),
                    found: token.kind,
                    span: token.span,
                });
            }
        }

        let end = self.expect(TokenKind::RBrace)?.span;
        Ok(BoundaryDecl {
            name,
            grants,
            quota,
            bindings,
            span: start.merge(end),
        })
    }

    fn parse_interface_decl(&mut self) -> Result<InterfaceDecl, ParseError> {
        let start = self.expect(TokenKind::Interface)?.span;
        let name = self.parse_ident()?;
        self.expect(TokenKind::LBrace)?;

        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let m_start = self.expect(TokenKind::Fn)?.span;
            let m_name = self.parse_ident()?;
            self.expect(TokenKind::LParen)?;
            let params = self.parse_param_list()?;
            self.expect(TokenKind::RParen)?;

            let return_type = if self.match_token(&TokenKind::Arrow) {
                Some(self.parse_type()?)
            } else {
                None
            };

            let effects = self.parse_effect_clause()?;
            let end = self.expect(TokenKind::Semicolon)?.span;

            methods.push(InterfaceMethod {
                name: m_name,
                params,
                return_type,
                effects,
                span: m_start.merge(end),
            });
        }

        let end = self.expect(TokenKind::RBrace)?.span;
        Ok(InterfaceDecl {
            name,
            methods,
            span: start.merge(end),
        })
    }

    fn parse_function_decl(&mut self) -> Result<FunctionDecl, ParseError> {
        let is_pub = self.match_token(&TokenKind::Pub);
        self.parse_function_decl_internal(is_pub)
    }

    fn parse_function_decl_internal(&mut self, is_pub: bool) -> Result<FunctionDecl, ParseError> {
        let start = self.expect(TokenKind::Fn)?.span;
        let name = self.parse_ident()?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(TokenKind::RParen)?;

        let return_type = if self.match_token(&TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let effects = self.parse_effect_clause()?;

        let mut contracts = Vec::new();
        while self.check(&TokenKind::Requires)
            || self.check(&TokenKind::Ensures)
            || self.check(&TokenKind::Invariant)
        {
            if self.match_token(&TokenKind::Requires) {
                let expr = self.parse_expr()?;
                let span = expr.span();
                contracts.push(ContractClause::Requires(expr, span));
            } else if self.match_token(&TokenKind::Ensures) {
                let expr = self.parse_expr()?;
                let span = expr.span();
                contracts.push(ContractClause::Ensures(expr, span));
            } else if self.match_token(&TokenKind::Invariant) {
                let expr = self.parse_expr()?;
                let span = expr.span();
                contracts.push(ContractClause::Invariant(expr, span));
            }
        }

        let body = self.parse_block()?;
        let span = start.merge(body.span);

        Ok(FunctionDecl {
            is_pub,
            name,
            params,
            return_type,
            effects,
            contracts,
            body,
            span,
        })
    }

    fn parse_effect_clause(&mut self) -> Result<Vec<Ident>, ParseError> {
        if self.match_token(&TokenKind::With) {
            self.expect(TokenKind::LBracket)?;
            let mut effects = Vec::new();
            while !self.check(&TokenKind::RBracket) && !self.is_at_end() {
                effects.push(self.parse_ident()?);
                self.match_token(&TokenKind::Comma);
            }
            self.expect(TokenKind::RBracket)?;
            Ok(effects)
        } else {
            Ok(Vec::new())
        }
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();
        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
            let is_mut = self.match_token(&TokenKind::Mut);
            let name = self.parse_ident()?;
            self.expect(TokenKind::Colon)?;
            let ty = self.parse_type()?;
            let span = name.span.merge(ty.span());
            params.push(Param {
                is_mut,
                name,
                ty,
                span,
            });
            self.match_token(&TokenKind::Comma);
        }
        Ok(params)
    }

    fn parse_type(&mut self) -> Result<TypeRef, ParseError> {
        let ident = self.parse_ident()?;
        match ident.name.as_str() {
            "Int" => Ok(TypeRef::Primitive(PrimitiveType::Int, ident.span)),
            "Float" => Ok(TypeRef::Primitive(PrimitiveType::Float, ident.span)),
            "Bool" => Ok(TypeRef::Primitive(PrimitiveType::Bool, ident.span)),
            "String" => Ok(TypeRef::Primitive(PrimitiveType::String, ident.span)),
            "Bytes" => Ok(TypeRef::Primitive(PrimitiveType::Bytes, ident.span)),
            "Unit" => Ok(TypeRef::Primitive(PrimitiveType::Unit, ident.span)),
            _ => {
                if self.match_token(&TokenKind::Lt) {
                    let mut type_args = Vec::new();
                    while !self.check(&TokenKind::Gt) && !self.is_at_end() {
                        type_args.push(self.parse_type()?);
                        self.match_token(&TokenKind::Comma);
                    }
                    let end = self.expect(TokenKind::Gt)?.span;
                    Ok(TypeRef::Generic(
                        ident.clone(),
                        type_args,
                        ident.span.merge(end),
                    ))
                } else {
                    Ok(TypeRef::Named(ident))
                }
            }
        }
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let start = self.expect(TokenKind::LBrace)?.span;
        let mut statements = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }

        let end = self.expect(TokenKind::RBrace)?.span;
        Ok(Block {
            statements,
            span: start.merge(end),
        })
    }

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        if self.match_token(&TokenKind::Let) {
            let is_mut = self.match_token(&TokenKind::Mut);
            let name = self.parse_ident()?;
            let ty = if self.match_token(&TokenKind::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };

            let init = if self.match_token(&TokenKind::Eq) {
                Some(self.parse_expr()?)
            } else {
                None
            };

            let end = self.expect(TokenKind::Semicolon)?.span;
            Ok(Stmt::Let {
                is_mut,
                name: name.clone(),
                ty,
                init,
                span: name.span.merge(end),
            })
        } else if self.match_token(&TokenKind::Return) {
            let start = self.previous_span();
            let value = if !self.check(&TokenKind::Semicolon) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            let end = self.expect(TokenKind::Semicolon)?.span;
            Ok(Stmt::Return {
                value,
                span: start.merge(end),
            })
        } else {
            let expr = self.parse_expr()?;
            if self.match_token(&TokenKind::Eq) {
                let val = self.parse_expr()?;
                let end = self.expect(TokenKind::Semicolon)?.span;
                Ok(Stmt::Assign {
                    target: expr.clone(),
                    value: val,
                    span: expr.span().merge(end),
                })
            } else {
                let end = self.expect(TokenKind::Semicolon)?.span;
                let span = expr.span().merge(end);
                Ok(Stmt::Expr(expr, span))
            }
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_logical_and()?;
        while self.match_token(&TokenKind::OrOr) {
            let right = self.parse_logical_and()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_equality()?;
        while self.match_token(&TokenKind::AndAnd) {
            let right = self.parse_equality()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_relational()?;
        while self.check(&TokenKind::EqEq) || self.check(&TokenKind::NotEq) {
            let op = if self.match_token(&TokenKind::EqEq) {
                BinaryOp::Eq
            } else {
                self.advance();
                BinaryOp::Neq
            };
            let right = self.parse_relational()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_additive()?;
        while self.check(&TokenKind::Lt)
            || self.check(&TokenKind::Lte)
            || self.check(&TokenKind::Gt)
            || self.check(&TokenKind::Gte)
        {
            let op = if self.match_token(&TokenKind::Lt) {
                BinaryOp::Lt
            } else if self.match_token(&TokenKind::Lte) {
                BinaryOp::Lte
            } else if self.match_token(&TokenKind::Gt) {
                BinaryOp::Gt
            } else {
                self.advance();
                BinaryOp::Gte
            };
            let right = self.parse_additive()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_multiplicative()?;
        while self.check(&TokenKind::Plus) || self.check(&TokenKind::Minus) {
            let op = if self.match_token(&TokenKind::Plus) {
                BinaryOp::Add
            } else {
                self.advance();
                BinaryOp::Sub
            };
            let right = self.parse_multiplicative()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        while self.check(&TokenKind::Star)
            || self.check(&TokenKind::Slash)
            || self.check(&TokenKind::Percent)
        {
            let op = if self.match_token(&TokenKind::Star) {
                BinaryOp::Mul
            } else if self.match_token(&TokenKind::Slash) {
                BinaryOp::Div
            } else {
                self.advance();
                BinaryOp::Mod
            };
            let right = self.parse_unary()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.match_token(&TokenKind::Bang) {
            let start = self.previous_span();
            let operand = self.parse_unary()?;
            let span = start.merge(operand.span());
            Ok(Expr::Unary {
                op: UnaryOp::Not,
                operand: Box::new(operand),
                span,
            })
        } else if self.match_token(&TokenKind::Minus) {
            let start = self.previous_span();
            let operand = self.parse_unary()?;
            let span = start.merge(operand.span());
            Ok(Expr::Unary {
                op: UnaryOp::Neg,
                operand: Box::new(operand),
                span,
            })
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let mut expr = match self.peek() {
            TokenKind::IntLiteral(i) => {
                let token = self.advance();
                Expr::Literal(Literal::Integer(i), token.span)
            }
            TokenKind::FloatLiteral(f) => {
                let token = self.advance();
                Expr::Literal(Literal::Float(f), token.span)
            }
            TokenKind::StringLiteral(s) => {
                let token = self.advance();
                Expr::Literal(Literal::String(s), token.span)
            }
            TokenKind::True => {
                let token = self.advance();
                Expr::Literal(Literal::Bool(true), token.span)
            }
            TokenKind::False => {
                let token = self.advance();
                Expr::Literal(Literal::Bool(false), token.span)
            }
            TokenKind::None => {
                let token = self.advance();
                Expr::Literal(Literal::None, token.span)
            }
            TokenKind::Old => {
                let start = self.advance().span;
                self.expect(TokenKind::LParen)?;
                let inner = self.parse_expr()?;
                let end = self.expect(TokenKind::RParen)?.span;
                Expr::Old {
                    expr: Box::new(inner),
                    span: start.merge(end),
                }
            }
            TokenKind::Ident(_) => {
                let ident = self.parse_ident()?;
                Expr::Ident(ident)
            }
            TokenKind::LParen => {
                self.advance();
                let inner = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                inner
            }
            _ => {
                let token = self.advance();
                return Err(ParseError::UnexpectedToken {
                    expected: "expression".to_string(),
                    found: token.kind,
                    span: token.span,
                });
            }
        };

        // Postfix calls and member accesses
        loop {
            if self.match_token(&TokenKind::LParen) {
                let mut args = Vec::new();
                while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                    args.push(self.parse_expr()?);
                    self.match_token(&TokenKind::Comma);
                }
                let end = self.expect(TokenKind::RParen)?.span;
                expr = Expr::Call {
                    span: expr.span().merge(end),
                    callee: Box::new(expr),
                    args,
                };
            } else if self.match_token(&TokenKind::Dot) {
                let field = self.parse_ident()?;
                let span = expr.span().merge(field.span);
                expr = Expr::FieldAccess {
                    span,
                    target: Box::new(expr),
                    field,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_ident(&mut self) -> Result<Ident, ParseError> {
        if let TokenKind::Ident(name) = self.peek() {
            let token = self.advance();
            Ok(Ident {
                name,
                span: token.span,
            })
        } else {
            let token = self.advance();
            Err(ParseError::UnexpectedToken {
                expected: "identifier".to_string(),
                found: token.kind,
                span: token.span,
            })
        }
    }

    fn peek(&self) -> TokenKind {
        self.tokens
            .get(self.cursor)
            .map(|t| t.kind.clone())
            .unwrap_or(TokenKind::Eof)
    }

    fn check(&self, kind: &TokenKind) -> bool {
        &self.peek() == kind
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.cursor += 1;
        }
        self.tokens[self.cursor - 1].clone()
    }

    fn is_at_end(&self) -> bool {
        self.cursor >= self.tokens.len() || self.peek() == TokenKind::Eof
    }

    fn expect(&mut self, expected: TokenKind) -> Result<Token, ParseError> {
        if self.check(&expected) {
            Ok(self.advance())
        } else {
            let current = self.advance();
            Err(ParseError::UnexpectedToken {
                expected: format!("{:?}", expected),
                found: current.kind,
                span: current.span,
            })
        }
    }

    fn current_span(&self) -> Span {
        self.tokens
            .get(self.cursor)
            .map(|t| t.span)
            .unwrap_or_default()
    }

    fn previous_span(&self) -> Span {
        if self.cursor > 0 {
            self.tokens[self.cursor - 1].span
        } else {
            Span::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_entity_and_contracts() {
        let source = r#"
            entity BankAccount {
                let id: String;
                mut balance: Int;

                pub fn deposit(amount: Int) -> Result
                    requires amount > 0
                    ensures self.balance == old(self.balance) + amount
                {
                    self.balance = self.balance + amount;
                    return Ok(self.balance);
                }
            }
        "#;

        let mut parser = Parser::from_source(source).expect("lexer should succeed");
        let program = parser.parse_program().expect("parse should succeed");
        assert_eq!(program.declarations.len(), 1);

        if let Declaration::Entity(entity) = &program.declarations[0] {
            assert_eq!(entity.name.name, "BankAccount");
            assert_eq!(entity.members.len(), 3);
        } else {
            panic!("Expected Entity declaration");
        }
    }

    #[test]
    fn test_parse_all_six_constructs() {
        let source = r#"
            capability NetConnect {
                host: String;
                port: Int;
            }

            interface Logger {
                fn log(msg: String) -> Unit with [NetConnect];
            }

            contract SecureBounds {
                invariant balance >= 0;
            }

            boundary Gateway {
                grant NetConnect { host: "example.internal", port: 443 };
                quota {
                    memory_limit: 64;
                    fuel_limit: 1000;
                }
            }
        "#;

        let mut parser = Parser::from_source(source).unwrap();
        let program = parser.parse_program().unwrap();
        assert_eq!(program.declarations.len(), 4);
    }
}
