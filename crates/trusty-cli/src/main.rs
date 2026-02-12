use anyhow::Result;
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use trusty_compiler;

#[derive(Parser)]
#[command(name = "trusty")]
#[command(about = "TRUST Language Compiler", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    input: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    Build {
        input: PathBuf,

        #[arg(short, long)]
        output: Option<PathBuf>,

        #[arg(short, long)]
        compile: bool,

        #[arg(short, long)]
        release: bool,
    },

    Run {
        input: PathBuf,

        #[arg(short, long)]
        release: bool,
    },

    Check {
        input: PathBuf,
    },

    Version,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Build {
            input,
            output,
            compile,
            release,
        }) => {
            build_file(input, output.as_ref(), *compile, *release)?;
        }
        Some(Commands::Run { input, release }) => {
            run_file(input, *release)?;
        }
        Some(Commands::Check { input }) => {
            check_file(input)?;
        }
        Some(Commands::Version) => {
            println!("trusty {}", env!("CARGO_PKG_VERSION"));
        }
        None => {
            if let Some(input) = &cli.input {
                // Default: build
                build_file(input, None, false, false)?;
            } else {
                println!("Usage: trusty <file.trs> or trusty --help");
            }
        }
    }

    Ok(())
}

fn build_file(
    input: &PathBuf,
    output: Option<&PathBuf>,
    compile: bool,
    release: bool,
) -> Result<()> {
    println!("🔨 Building {}...", input.display());

    let source = fs::read_to_string(input)?;

    let rust_code = trusty_compiler::compile(&source)?;

    let output_path = output
        .cloned()
        .unwrap_or_else(|| PathBuf::from("output.rs"));

    fs::write(&output_path, &rust_code)?;
    println!("✅ Generated {}", output_path.display());

    if compile {
        compile_rust(&output_path, release)?;
    }

    Ok(())
}

fn run_file(input: &PathBuf, release: bool) -> Result<()> {
    println!("🚀 Running {}...", input.display());

    build_file(input, None, true, release)?;

    let binary = if release {
        "./output_release"
    } else {
        "./output"
    };
    std::process::Command::new(binary).spawn()?.wait()?;

    Ok(())
}

fn check_file(input: &PathBuf) -> Result<()> {
    println!("🔍 Checking {}...", input.display());

    let source = fs::read_to_string(input)?;
    let _ = trusty_compiler::compile(&source)?;

    println!("✅ No errors found");
    Ok(())
}

fn compile_rust(rust_file: &PathBuf, release: bool) -> Result<()> {
    println!("🦀 Compiling Rust code...");

    let mut cmd = std::process::Command::new("rustc");
    cmd.arg(rust_file);

    if release {
        cmd.arg("-C").arg("opt-level=3");
        cmd.arg("-o").arg("output_release");
    } else {
        cmd.arg("-o").arg("output");
    }

    let output = cmd.output()?;

    if output.status.success() {
        println!("✅ Compilation successful");
    } else {
        eprintln!("❌ Compilation failed:");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}
