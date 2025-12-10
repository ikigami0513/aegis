use aegis_core::{compiler, loader, native, plugins};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Write;
use std::{fs, io};
use serde_json::Value as JsonValue;
use std::path::Path;
// use std::time::Instant; // Décommenter si tu veux mesurer le temps d'exécution

mod package_manager;

use aegis_core::vm::VM;

#[derive(Parser)]
#[command(name = "aegis")]
#[command(about = "Aegis Language Compiler & Package Manager", version = "2.0", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>
}

#[derive(Subcommand)]
enum Commands {
    /// Exécute un script Aegis (Moteur v2)
    Run {
        /// Le chemin du fichier .aeg
        file: String,
        
        /// Arguments à passer au script (accessibles via System.args())
        /// Note: L'intégration des args dans la VM v2 reste à faire.
        #[arg(last = true)]
        args: Vec<String>,
    },

    /// Lance le mode interactif (REPL) - Toujours sur v1 pour l'instant
    Repl,

    /// [APM] Installe un paquet depuis le registre
    Add {
        /// Nom du paquet (ex: "glfw")
        name: String,
        /// Version spécifique (optionnel)
        version: Option<String>,
    },

    /// [APM] Publie le paquet courant
    Publish,

    /// [APM] Se connecte au registre
    Login {
        token: String
    },
    
