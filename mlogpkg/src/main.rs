// ── mlogpkg: METALOGOS Package Manager (Phase 3.4) ───────────────────────
//
// `mlogpkg init`              — creates mlog.toml (name, version, dependencies)
// `mlogpkg add <pkg>`          — adds a dependency to mlog.toml
// `mlogpkg build`              — resolves dependencies, collects sources, runs check
//
// Local registry: packages stored in ~/.mlog/registry/<pkg-name>/
// No remote server at this phase.

use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "mlogpkg",
    about = "METALOGOS package manager",
    version,
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Initialize a new METALOGOS project (creates mlog.toml)
    Init {
        /// Project name (defaults to directory name)
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Add a dependency to mlog.toml
    Add {
        /// Package name to add as dependency
        pkg: String,
        /// Optional version constraint (e.g. "0.3.0")
        version: Option<String>,
    },
    /// Resolve dependencies and build the project
    Build,
    /// Show current project info
    Info,
}

/// mlog.toml manifest structure.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Manifest {
    package: Package,
    #[serde(default)]
    dependencies: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Package {
    name: String,
    version: String,
    #[serde(default)]
    edition: String,
}

impl Default for Package {
    fn default() -> Self {
        Self {
            name: "untitled".to_string(),
            version: "0.1.0".to_string(),
            edition: "2024".to_string(),
        }
    }
}

/// Lock file structure: resolved dependency versions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LockFile {
    #[serde(default)]
    packages: HashMap<String, LockedPkg>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LockedPkg {
    version: String,
    source: String,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => cmd_init(name),
        Commands::Add { pkg, version } => cmd_add(&pkg, version.as_deref()),
        Commands::Build => cmd_build(),
        Commands::Info => cmd_info(),
    }
}

/// `mlogpkg init [name]` — create mlog.toml in current directory.
fn cmd_init(name: Option<String>) {
    let manifest_path = PathBuf::from("mlog.toml");
    if manifest_path.exists() {
        eprintln!("error: mlog.toml already exists");
        std::process::exit(1);
    }

    let project_name = name.unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "untitled".to_string())
    });

    let manifest = Manifest {
        package: Package {
            name: project_name,
            version: "0.1.0".to_string(),
            edition: "2024".to_string(),
        },
        dependencies: HashMap::new(),
    };

    let toml_str = toml::to_string_pretty(&manifest).expect("failed to serialize manifest");
    fs::write(&manifest_path, &toml_str).expect("failed to write mlog.toml");

    // Also create src/ and a minimal entry.mlog
    fs::create_dir_all("src").ok();
    let entry = "entity greeting: String = \"Hello from mlogpkg!\"\nflow Main { input: String = greeting -> output }\n";
    fs::write("src/main.mlog", entry).ok();

    println!("Created mlog.toml for project");
    println!("  name: {}", manifest.package.name);
    println!("  version: {}", manifest.package.version);
    println!("  entry: src/main.mlog");
}

/// `mlogpkg add <pkg> [version]` — add dependency to mlog.toml.
fn cmd_add(pkg: &str, version: Option<&str>) {
    let manifest_path = PathBuf::from("mlog.toml");
    if !manifest_path.exists() {
        eprintln!("error: mlog.toml not found. Run 'mlogpkg init' first.");
        std::process::exit(1);
    }

    // Check if package exists in local registry
    let registry_dir = registry_dir();
    let pkg_dir = registry_dir.join(pkg);
    if !pkg_dir.exists() {
        eprintln!(
            "error: package '{}' not found in local registry ({})",
            pkg,
            registry_dir.display()
        );
        eprintln!("  Install packages manually to: {}", registry_dir.display());
        std::process::exit(1);
    }

    // Read the package's manifest to get its version
    let pkg_manifest_path = pkg_dir.join("mlog.toml");
    let resolved_version = match version {
        Some(v) => v.to_string(),
        None => {
            let pkg_manifest = read_manifest(&pkg_manifest_path);
            pkg_manifest.package.version
        }
    };

    let mut manifest = read_manifest(&manifest_path);
    let prev = manifest
        .dependencies
        .insert(pkg.to_string(), resolved_version);
    if prev.is_some() {
        println!("Updated dependency: {} (was {})", pkg, prev.unwrap());
    } else {
        println!("Added dependency: {} = {}", pkg, manifest.dependencies[pkg]);
    }

    write_manifest(&manifest_path, &manifest);
    println!("Dependencies:");
    for (dep_name, dep_ver) in &manifest.dependencies {
        println!("  {} = {}", dep_name, dep_ver);
    }
}

