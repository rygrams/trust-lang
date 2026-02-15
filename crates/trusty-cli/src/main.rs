use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
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
    /// Create a new TRUST project
    New {
        name: String,
    },

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
        Some(Commands::New { name }) => {
            create_project(name)?;
        }
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
                build_file(input, None, false, false)?;
            } else {
                println!("Usage: trusty <file.trs> or trusty --help");
            }
        }
    }

    Ok(())
}

// ─── trusty new ──────────────────────────────────────────────────────────────

fn create_project(name: &str) -> Result<()> {
    let root = PathBuf::from(name);
    if root.exists() {
        bail!("Directory '{}' already exists", name);
    }

    fs::create_dir_all(root.join("src"))?;

    let manifest = serde_json::json!({
        "name": name,
        "version": "0.1.0",
        "dependencies": {}
    });
    fs::write(
        root.join("trusty.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;

    fs::write(
        root.join("src").join("main.trs"),
        "function main() {\n    console.write(\"Hello from TRUST!\");\n}\n",
    )?;

    fs::write(root.join(".gitignore"), "build/\n")?;

    println!("✅ Created project '{}'", name);
    println!("   cd {} && trusty run src/main.trs", name);

    Ok(())
}

// ─── build helpers ───────────────────────────────────────────────────────────

/// Returns the project-level `build/` directory (next to `src/`) when `trusty.json` exists.
/// Falls back to a local `build/` next to the input file otherwise.
fn build_dir(input: &Path) -> Result<PathBuf> {
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let dir = find_manifest(parent)
        .and_then(|manifest| manifest.parent().map(|p| p.join("build")))
        .unwrap_or_else(|| parent.join("build"));
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create build directory: {}", dir.display()))?;
    Ok(dir)
}

/// Stem of the input file (e.g. `hello` from `hello.trs`).
fn stem(input: &Path) -> String {
    input
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// Walk up from `start` looking for `trusty.json`. Returns its path if found.
fn find_manifest(start: &Path) -> Option<PathBuf> {
    let mut dir = start.canonicalize().ok()?;
    loop {
        let candidate = dir.join("trusty.json");
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Read `dependencies` map from `trusty.json`.
fn read_dependencies(manifest_path: &Path) -> Result<HashMap<String, String>> {
    let text = fs::read_to_string(manifest_path)?;
    let json: Value = serde_json::from_str(&text)?;
    let mut deps = HashMap::new();
    if let Some(obj) = json.get("dependencies").and_then(|v| v.as_object()) {
        for (k, v) in obj {
            let version = v.as_str().unwrap_or("*").to_string();
            deps.insert(k.clone(), version);
        }
    }
    Ok(deps)
}

// ─── trusty build ────────────────────────────────────────────────────────────

fn build_file(
    input: &PathBuf,
    output: Option<&PathBuf>,
    compile: bool,
    release: bool,
) -> Result<PathBuf> {
    println!("🔨 Building {}...", input.display());

    let source = resolve_and_bundle_modules(input)?;

    let transpile_output = trusty_compiler::compile_full(&source)?;

    let build = build_dir(input)?;
    let stem = stem(input);

    // Always write the intermediate .rs into build/
    let rs_path = build.join(format!("{}.rs", stem));
    fs::write(&rs_path, &transpile_output.rust_code)?;

    if compile {
        let bin_path = output
            .cloned()
            .unwrap_or_else(|| build.join(&stem));

        if transpile_output.required_crates.is_empty() {
            // No external crates → fast rustc path
            compile_with_rustc(&rs_path, &bin_path, release)?;
        } else {
            // External crates → generate a Cargo project and use cargo build
            compile_with_cargo(input, &transpile_output.rust_code, &transpile_output.required_crates, &bin_path, release)?;
        }

        fs::remove_file(&rs_path).ok();
        Ok(bin_path)
    } else {
        let final_path = output.cloned().unwrap_or(rs_path);
        println!("✅ Generated {}", final_path.display());
        Ok(final_path)
    }
}

// ─── rustc (no external deps) ────────────────────────────────────────────────

fn compile_with_rustc(rs_file: &Path, bin_path: &Path, release: bool) -> Result<()> {
    println!("🦀 Compiling with rustc...");

    let mut cmd = std::process::Command::new("rustc");
    cmd.arg(rs_file);
    cmd.arg("-o").arg(bin_path);
    if release {
        cmd.arg("-C").arg("opt-level=3");
    }

    let out = cmd.output()?;
    if out.status.success() {
        println!("✅ Binary ready: {}", bin_path.display());
    } else {
        eprintln!("❌ Compilation failed:");
        eprintln!("{}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(())
}

// ─── cargo (with external deps) ──────────────────────────────────────────────

fn compile_with_cargo(
    input: &Path,
    rust_code: &str,
    required_crates: &[String],
    bin_path: &Path,
    release: bool,
) -> Result<()> {
    println!("📦 External crates detected, building with cargo...");

    // Resolve dependency versions from trusty.json (if present)
    let manifest_deps = input
        .parent()
        .and_then(|p| find_manifest(p))
        .map(|m| read_dependencies(&m).unwrap_or_default())
        .unwrap_or_default();

    let build = build_dir(input)?;
    let stem = stem(input);
    let cargo_project = build.join(format!("{}_cargo", stem));

    fs::create_dir_all(cargo_project.join("src"))?;

    // Generate Cargo.toml
    let mut deps_toml = String::new();
    for crate_name in required_crates {
        let version = manifest_deps.get(crate_name).map(String::as_str).unwrap_or("*");
        deps_toml.push_str(&format!("{} = \"{}\"\n", crate_name, version));
    }

    let cargo_toml = format!(
        "[package]\nname = \"{stem}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n{deps_toml}\n[workspace]\n"
    );
    fs::write(cargo_project.join("Cargo.toml"), &cargo_toml)?;

    // Write generated Rust source
    fs::write(cargo_project.join("src").join("main.rs"), rust_code)?;

    // cargo build
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("build");
    cmd.arg("--manifest-path").arg(cargo_project.join("Cargo.toml"));
    if release {
        cmd.arg("--release");
    }

    let out = cmd.output()?;
    if !out.status.success() {
        eprintln!("❌ Compilation failed:");
        eprintln!("{}", String::from_utf8_lossy(&out.stderr));
        return Ok(());
    }

    // Copy binary to the expected bin_path
    let profile = if release { "release" } else { "debug" };
    let cargo_bin = cargo_project
        .join("target")
        .join(profile)
        .join(&stem);

    fs::copy(&cargo_bin, bin_path)
        .with_context(|| format!("Failed to copy binary from {}", cargo_bin.display()))?;

    println!("✅ Binary ready: {}", bin_path.display());
    Ok(())
}

// ─── trusty run ──────────────────────────────────────────────────────────────

fn run_file(input: &PathBuf, release: bool) -> Result<()> {
    println!("🚀 Running {}...", input.display());

    let bin_path = build_file(input, None, true, release)?;

    std::process::Command::new(&bin_path)
        .spawn()
        .with_context(|| format!("Failed to run {}", bin_path.display()))?
        .wait()?;

    Ok(())
}

// ─── trusty check ────────────────────────────────────────────────────────────

fn check_file(input: &PathBuf) -> Result<()> {
    println!("🔍 Checking {}...", input.display());

    let source = resolve_and_bundle_modules(input)?;
    let _ = trusty_compiler::compile(&source)?;

    println!("✅ No errors found");
    Ok(())
}

// ─── local module resolver (TRUST files) ────────────────────────────────────

fn resolve_and_bundle_modules(entry: &Path) -> Result<String> {
    let entry = entry
        .canonicalize()
        .with_context(|| format!("Failed to resolve {}", entry.display()))?;

    let mut seen = HashSet::new();
    let mut stack = Vec::new();
    resolve_module_file(&entry, &mut seen, &mut stack)
}

fn resolve_module_file(
    file: &Path,
    seen: &mut HashSet<PathBuf>,
    stack: &mut Vec<PathBuf>,
) -> Result<String> {
    let canonical = file
        .canonicalize()
        .with_context(|| format!("Failed to resolve {}", file.display()))?;

    if seen.contains(&canonical) {
        return Ok(String::new());
    }

    if stack.contains(&canonical) {
        let chain = stack
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        bail!("Circular local import detected: {} -> {}", chain, canonical.display());
    }
    stack.push(canonical.clone());

    let source = fs::read_to_string(&canonical)
        .with_context(|| format!("Failed to read {}", canonical.display()))?;

    let mut dep_code = String::new();
    let mut body_lines = Vec::new();
    let base_dir = canonical.parent().unwrap_or_else(|| Path::new("."));

    for line in source.lines() {
        if let Some(import_path) = parse_local_import_path(line) {
            let dep_file = resolve_local_import_target(base_dir, &import_path)?;
            let child = resolve_module_file(&dep_file, seen, stack)?;
            if !child.trim().is_empty() {
                dep_code.push_str(&child);
                if !dep_code.ends_with('\n') {
                    dep_code.push('\n');
                }
            }
            continue;
        }
        body_lines.push(line.to_string());
    }

    let body = body_lines.join("\n");
    let rewritten = rewrite_export_declarations(&body)
        .with_context(|| format!("In module {}", canonical.display()))?;

    stack.pop();
    seen.insert(canonical.clone());

    let mut out = String::new();
    out.push_str(&dep_code);
    out.push_str(&format!("// --- module: {} ---\n", canonical.display()));
    out.push_str(&rewritten);
    out.push('\n');
    Ok(out)
}

fn parse_local_import_path(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with("import ") {
        return None;
    }
    let from_idx = trimmed.find(" from ")?;
    let after_from = trimmed[from_idx + " from ".len()..].trim();
    let quote = after_from.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &after_from[1..];
    let end = rest.find(quote)?;
    let path = &rest[..end];
    if path.starts_with("./") || path.starts_with("../") {
        Some(path.to_string())
    } else {
        None
    }
}

fn resolve_local_import_target(base_dir: &Path, import_path: &str) -> Result<PathBuf> {
    let candidate = base_dir.join(import_path);
    let mut tries = Vec::new();

    tries.push(candidate.clone());
    if candidate.extension().is_none() {
        tries.push(candidate.with_extension("trs"));
        tries.push(candidate.join("index.trs"));
    }

    for t in tries {
        if t.exists() {
            return Ok(t);
        }
    }

    bail!(
        "Cannot resolve local import '{}' from {}",
        import_path,
        base_dir.display()
    )
}

fn rewrite_export_declarations(source: &str) -> Result<String> {
    let mut out = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("export ") {
            let allowed = rest.starts_with("function ")
                || rest.starts_with("const ")
                || rest.starts_with("struct ")
                || rest.starts_with("enum ")
                || rest.starts_with("implements ");
            if !allowed {
                bail!(
                    "Unsupported export syntax: '{}'. Supported: export function/const/struct/enum/implements",
                    trimmed
                );
            }
            let indent = &line[..line.len() - trimmed.len()];
            out.push(format!("{}{}", indent, rest));
        } else {
            out.push(line.to_string());
        }
    }
    Ok(out.join("\n"))
}