    // La commande Vm a été supprimée.
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

fn load_config() {
    if let Ok(content) = fs::read_to_string("aegis.toml") {
        // println!("📝 Configuration trouvée..."); // Moins verbeux pour l'exécution normale
        let config: ProjectConfig = toml::from_str(&content).unwrap_or_else(|_| ProjectConfig { dependencies: None });

        if let Some(deps) = config.dependencies {
            for (name, _version_req) in deps {
                // println!("📦 Résolution de la dépendance '{}' ({})", name, version_req);

                let package_path = Path::new("packages").join(&name);

                if !package_path.exists() {
                    eprintln!("❌ Erreur : Le paquet '{}' n'est pas trouvé dans ./packages/", name);
                    eprintln!("   💡 Astuce : Lancez 'aegis add {}' pour l'installer.", name);
                    continue;
                }

                match resolve_library_path(&package_path) {
                    Ok(final_path) => {
                        // println!("   🔌 Chargement binaire : {:?}", final_path);
                        if let Err(e) = plugins::load_plugin(final_path.to_str().unwrap()) {
                            eprintln!("   ❌ Echec du chargement : {}", e);
                        }
                    },
                    Err(e) => eprintln!("   ❌ Erreur de configuration du paquet '{}': {}", name, e),
                }
            }
        }
    }
}

fn resolve_library_path(path: &Path) -> Result<std::path::PathBuf, String> {
    if path.is_dir() {
        let manifest_path = path.join("aegis.toml");
        
        if !manifest_path.exists() {
            return Err(format!("Manifeste 'aegis.toml' manquant dans {:?}", path));
        }

        let content = fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
        let manifest: PackageManifest = toml::from_str(&content)
            .map_err(|e| format!("Manifeste corrompu: {}", e))?;

        let current_os = std::env::consts::OS;

        if let Some(targets) = manifest.targets {
            if let Some(binary_file) = targets.get(current_os) {
                let binary_path = path.join(binary_file);
                if binary_path.exists() {
                    return Ok(binary_path);
                } else {
                    return Err(format!("Le binaire spécifié pour {} ({}) est introuvable sur le disque", current_os, binary_file));
                }
            } else {
                return Err(format!("Ce paquet ne supporte pas votre système ({})", current_os));
            }
        } else {
            return Err("Section [targets] manquante dans le paquet".into());
        }
    }
    
    if path.is_file() {
        return Ok(path.to_path_buf());
    }

    Err(format!("Chemin invalide : {:?}", path))
}

fn main() -> Result<(), String> {
    native::init_registry();
    load_config();

    let cli = Cli::parse();

    match &cli.command {
        // La commande RUN utilise maintenant la VM v2
        Some(Commands::Run { file, args: _ }) => {
            run_file(file)
        }

        Some(Commands::Repl) | None => {
            println!("Aegis v2.0 - REPL");
            println!("Tapez 'exit' ou 'quit' pour quitter.");
            run_repl();
            Ok(())
        }

        Some(Commands::Add { name, version }) => {
            package_manager::install(name, version.clone())
        }

        Some(Commands::Publish) => {
            package_manager::publish()
        }

        Some(Commands::Login { token }) => {
            package_manager::login(token)
        },
    }
}

// Nouvelle implémentation utilisant la VM v2
fn run_file(filename: &str) -> Result<(), String> {
    let content = fs::read_to_string(filename)
        .map_err(|e| format!("Impossible de lire {}: {}", filename, e))?;

    // 1. Frontend (Partagé avec la v1) : Source -> AST (JSON)
    let json_data: JsonValue = if filename.ends_with(".aeg") {
        compiler::compile(&content)?
    } else {
        serde_json::from_str(&content).map_err(|e| e.to_string())?
    };
    
    // 2. Loader (Partagé avec la v1) : AST -> Statements
    let statements = loader::parse_block(&json_data)?;

    // 3. Adapter : Statements -> Instructions pures
    // C'est nécessaire car le loader v1 retourne des objets Statement (avec métadonnées)
    // alors que le compilateur v2 attend des instructions brutes.
    let instructions = statements.into_iter().map(|s| s.kind).collect();

    // 4. Compilation v2 : Instructions -> Bytecode Chunk
    let compiler = aegis_core::vm::compiler::Compiler::new();
    let (chunk, global_names) = compiler.compile(instructions);

    // Debug: Décommenter pour voir le bytecode généré lors d'un simple run
    // use aegis_core::vm::debug;
    // debug::disassemble_chunk(&chunk, &format!("EXECUTION DE {}", filename));

    let mut script_args = Vec::new();
    // On filtre le "--" s'il est présent en premier
    if let Some(Commands::Run { args, .. }) = &Cli::parse().command {
        for arg in args {
            if arg != "--" {
                script_args.push(arg.clone());
            }
        }
    }

    // 5. Exécution VM
    let mut vm = VM::new(chunk, global_names, script_args);
    
    // On pourrait passer 'args' à la VM ici dans le futur
    vm.run()
}

fn run_repl() {
    let stdin = io::stdin();
    let mut input = String::new();

    // 1. Initialisation de l'environnement partagé
    // On crée la table des noms globaux qui sera partagée entre le compilateur et la VM
    let global_names = std::rc::Rc::new(std::cell::RefCell::new(HashMap::new()));
    
    // 2. Initialisation de la VM (à vide)
    // On crée un chunk vide juste pour initialiser la VM
    let empty_chunk = aegis_core::chunk::Chunk::new();
    let mut vm = VM::new(empty_chunk, global_names.clone(), vec![]);

    println!("Aegis v0.2.0 REPL");
    println!("Type 'exit' to quit.");

    loop {
        print!(">> ");
        io::stdout().flush().unwrap();

        input.clear();
        match stdin.read_line(&mut input) {
            Ok(_) => {
                let source = input.trim();
                if source == "exit" || source == "quit" { break; }
                if source.is_empty() { continue; }

                // --- PIPELINE v2 ---

                // A. Compilation v1 (Source -> JSON AST)
                match compiler::compile(source) {
                    Ok(json_ast) => {
                        // B. Loader (JSON -> Instructions)
                        match loader::parse_block(&json_ast) {
                            Ok(statements) => {
                                // Conversion Statement -> Instruction
                                let instructions: Vec<_> = statements.into_iter().map(|s| s.kind).collect();

                                // C. Compilation v2 (Instructions -> Bytecode)
                                // IMPORTANT : On utilise 'new_with_globals' pour que le compilateur
                                // connaisse les variables définies aux lignes précédentes !
                                let mut repl_compiler = aegis_core::vm::compiler::Compiler::new_with_globals(global_names.clone());
                                
                                // On force le scope global pour que 'var x = 1' soit persistant (SetGlobal)
                                repl_compiler.scope_depth = 0; 
                                
                                let (chunk, _) = repl_compiler.compile(instructions);

                                // D. Exécution (Injection dans la VM existante)
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