/// `mlogpkg build` — resolve dependencies, collect sources, run semantic check.
fn cmd_build() {
    let manifest_path = PathBuf::from("mlog.toml");
    if !manifest_path.exists() {
        eprintln!("error: mlog.toml not found. Run 'mlogpkg init' first.");
        std::process::exit(1);
    }

    let manifest = read_manifest(&manifest_path);
    println!(
        "Building {} v{}",
        manifest.package.name, manifest.package.version
    );

    // 1. Find entry point
    let entry = find_entry_point();
    println!("  entry: {}", entry.display());

    // 2. Resolve dependencies (collect import paths)
    let resolved = resolve_dependencies(&manifest);
    println!("  dependencies resolved: {}", resolved.len());
    for (name, ver) in &resolved {
        println!("    {} = {}", name, ver);
    }

    // 3. Collect all source files
    let mut all_sources = Vec::new();
    collect_sources(PathBuf::from("src"), &mut all_sources);
    println!("  source files: {}", all_sources.len());

    // 4. Semantic check each source file
    let mut errors = 0;
    for src in &all_sources {
        match fs::read_to_string(src) {
            Ok(source) => match metalogos::check_program(&source) {
                Ok(result) => {
                    if !result.is_ok() {
                        println!("  ERRORS in {}: {}", src.display(), result.format());
                        errors += result.error_count();
                    }
                }
                Err(e) => {
                    println!("  ERROR in {}: {}", src.display(), e);
                    errors += 1;
                }
            },
            Err(e) => {
                println!("  ERROR: cannot read {}: {}", src.display(), e);
                errors += 1;
            }
        }
    }

    // 5. Write lock file
    let lock = LockFile {
        packages: resolved
            .into_iter()
            .map(|(name, ver)| {
                (
                    name.clone(),
                    LockedPkg {
                        version: ver,
                        source: format!("registry:{}", name),
                    },
                )
            })
            .collect(),
    };
    let lock_str = serde_json::to_string_pretty(&lock).unwrap();
    fs::write("mlog.lock", &lock_str).ok();

    if errors == 0 {
        println!(
            "Build OK: {} source files checked, 0 errors.",
            all_sources.len()
        );
    } else {
        println!("Build FAILED: {} error(s).", errors);
        std::process::exit(1);
    }
}

/// `mlogpkg info` — show project info.
fn cmd_info() {
    let manifest_path = PathBuf::from("mlog.toml");
    if !manifest_path.exists() {
        println!("No mlog.toml found. Run 'mlogpkg init' first.");
        return;
    }
    let manifest = read_manifest(&manifest_path);
    println!(
        "Project: {} v{}",
        manifest.package.name, manifest.package.version
    );
    println!("Edition: {}", manifest.package.edition);
    if manifest.dependencies.is_empty() {
        println!("Dependencies: (none)");
    } else {
        println!("Dependencies:");
        for (name, ver) in &manifest.dependencies {
            println!("  {} = {}", name, ver);
        }
    }
    let entry = find_entry_point();
    println!("Entry: {}", entry.display());
    let registry = registry_dir();
    println!("Registry: {}", registry.display());
}

// ── Helper functions ────────────────────────────────────────────────────

fn registry_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".mlog")
        .join("registry")
}

fn read_manifest(path: &Path) -> Manifest {
    let content = fs::read_to_string(path).expect(&format!("failed to read {:?}", path));
    toml::from_str(&content).expect(&format!("failed to parse {:?}", path))
}

fn write_manifest(path: &Path, manifest: &Manifest) {
    let toml_str = toml::to_string_pretty(manifest).expect("failed to serialize manifest");
    fs::write(path, &toml_str).expect(&format!("failed to write {:?}", path));
}

fn find_entry_point() -> PathBuf {
    // Look for src/main.mlog first, then src/main.mlog
    let candidates = [PathBuf::from("src/main.mlog"), PathBuf::from("main.mlog")];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    PathBuf::from("src/main.mlog") // default even if not yet created
}

fn resolve_dependencies(manifest: &Manifest) -> HashMap<String, String> {
    let mut resolved = HashMap::new();
    let registry = registry_dir();

    for (name, version_req) in &manifest.dependencies {
        let pkg_dir = registry.join(name);
        if pkg_dir.exists() {
            let pkg_manifest = match read_manifest_optional(&pkg_dir.join("mlog.toml")) {
                Some(m) => m,
                None => {
                    eprintln!("warning: dependency '{}' has no mlog.toml, skipping", name);
                    continue;
                }
            };
            resolved.insert(name.clone(), pkg_manifest.package.version);
        } else {
            eprintln!("warning: dependency '{}' not found in registry", name);
            resolved.insert(name.clone(), version_req.clone());
        }
    }

    resolved
}

fn read_manifest_optional(path: &Path) -> Option<Manifest> {
    let content = fs::read_to_string(path).ok()?;
    toml::from_str(&content).ok()
}

fn collect_sources(dir: PathBuf, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_sources(path, files);
            } else if path.extension().map(|e| e == "mlog").unwrap_or(false) {
                files.push(path);
            }
        }
    }
}
