use ordyn_ast::*;
use ordyn_diagnostics::Span;
use miette::Diagnostic;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug, Clone, PartialEq)]
pub enum EffectError {
    #[error("Unhandled effect `{effect}` in function `{function}`")]
    #[diagnostic(code(ordyn::effects::unhandled))]
    UnhandledEffect {
        effect: String,
        function: String,
        #[label("function `{function}` does not declare effect `{effect}`")]
        span: Span,
        #[label("effect required by operation or call here")]
        cause_span: Span,
    },

    #[error("Pure function `{function}` cannot perform effectful operation `{operation}`")]
    #[diagnostic(code(ordyn::effects::pure_violation))]
    PureViolation {
        operation: String,
        function: String,
        #[label("pure function cannot perform `{operation}`")]
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Effect {
    State,
    Storage,
    Net,
    Audit,
    Time,
    Entropy,
    Custom(String),
}

impl Effect {
    pub fn from_name(name: &str) -> Self {
        match name {
            "State" => Effect::State,
            "Storage" => Effect::Storage,
            "Net" => Effect::Net,
            "Audit" => Effect::Audit,
            "Time" => Effect::Time,
            "Entropy" => Effect::Entropy,
            custom => Effect::Custom(custom.to_string()),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Effect::State => "State",
            Effect::Storage => "Storage",
            Effect::Net => "Net",
            Effect::Audit => "Audit",
            Effect::Time => "Time",
            Effect::Entropy => "Entropy",
            Effect::Custom(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for Effect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EffectSet {
    effects: HashSet<Effect>,
}

impl EffectSet {
    pub fn empty() -> Self {
        Self {
            effects: HashSet::new(),
        }
    }

    pub fn insert(&mut self, effect: Effect) {
        self.effects.insert(effect);
    }

    pub fn contains(&self, effect: &Effect) -> bool {
        self.effects.contains(effect)
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    pub fn is_subset(&self, other: &EffectSet) -> bool {
        self.effects.is_subset(&other.effects)
    }

    pub fn difference(&self, other: &EffectSet) -> HashSet<Effect> {
        self.effects.difference(&other.effects).cloned().collect()
    }
}

pub struct EffectAnalyzer {
    function_effects: HashMap<String, EffectSet>,
    entity_method_effects: HashMap<(String, String), EffectSet>, // (Entity, Method) -> EffectSet
    interface_method_effects: HashMap<(String, String), EffectSet>, // (Interface, Method) -> EffectSet
}

impl Default for EffectAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectAnalyzer {
    pub fn new() -> Self {
        Self {
            function_effects: HashMap::new(),
            entity_method_effects: HashMap::new(),
            interface_method_effects: HashMap::new(),
        }
    }

    pub fn analyze_program(&mut self, program: &Program) -> Result<(), EffectError> {
        // Pass 1: Collect declared effects for all functions, methods, and interfaces
        for decl in &program.declarations {
            self.register_signatures(decl);
        }

        // Pass 2: Analyze bodies to verify effect conformance
        for decl in &program.declarations {
            self.analyze_declaration(decl)?;
        }

        Ok(())
    }

    fn register_signatures(&mut self, decl: &Declaration) {
        match decl {
            Declaration::Function(func) => {
                let mut set = EffectSet::empty();
                for eff in &func.effects {
                    set.insert(Effect::from_name(&eff.name));
                }
                self.function_effects.insert(func.name.name.clone(), set);
            }
            Declaration::Entity(entity) => {
                for member in &entity.members {
                    if let EntityMember::Method(m) = member {
                        let mut set = EffectSet::empty();
                        for eff in &m.effects {
                            set.insert(Effect::from_name(&eff.name));
                        }
                        self.entity_method_effects.insert(
                            (entity.name.name.clone(), m.name.name.clone()),
                            set,
                        );
                    }
                }
            }
            Declaration::Interface(iface) => {
                for m in &iface.methods {
                    let mut set = EffectSet::empty();
                    for eff in &m.effects {
                        set.insert(Effect::from_name(&eff.name));
                    }
                    self.interface_method_effects.insert(
                        (iface.name.name.clone(), m.name.name.clone()),
                        set,
                    );
                }
            }
            Declaration::Flow(flow) => {
                let mut set = EffectSet::empty();
                for eff in &flow.effects {
                    set.insert(Effect::from_name(&eff.name));
                }
                self.function_effects.insert(flow.name.name.clone(), set);
            }
            _ => {}
        }
    }

    fn analyze_declaration(&self, decl: &Declaration) -> Result<(), EffectError> {
        match decl {
            Declaration::Function(func) => {
                let declared_effects = self.function_effects.get(&func.name.name).cloned().unwrap_or_default();
                self.analyze_block(&func.body, &func.name.name, &declared_effects, None)?;
            }
            Declaration::Entity(entity) => {
                for member in &entity.members {
                    if let EntityMember::Method(m) = member {
                        let declared_effects = self.entity_method_effects
                            .get(&(entity.name.name.clone(), m.name.name.clone()))
                            .cloned()
                            .unwrap_or_default();
                        self.analyze_block(
                            &m.body,
                            &format!("{}::{}", entity.name.name, m.name.name),
                            &declared_effects,
                            Some(&entity.name.name),
                        )?;
                    }
                }
            }
            Declaration::Flow(flow) => {
                let declared_effects = self.function_effects.get(&flow.name.name).cloned().unwrap_or_default();
                for expr in &flow.body {
                    self.analyze_expr(expr, &flow.name.name, &declared_effects, None)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn analyze_block(
        &self,
        block: &Block,
        function_name: &str,
        declared_effects: &EffectSet,
        entity_name: Option<&str>,
    ) -> Result<(), EffectError> {
        for stmt in &block.statements {
            self.analyze_stmt(stmt, function_name, declared_effects, entity_name)?;
        }
        Ok(())
    }

    fn analyze_stmt(
        &self,
        stmt: &Stmt,
        function_name: &str,
        declared_effects: &EffectSet,
        entity_name: Option<&str>,
    ) -> Result<(), EffectError> {
        match stmt {
            Stmt::Let { init, .. } => {
                if let Some(expr) = init {
                    self.analyze_expr(expr, function_name, declared_effects, entity_name)?;
                }
            }
            Stmt::Assign { target, value, span } => {
                // Mutating entity field requires either enclosing entity context or State effect
                if let Expr::FieldAccess { target: inner_target, .. } = target {
                    if let Expr::Ident(id) = &**inner_target {
                        if id.name == "self" {
                            // Local mutation inside an entity method is permitted; outside an entity, require State
                            if entity_name.is_none() && !declared_effects.contains(&Effect::State) {
                                return Err(EffectError::UnhandledEffect {
                                    effect: "State".to_string(),
                                    function: function_name.to_string(),
                                    span: *span,
                                    cause_span: *span,
                                });
                            }
                        }
                    }
                }
                self.analyze_expr(target, function_name, declared_effects, entity_name)?;
                self.analyze_expr(value, function_name, declared_effects, entity_name)?;
            }
            Stmt::Return { value, .. } => {
                if let Some(expr) = value {
                    self.analyze_expr(expr, function_name, declared_effects, entity_name)?;
                }
            }
            Stmt::Expr(expr, _) => {
                self.analyze_expr(expr, function_name, declared_effects, entity_name)?;
            }
        }
        Ok(())
    }

    fn analyze_expr(
        &self,
        expr: &Expr,
        function_name: &str,
        declared_effects: &EffectSet,
        entity_name: Option<&str>,
    ) -> Result<(), EffectError> {
        match expr {
            Expr::Call { callee, args, span } => {
                // If callee is a known function or method, verify its required effects are allowed
                if let Expr::Ident(id) = &**callee {
                    if let Some(callee_effects) = self.function_effects.get(&id.name) {
                        for required_effect in &callee_effects.effects {
                            if !declared_effects.contains(required_effect) {
                                return Err(EffectError::UnhandledEffect {
                                    effect: required_effect.to_string(),
                                    function: function_name.to_string(),
                                    span: *span,
                                    cause_span: id.span,
                                });
                            }
                        }
                    }
                } else if let Expr::FieldAccess { target, field, .. } = &**callee {
                    // Check entity method effects
                    if let Expr::Ident(target_id) = &**target {
                        if target_id.name == "self" {
                            if let Some(e_name) = entity_name {
                                if let Some(m_effects) = self.entity_method_effects.get(&(e_name.to_string(), field.name.clone())) {
                                    for required_effect in &m_effects.effects {
                                        if !declared_effects.contains(required_effect) {
                                            return Err(EffectError::UnhandledEffect {
                                                effect: required_effect.to_string(),
                                                function: function_name.to_string(),
                                                span: *span,
                                                cause_span: field.span,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                self.analyze_expr(callee, function_name, declared_effects, entity_name)?;
                for arg in args {
                    self.analyze_expr(arg, function_name, declared_effects, entity_name)?;
                }
            }
            Expr::Binary { left, right, .. } => {
                self.analyze_expr(left, function_name, declared_effects, entity_name)?;
                self.analyze_expr(right, function_name, declared_effects, entity_name)?;
            }
            Expr::Unary { operand, .. } => {
                self.analyze_expr(operand, function_name, declared_effects, entity_name)?;
            }
            Expr::FieldAccess { target, .. } => {
                self.analyze_expr(target, function_name, declared_effects, entity_name)?;
            }
            Expr::Old { expr, .. } => {
                self.analyze_expr(expr, function_name, declared_effects, entity_name)?;
            }
            Expr::Block(block) => {
                self.analyze_block(block, function_name, declared_effects, entity_name)?;
            }
            Expr::Literal(..) | Expr::Ident(..) => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ordyn_parser::Parser;

    #[test]
    fn test_pure_function_calling_pure_function_allowed() {
        let source = r#"
            fn add_one(x: Int) -> Int {
                return x + 1;
            }

            fn compute(val: Int) -> Int {
                return add_one(val);
            }
        "#;

        let mut parser = Parser::from_source(source).unwrap();
        let program = parser.parse_program().unwrap();

        let mut analyzer = EffectAnalyzer::new();
        assert!(analyzer.analyze_program(&program).is_ok());
    }

    #[test]
    fn test_pure_function_calling_effectful_function_rejected() {
        let source = r#"
            fn send_packet(payload: String) -> Unit with [Net] {
                return;
            }

            fn pure_computation() -> Unit {
                send_packet("test"); // Error: Net effect not declared
                return;
            }
        "#;

        let mut parser = Parser::from_source(source).unwrap();
        let program = parser.parse_program().unwrap();

        let mut analyzer = EffectAnalyzer::new();
        let err = analyzer.analyze_program(&program).unwrap_err();
        assert!(matches!(err, EffectError::UnhandledEffect { effect, .. } if effect == "Net"));
    }

    #[test]
    fn test_effectful_caller_with_matching_effects_allowed() {
        let source = r#"
            fn perform_io() -> Unit with [Storage, Audit] {
                return;
            }

            fn orchestrator() -> Unit with [Storage, Audit, Net] {
                perform_io();
                return;
            }
        "#;

        let mut parser = Parser::from_source(source).unwrap();
        let program = parser.parse_program().unwrap();

        let mut analyzer = EffectAnalyzer::new();
        assert!(analyzer.analyze_program(&program).is_ok());
    }
}
