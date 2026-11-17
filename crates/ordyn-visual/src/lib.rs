use ordyn_ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct VisualNode {
    pub id: String,
    pub name: String,
    pub kind: String, // "Entity", "Boundary", "Capability", "Interface", "Contract", "Flow"
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualEdge {
    pub source: String,
    pub target: String,
    pub relation: String, // "grants", "binds", "constrains", "calls"
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ArchitectureGraph {
    pub nodes: Vec<VisualNode>,
    pub edges: Vec<VisualEdge>,
}

impl ArchitectureGraph {
    pub fn from_program(program: &Program) -> Self {
        let mut graph = ArchitectureGraph::default();

        for decl in &program.declarations {
            match decl {
                Declaration::Boundary(b) => {
                    let mut props = HashMap::new();
                    if let Some(quota) = &b.quota {
                        if let Some(m) = quota.memory_limit {
                            props.insert("memory_limit_mb".to_string(), m.to_string());
                        }
                        if let Some(f) = quota.fuel_limit {
                            props.insert("fuel_limit".to_string(), f.to_string());
                        }
                    }

                    graph.nodes.push(VisualNode {
                        id: format!("boundary_{}", b.name.name),
                        name: b.name.name.clone(),
                        kind: "Boundary".to_string(),
                        properties: props,
                    });

                    // Grants: Boundary -> Capability
                    for grant in &b.grants {
                        graph.edges.push(VisualEdge {
                            source: format!("boundary_{}", b.name.name),
                            target: format!("cap_{}", grant.capability_name.name),
                            relation: "grants".to_string(),
                        });
                    }

                    // Bindings: Boundary -> Entity
                    for binding in &b.bindings {
                        graph.edges.push(VisualEdge {
                            source: format!("boundary_{}", b.name.name),
                            target: format!("entity_{}", binding.name),
                            relation: "binds".to_string(),
                        });
                    }
                }
                Declaration::Entity(e) => {
                    let mut props = HashMap::new();
                    let field_count = e
                        .members
                        .iter()
                        .filter(|m| matches!(m, EntityMember::Field(_)))
                        .count();
                    let method_count = e
                        .members
                        .iter()
                        .filter(|m| matches!(m, EntityMember::Method(_)))
                        .count();
                    props.insert("fields".to_string(), field_count.to_string());
                    props.insert("methods".to_string(), method_count.to_string());

                    graph.nodes.push(VisualNode {
                        id: format!("entity_{}", e.name.name),
                        name: e.name.name.clone(),
                        kind: "Entity".to_string(),
                        properties: props,
                    });
                }
                Declaration::Capability(c) => {
                    let mut props = HashMap::new();
                    props.insert("fields".to_string(), c.fields.len().to_string());

                    graph.nodes.push(VisualNode {
                        id: format!("cap_{}", c.name.name),
                        name: c.name.name.clone(),
                        kind: "Capability".to_string(),
                        properties: props,
                    });
                }
                Declaration::Interface(i) => {
                    let mut props = HashMap::new();
                    props.insert("methods".to_string(), i.methods.len().to_string());

                    graph.nodes.push(VisualNode {
                        id: format!("iface_{}", i.name.name),
                        name: i.name.name.clone(),
                        kind: "Interface".to_string(),
                        properties: props,
                    });
                }
                Declaration::Contract(k) => {
                    let mut props = HashMap::new();
                    props.insert("clauses".to_string(), k.clauses.len().to_string());

                    graph.nodes.push(VisualNode {
                        id: format!("contract_{}", k.name.name),
                        name: k.name.name.clone(),
                        kind: "Contract".to_string(),
                        properties: props,
                    });
                }
                Declaration::Flow(fl) => {
                    let mut props = HashMap::new();
                    props.insert("steps".to_string(), fl.body.len().to_string());

                    graph.nodes.push(VisualNode {
                        id: format!("flow_{}", fl.name.name),
                        name: fl.name.name.clone(),
                        kind: "Flow".to_string(),
                        properties: props,
                    });
                }
                Declaration::Function(f) => {
                    graph.nodes.push(VisualNode {
                        id: format!("fn_{}", f.name.name),
                        name: f.name.name.clone(),
                        kind: "Function".to_string(),
                        properties: HashMap::new(),
                    });
                }
                Declaration::Enum(en) => {
                    let mut props = HashMap::new();
                    props.insert("variants".to_string(), en.variants.len().to_string());
                    graph.nodes.push(VisualNode {
                        id: format!("enum_{}", en.name.name),
                        name: en.name.name.clone(),
                        kind: "Enum".to_string(),
                        properties: props,
                    });
                }
            }
        }

        graph
    }

    pub fn to_mermaid(&self) -> String {
        let mut mermaid = String::from("graph TD\n");
        for node in &self.nodes {
            mermaid.push_str(&format!(
                "    {}[\"{}: {}\"]\n",
                node.id, node.kind, node.name
            ));
        }
        for edge in &self.edges {
            mermaid.push_str(&format!(
                "    {} -->|{}| {}\n",
                edge.source, edge.relation, edge.target
            ));
        }
        mermaid
    }

    pub fn to_interactive_html(&self, title: &str) -> String {
        let mermaid_chart = self.to_mermaid();
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>ORDYN Visual Architecture - {title}</title>
    <script src="https://cdn.jsdelivr.net/npm/mermaid/dist/mermaid.min.js"></script>
    <script>mermaid.initialize({{ startOnLoad: true, theme: 'dark' }});</script>
    <style>
        body {{
            background: #0d1117;
            color: #c9d1d9;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            margin: 0;
            padding: 24px;
        }}
        .header {{
            border-bottom: 1px solid #30363d;
            padding-bottom: 16px;
            margin-bottom: 24px;
        }}
        .badge {{
            background: #6e40c9;
            color: #ffffff;
            padding: 4px 8px;
            border-radius: 4px;
            font-size: 12px;
            font-weight: bold;
        }}
        .card {{
            background: #161b22;
            border: 1px solid #30363d;
            border-radius: 6px;
            padding: 24px;
            box-shadow: 0 8px 24px rgba(0,0,0,0.5);
        }}
    </style>
</head>
<body>
    <div class="header">
        <h1>ORDYN Visual Architecture Explorer <span class="badge">v0.1</span></h1>
        <p>Interactive Security Boundary & Component DAG for <strong>{title}</strong></p>
    </div>
    <div class="card">
        <div class="mermaid">
{mermaid_chart}
        </div>
    </div>
</body>
</html>"#
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ordyn_parser::Parser;

    #[test]
    fn test_architecture_graph_generation() {
        let source = r#"
            capability NetConnect { host: String; port: Int; }
            entity Server { mut active: Bool; }
            boundary Gateway {
                grant NetConnect { host: "127.0.0.1", port: 80 };
                quota { memory_limit: 32; fuel_limit: 1000; }
                bind Server;
            }
        "#;

        let mut parser = Parser::from_source(source).unwrap();
        let program = parser.parse_program().unwrap();

        let graph = ArchitectureGraph::from_program(&program);
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);

        let mermaid = graph.to_mermaid();
        assert!(mermaid.contains("boundary_Gateway"));
        assert!(mermaid.contains("cap_NetConnect"));
        assert!(mermaid.contains("entity_Server"));

        let html = graph.to_interactive_html("GatewayApp");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("mermaid"));
    }
}
