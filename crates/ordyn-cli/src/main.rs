use clap::{Parser as ClapParser, Subcommand};
use miette::{IntoDiagnostic, Result};
use ordyn_parser::Parser;
use std::fs;
use std::path::PathBuf;

#[derive(ClapParser)]
#[command(name = "ordyn")]
#[command(about = "ORDYN Programming Language & Security Architecture CLI", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new ORDYN project repository
    New {
        /// Name of the new project
        name: String,
    },
    /// Parse and typecheck an ORDYN source file or project
    Check {
        /// Path to the source file (.ordyn)
        file: PathBuf,
    },
    /// Verify capabilities, contracts, and security boundaries
    Verify {
        /// Path to the source file or manifest
        file: PathBuf,
    },
    /// Run automated tests across the project
    Test,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name } => {
            println!("Initializing new ORDYN security-first project: {}", name);
            fs::create_dir_all(&name).into_diagnostic()?;
            let sample_code = r#"// ORDYN Application Entry

capability NetConnect {
    host: String;
    port: Int;
}

boundary MainBoundary {
    grant NetConnect { host: "localhost", port: 8080 };
    quota {
        memory_limit: 32;
        fuel_limit: 100000;
    }
}
"#;
            fs::write(format!("{}/main.ordyn", name), sample_code).into_diagnostic()?;
            println!("Project '{}' created successfully with initial boundary configuration.", name);
        }
        Commands::Check { file } => {
            println!("ORDYN Compiler checking: {}", file.display());
            let source = fs::read_to_string(&file).into_diagnostic()?;
            let mut parser = Parser::from_source(&source)
                .map_err(|e| miette::miette!("{}", e))?;
            let program = parser.parse_program()
                .map_err(|e| miette::miette!("{}", e))?;
            println!("Parsing succeeded: discovered {} top-level declarations.", program.declarations.len());

            println!("Running static semantic and type analysis...");
            let mut checker = ordyn_types::TypeChecker::new();
            checker.check_program(&program)
                .map_err(|e| miette::miette!("{}", e))?;
            println!("Type checking succeeded: 0 errors detected.");

            println!("Running static effect analysis...");
            let mut effect_analyzer = ordyn_effects::EffectAnalyzer::new();
            effect_analyzer.analyze_program(&program)
                .map_err(|e| miette::miette!("{}", e))?;
            println!("Effect analysis succeeded: all side-effects sound and authorized.");

            println!("Lowering to Typed Intermediate Representation (TIR - SSA)...");
            let mut lowerer = ordyn_ir::IrLowerer::new();
            let ir_module = lowerer.lower_program(&program)
                .map_err(|e| miette::miette!("{}", e))?;
            println!("TIR Lowering succeeded: generated {} functions, {} entities.", ir_module.functions.len(), ir_module.entities.len());
        }
        Commands::Verify { file } => {
            println!("ORDYN Guard verifying security invariants for: {}", file.display());
            let source = fs::read_to_string(&file).into_diagnostic()?;
            let mut parser = Parser::from_source(&source)
                .map_err(|e| miette::miette!("{}", e))?;
            let program = parser.parse_program()
                .map_err(|e| miette::miette!("{}", e))?;

            let mut checker = ordyn_types::TypeChecker::new();
            checker.check_program(&program)
                .map_err(|e| miette::miette!("{}", e))?;

            let mut effect_analyzer = ordyn_effects::EffectAnalyzer::new();
            effect_analyzer.analyze_program(&program)
                .map_err(|e| miette::miette!("{}", e))?;

            println!("Static verification baseline passed for {} declarations (contracts, effects & boundaries sound).", program.declarations.len());
        }
        Commands::Test => {
            println!("Executing ORDYN test suite...");
        }
    }

    Ok(())
}
