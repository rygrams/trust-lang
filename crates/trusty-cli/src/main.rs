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

    /// Format a TRUST source file
    Format {
        input: PathBuf,

        /// Check formatting without writing changes
        #[arg(long)]
        check: bool,
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
        Some(Commands::Format { input, check }) => {
            format_file(input, *check)?;
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

// ─── trusty format ───────────────────────────────────────────────────────────

fn format_file(input: &PathBuf, check: bool) -> Result<()> {
    let source = fs::read_to_string(input)
        .with_context(|| format!("Failed to read {}", input.display()))?;
    let formatted = format_trust_source(&source);

    if check {
        if source == formatted {
            println!("✅ Already formatted: {}", input.display());
            return Ok(());
        }
        bail!("❌ Needs formatting: {}", input.display());
    }

    if source != formatted {
        fs::write(input, formatted)
            .with_context(|| format!("Failed to write {}", input.display()))?;
        println!("✅ Formatted {}", input.display());
    } else {
        println!("✅ Already formatted: {}", input.display());
    }

    Ok(())
}

fn format_trust_source(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0usize;
    let mut out = String::with_capacity(source.len() + source.len() / 8);
    let mut indent = 0usize;
    let mut paren_depth = 0usize;
    let mut at_line_start = true;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_template = false;
    let mut prev_input_was_newline = false;

    fn push_indent(out: &mut String, indent: usize) {
        for _ in 0..indent {
            out.push_str("    ");
        }
    }

    fn trim_trailing_spaces(out: &mut String) {
        while out.ends_with(' ') || out.ends_with('\t') {
            out.pop();
        }
    }

    fn ensure_line(out: &mut String, at_line_start: &mut bool, indent: usize) {
        if *at_line_start {
            push_indent(out, indent);
            *at_line_start = false;
        }
    }

    while i < chars.len() {
        let c = chars[i];
        let next = if i + 1 < chars.len() { Some(chars[i + 1]) } else { None };

        if in_single {
            prev_input_was_newline = false;
            ensure_line(&mut out, &mut at_line_start, indent);
            out.push(c);
            if c == '\\' && next.is_some() {
                i += 1;
                out.push(chars[i]);
            } else if c == '\'' {
                in_single = false;
            }
            i += 1;
            continue;
        }

        if in_double {
            prev_input_was_newline = false;
            ensure_line(&mut out, &mut at_line_start, indent);
            out.push(c);
            if c == '\\' && next.is_some() {
                i += 1;
                out.push(chars[i]);
            } else if c == '"' {
                in_double = false;
            }
            i += 1;
            continue;
        }

        if in_template {
            prev_input_was_newline = false;
            ensure_line(&mut out, &mut at_line_start, indent);
            out.push(c);
            if c == '\\' && next.is_some() {
                i += 1;
                out.push(chars[i]);
            } else if c == '`' {
                in_template = false;
            }
            i += 1;
            continue;
        }

        if c == '/' && next == Some('/') {
            prev_input_was_newline = false;
            ensure_line(&mut out, &mut at_line_start, indent);
            out.push('/');
            out.push('/');
            i += 2;
            while i < chars.len() && chars[i] != '\n' {
                out.push(chars[i]);
                i += 1;
            }
            trim_trailing_spaces(&mut out);
            out.push('\n');
            at_line_start = true;
            continue;
        }

        if c == '/' && next == Some('*') {
            prev_input_was_newline = false;
            ensure_line(&mut out, &mut at_line_start, indent);
            out.push('/');
            out.push('*');
            i += 2;
            while i < chars.len() {
                out.push(chars[i]);
                if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] == '/' {
                    i += 1;
                    out.push('/');
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        match c {
            '\'' => {
                ensure_line(&mut out, &mut at_line_start, indent);
                out.push(c);
                in_single = true;
            }
            '"' => {
                ensure_line(&mut out, &mut at_line_start, indent);
                out.push(c);
                in_double = true;
            }
            '`' => {
                ensure_line(&mut out, &mut at_line_start, indent);
                out.push(c);
                in_template = true;
            }
            '(' => {
                ensure_line(&mut out, &mut at_line_start, indent);
                out.push('(');
                paren_depth += 1;
            }
            ')' => {
                ensure_line(&mut out, &mut at_line_start, indent);
                out.push(')');
                paren_depth = paren_depth.saturating_sub(1);
            }
            '{' => {
                trim_trailing_spaces(&mut out);
                if !at_line_start && !out.ends_with('\n') {
                    out.push(' ');
                }
                out.push('{');
                out.push('\n');
                indent += 1;
                at_line_start = true;
            }
            '}' => {
                trim_trailing_spaces(&mut out);
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                indent = indent.saturating_sub(1);
                push_indent(&mut out, indent);
                out.push('}');
                at_line_start = false;
            }
            ';' => {
                ensure_line(&mut out, &mut at_line_start, indent);
                out.push(';');
                if paren_depth == 0 {
                    trim_trailing_spaces(&mut out);
                    out.push('\n');
                    at_line_start = true;
                }
            }
            ',' => {
                ensure_line(&mut out, &mut at_line_start, indent);
                out.push(',');
                if next.is_some() && next != Some('\n') && next != Some(' ') {
                    out.push(' ');
                }
            }
            '\n' | '\r' => {
                trim_trailing_spaces(&mut out);
                if prev_input_was_newline {
                    if !out.ends_with("\n\n") {
                        out.push('\n');
                    }
                } else if !out.ends_with('\n') {
                    out.push('\n');
                }
                at_line_start = true;
                prev_input_was_newline = true;
            }
            _ => {
                prev_input_was_newline = false;
                match c {
                    ' ' | '\t' => {
                        if !at_line_start && !out.ends_with(' ') && !out.ends_with('\n') {
                            out.push(' ');
                        }
                    }
                    _ => {
                        ensure_line(&mut out, &mut at_line_start, indent);
                        out.push(c);
                    }
                }
            }
        }

        i += 1;
    }

    let mut formatted = out
        .lines()
        .map(|line| line.trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    formatted = reflow_named_imports(&formatted, 85);
    if !formatted.ends_with('\n') {
        formatted.push('\n');
    }
    formatted
}

fn reflow_named_imports(source: &str, print_width: usize) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0usize;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.starts_with("import {") {
            let mut decl = trimmed.to_string();
            let mut j = i;
            while !decl.contains("} from ") && j + 1 < lines.len() {
                j += 1;
                decl.push(' ');
                decl.push_str(lines[j].trim());
            }

            if let Some(reflowed) = reflow_named_import_decl(&decl, print_width) {
                out.extend(reflowed);
                i = j + 1;
                continue;
            }
        }

        out.push(lines[i].to_string());
        i += 1;
    }

    out.join("\n")
}

fn reflow_named_import_decl(decl: &str, print_width: usize) -> Option<Vec<String>> {
    let trimmed = decl.trim();
    if !trimmed.starts_with("import {") {
        return None;
    }
    let from_marker = "} from ";
    let from_pos = trimmed.find(from_marker)?;
    let inner = trimmed["import {".len()..from_pos].trim();
    let tail = format!("}}{}", &trimmed[from_pos + 1..]);

    let names: Vec<String> = inner
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    if names.is_empty() {
        return None;
    }

    let one_line = format!("import {{ {} {}", names.join(", "), tail);
    if one_line.len() <= print_width {
        return Some(vec![one_line]);
    }

    let mut out = Vec::with_capacity(names.len() + 2);
    out.push("import {".to_string());
    for name in names {
        out.push(format!("    {},", name));
    }
    out.push(format!("}}{}", &trimmed[from_pos + 1..]));
    Some(out)
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

#[cfg(test)]
mod tests {
    use super::format_trust_source;

    #[test]
    fn test_format_trust_source_basic() {
        let src = "function main(){let x=1; if(x>0){console.write(\"ok\");}}\n";
        let got = format_trust_source(src);
        assert!(got.contains("function main() {"));
        assert!(got.contains("let x=1;"));
        assert!(got.contains("if(x>0) {"));
        assert!(got.contains("console.write(\"ok\");"));
    }

    #[test]
    fn test_format_trust_source_keeps_for_header() {
        let src = "function main(){for (var i = 0; i < 10; i = i + 1){console.write(i);}}\n";
        let got = format_trust_source(src);
        assert!(got.contains("for (var i = 0; i < 10; i = i + 1) {"));
    }

    #[test]
    fn test_format_trust_source_keeps_single_blank_line() {
        let src = "function main(){\n\n\nconsole.write(\"a\");\n\n\nconsole.write(\"b\");\n}\n";
        let got = format_trust_source(src);
        assert!(got.contains("\n\n    console.write(\"a\");"));
        assert!(got.contains("console.write(\"a\");\n\n    console.write(\"b\");"));
        assert!(!got.contains("\n\n\n"));
    }

    #[test]
    fn test_format_trust_source_wraps_long_named_imports() {
        let src = "import { Instant, Duration, Date, Time, DateTime, sleep, compare, addDays, addMonths, addYears, subMinutes, subMonths, subYears } from \"trusty:time\";\n";
        let got = format_trust_source(src);
        assert!(got.contains("import {\n"));
        assert!(got.contains("    Instant,\n"));
        assert!(got.contains("} from \"trusty:time\";"));
    }
}
