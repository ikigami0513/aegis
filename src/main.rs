use aegis_core::{compiler, loader, native, package_manager, plugins};
use clap::{Parser, Subcommand};
use rustyline::DefaultEditor;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use serde_json::Value as JsonValue;
use std::path::Path;
use aegis_core::vm::VM;

#[derive(Parser)]
#[command(name = "aegis")]
#[command(about = "Aegis Language Compiler & Package Manager", version = "0.4.2", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>
}

#[derive(Subcommand)]
enum Commands {
    /// Exécute un script Aegis
    Run {
        /// Le chemin du fichier .aeg
        file: String,

        /// Affiche le bytecode généré avant l'exécution
        #[arg(long, short)]
        debug: bool,
        
        /// Arguments à passer au script (accessibles via System.args())
        /// Ils capturent tout ce qui se trouve après le nom du fichier ou "--"
        #[arg(last = true)]
        args: Vec<String>,
    },

    /// Lance le mode interactif (REPL)
    Repl,

    /// [APM] Installe un paquet depuis le registre
    Add {
        /// Nom du paquet (ex: "glfw")
        name: String,
        /// Version spécifique (optionnel)
        version: Option<String>,
    },

    /// [APM] Publie le paquet courant
    Publish {
        /// Cible OS spécifique (ex: linux, windows)
        #[arg(long)] 
        os: Option<String>,
        
        /// Architecture cible (ex: x86_64, arm64)
        #[arg(long)]
        arch: Option<String>
    },

    /// [APM] Se connecte au registre
    Login {
        token: String
    },
}

#[derive(Deserialize)]
struct ProjectConfig {
    dependencies: Option<HashMap<String, String>>
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct PackageManifest {
    package: PackageInfo,
    targets: Option<HashMap<String, String>>
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct PackageInfo {
    name: String,
    version: String,
}

// Charge les plugins natifs basés sur le fichier aegis.toml (Legacy support pour les DLLs locales)
fn load_config() {
    if let Ok(content) = fs::read_to_string("aegis.toml") {
        let config: ProjectConfig = toml::from_str(&content).unwrap_or_else(|_| ProjectConfig { dependencies: None });

        if let Some(deps) = config.dependencies {
            for (name, _version_req) in deps {
                let package_path = Path::new("packages").join(&name);

                if !package_path.exists() {
                    // On ne crie pas si le dossier n'existe pas, car ça peut être une dépendance pure source (.aeg)
                    // gérée par le compilateur via l'OpCode::Import
                    continue; 
                }

                // Si on trouve une librairie native, on la charge
                if let Ok(final_path) = resolve_library_path(&package_path) {
                    if let Err(e) = plugins::load_plugin(final_path.to_str().unwrap()) {
                        eprintln!("   ⚠️ Warning chargement plugin '{}': {}", name, e);
                    }
                }
            }
        }
    }
}

// Tente de trouver un .dll/.so dans le dossier du paquet
fn resolve_library_path(path: &Path) -> Result<std::path::PathBuf, String> {
    // 1. Essayer via le manifest (si présent)
    let manifest_path = path.join("aegis.toml");
    if manifest_path.exists() {
        if let Ok(content) = fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = toml::from_str::<PackageManifest>(&content) {
                let current_os = std::env::consts::OS;
                if let Some(targets) = manifest.targets {
                    if let Some(binary_file) = targets.get(current_os) {
                        return Ok(path.join(binary_file));
                    }
                }
            }
        }
    }
    
    // 2. Fallback : scan bourrin du dossier pour trouver un .so/.dll/.dylib
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Some(ext) = p.extension() {
                if ext == "dll" || ext == "so" || ext == "dylib" {
                    return Ok(p);
                }
            }
        }
    }

    Err("Aucun binaire trouvé".into())
}

fn main() -> Result<(), String> {
    native::init_registry();
    
    // On charge les plugins natifs AVANT de lancer la VM
    load_config();

    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Run { file, debug, args }) => {
            // On passe les args (clonés pour ownership) à run_file
            run_file(file, *debug, args.clone())
        }

        Some(Commands::Repl) | None => {
            println!("Aegis v2.0 - REPL");
            println!("Tapez 'exit' ou 'quit' pour quitter.");
            run_repl();
            Ok(())
        }

        Some(Commands::Add { name, version }) => {
            // package_manager::install attend &str et Option<String>
            package_manager::install(name, version.clone())
        }

        Some(Commands::Publish { os, arch }) => {
            // Il faut cloner les Options car `cli` est emprunté dans le match
            package_manager::publish(os.clone(), arch.clone())
        }

        Some(Commands::Login { token }) => {
            package_manager::login(token)
        },
    }
}

// Nouvelle implémentation utilisant la VM v2
fn run_file(filename: &str, debug: bool, args: Vec<String>) -> Result<(), String> {
    let content = fs::read_to_string(filename)
        .map_err(|e| format!("Impossible de lire {}: {}", filename, e))?;

    // 1. Frontend 
    let json_data: JsonValue = if filename.ends_with(".aeg") {
        compiler::compile(&content)?
    } else {
        serde_json::from_str(&content).map_err(|e| e.to_string())?
    };
    
    // 2. Loader
    let statements = loader::parse_block(&json_data)?;

    // 3. Compilation v2
    let compiler = aegis_core::vm::compiler::Compiler::new();
    let (chunk, global_names) = compiler.compile(statements);

    if debug {
        use aegis_core::vm::debug;
        println!("\n=== DEBUG: BYTECODE GENERATED ===");
        debug::disassemble_chunk(&chunk, filename);
        println!("=================================\n");
    }

    // 4. Nettoyage des arguments "--" si présents
    let mut script_args = Vec::new();
    for arg in args {
        if arg != "--" {
            script_args.push(arg);
        }
    }

    // 5. Exécution VM avec les arguments
    let mut vm = VM::new(chunk, global_names, script_args);
    
    vm.run()
}

fn run_repl() {
    let global_names = std::rc::Rc::new(std::cell::RefCell::new(HashMap::new()));
    let empty_chunk = aegis_core::chunk::Chunk::new();
    let mut vm = VM::new(empty_chunk, global_names.clone(), vec![]);

    let mut rl = DefaultEditor::new().unwrap();

    loop {
        let readline = rl.readline(">> ");

        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str()).unwrap();
                let source = line.trim();
                if source == "exit" || source == "quit" { break; }
                
                // Pipeline v2 pour REPL
                match compiler::compile(source) {
                    Ok(json_ast) => {
                        match loader::parse_block(&json_ast) {
                            Ok(statements) => {
                                // Important: préserver le contexte global
                                let mut repl_compiler = aegis_core::vm::compiler::Compiler::new_with_globals(global_names.clone());
                                repl_compiler.scope_depth = 0; 
                                
                                let (chunk, _) = repl_compiler.compile(statements);

                                if let Err(e) = vm.execute_chunk(chunk) {
                                    println!("Runtime Error: {}", e);
                                }
                            },
                            Err(e) => println!("Loader Error: {}", e)
                        }
                    },
                    Err(e) => println!("Syntax Error: {}", e)
                }
            }
            Err(error) => {
                println!("IO Error: {}", error);
                break;
            }
        }
    }
}
