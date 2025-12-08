use aegis_core::{compiler, interpreter, loader, native, plugins};
use aegis_core::ast::environment::Environment;
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
    
    /// Lance les tests unitaires d'un fichier - Toujours sur v1 pour l'instant
    Test {
        file: String
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
            println!("Aegis v2.0 - Mode Interactif (Legacy v1 Engine)");
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
        
        Some(Commands::Test { file }) => {
            run_tests(file)
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
    let chunk = compiler.compile(instructions);

    // Debug: Décommenter pour voir le bytecode généré lors d'un simple run
    // use aegis_core::vm::debug;
    // debug::disassemble_chunk(&chunk, &format!("EXECUTION DE {}", filename));

    // 5. Exécution VM
    let mut vm = VM::new(chunk);
    
    // On pourrait passer 'args' à la VM ici dans le futur
    vm.run()
}

// NOTE: Le REPL utilise encore l'interpréteur v1 pour l'instant
// Car la VM v2 actuelle ne persiste pas l'état entre deux 'chunks' (run reset la VM)
fn run_repl() {
    let global_env = Environment::new_global();
    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        print!(">> ");
        io::stdout().flush().unwrap();

        input.clear();
        match stdin.read_line(&mut input) {
            Ok(_) => {
                let source = input.trim();
                if source == "exit" || source == "quit" { break; }
                if source.is_empty() { continue; }

                match compiler::compile(source) {
                    Ok(json_ast) => {
                        match loader::parse_block(&json_ast) {
                            Ok(instructions) => {
                                for instr in instructions {
                                    if let Err(e) = interpreter::execute(&instr, global_env.clone()) {
                                        println!("Erreur Runtime: {}", e);
                                    }
                                }
                            },
                            Err(e) => println!("Erreur Loader: {}", e)
                        }
                    },
                    Err(e) => println!("Erreur Syntaxe: {}", e)
                }
            }
            Err(error) => {
                println!("Erreur lecture: {}", error);
                break;
            }
        }
    }
}

// NOTE: Les tests utilisent encore le moteur v1 pour l'instant
fn run_tests(filename: &str) -> Result<(), String> {
    println!("🧪 Lancement des tests pour : {}", filename);

    let content = fs::read_to_string(filename)
        .map_err(|e| format!("Impossible de lire {}: {}", filename, e))?;

    let json_data = if filename.ends_with(".aeg") {
        compiler::compile(&content)?
    } else {
        serde_json::from_str(&content).map_err(|e| e.to_string())?
    };
    
    let instructions = loader::parse_block(&json_data)?;
    let env = Environment::new_global();

    for instr in instructions {
        if let Err(e) = interpreter::execute(&instr, env.clone()) {
            return Err(format!("Erreur lors du chargement du script : {}", e));
        }
    }

    let tests_val = env.borrow().get_variable("__AEGIS_TESTS")
        .ok_or("Aucun test trouvé (Avez-vous importé 'stdlib/test.aeg' ?)")?;

    let tests_list = match tests_val {
        aegis_core::Value::List(l) => l,
        _ => return Err("Le registre des tests est corrompu".into())
    };

    let tests = tests_list.borrow();
    println!("📝 {} tests détectés.\n", tests.len());

    let mut passed = 0;
    let mut failed = 0;

    for (i, test_func) in tests.iter().enumerate() {
        print!("Test #{} ... ", i + 1);
        io::stdout().flush().unwrap();

        let res = interpreter::apply_func(test_func.clone(), vec![], env.clone());

        match res {
            Ok(_) => {
                println!("✅ OK");
                passed += 1;
            },
            Err(e) => {
                println!("❌ ECHEC");
                println!("   └─ {}", e);
                failed += 1;
            }
        }
    }

    println!("\n--- RÉSULTATS ---");
    println!("Total : {}", tests.len());
    println!("Succès: {}", passed);
    println!("Echecs: {}", failed);

    if failed > 0 {
        return Err("Certains tests ont échoué.".into());
    }
    
    Ok(())
}
