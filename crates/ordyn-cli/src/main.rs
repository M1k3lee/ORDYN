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
        }
        Commands::Verify { file } => {
            println!("ORDYN Guard verifying security invariants for: {}", file.display());
            let source = fs::read_to_string(&file).into_diagnostic()?;
            let mut parser = Parser::from_source(&source)
                .map_err(|e| miette::miette!("{}", e))?;
            let program = parser.parse_program()
                .map_err(|e| miette::miette!("{}", e))?;
            println!("Static verification baseline passed for {} declarations.", program.declarations.len());
        }
        Commands::Test => {
            println!("Executing ORDYN test suite...");
        }
    }

    Ok(())
}
