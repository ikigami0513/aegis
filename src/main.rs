use aegis_core::{compiler, interpreter, loader, native, plugins};
use aegis_core::ast::environment::Environment;
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Write;
use std::{fs, io};
use serde_json::Value as JsonValue;
use std::path::Path;

mod package_manager;

#[derive(Parser)]
#[command(name = "aegis")]
#[command(about = "Aegis Language Compiler & Package Manager", version = "1.0", long_about = None)]
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
        
        /// Arguments à passer au script (accessibles via System.args())
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
    Publish,

    /// [APM] Se connecte au registre
    Login {
        token: String
    },
    
    /// Lance les tests unitaires d'un fichier
    Test {
        file: String
    }
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
        println!("📝 Configuration trouvée...");
        let config: ProjectConfig = toml::from_str(&content).unwrap_or_else(|_| ProjectConfig { dependencies: None });

        if let Some(deps) = config.dependencies {
            for (name, version_req) in deps {
                println!("📦 Résolution de la dépendance '{}' ({})", name, version_req);

                // --- NOUVELLE LOGIQUE DE RÉSOLUTION ---
                // Convention : Le paquet est dans ./packages/<nom>/
                let package_path = Path::new("packages").join(&name);

                // On vérifie d'abord si le paquet est installé
                if !package_path.exists() {
                    eprintln!("❌ Erreur : Le paquet '{}' n'est pas trouvé dans ./packages/", name);
                    eprintln!("   💡 Astuce : Lancez 'aegis add {}' pour l'installer.", name);
                    continue;
                }

                // On demande au résolveur de trouver le bon binaire (.so/.dll) dans ce dossier
                match resolve_library_path(&package_path) {
                    Ok(final_path) => {
                        println!("   🔌 Chargement binaire : {:?}", final_path);
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
    // Si c'est un dossier (Cas standard maintenant)
    if path.is_dir() {
        let manifest_path = path.join("aegis.toml");
        
        if !manifest_path.exists() {
            return Err(format!("Manifeste 'aegis.toml' manquant dans {:?}", path));
        }

        let content = fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
        let manifest: PackageManifest = toml::from_str(&content)
            .map_err(|e| format!("Manifeste corrompu: {}", e))?;

        // Détection de l'OS
        let current_os = std::env::consts::OS; // "linux", "windows", "macos"

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
    
    // Support Legacy (si on pointe directement un fichier .so/.dll)
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
        Some(Commands::Run { file, args: _ }) => {
            run_file(file)
        }

        Some(Commands::Repl) | None => {
            println!("Aegis v1.0 - Mode Interactif");
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
        }
    }
}

fn run_file(filename: &str) -> Result<(), String> {
    let content = fs::read_to_string(filename)
        .map_err(|e| format!("Impossible de lire {}: {}", filename, e))?;

    let json_data: JsonValue = if filename.ends_with(".aeg") {
        compiler::compile(&content)?
    } else {
        serde_json::from_str(&content).map_err(|e| e.to_string())?
    };
    
    let instructions = loader::parse_block(&json_data)?;
    let global_env = Environment::new_global();

    for instr in instructions {
        if let Err(e) = interpreter::execute(&instr, global_env.clone()) {
            eprintln!("Erreur d'exécution : {}", e);
            return Err(e); // Arrêt propre en cas d'erreur
        }
    }
    Ok(())
}

fn run_repl() {
    let global_env = Environment::new_global();
    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        // Affiche le prompt ">> "
        print!(">> ");
        io::stdout().flush().unwrap();

        input.clear();
        match stdin.read_line(&mut input) {
            Ok(_) => {
                let source = input.trim();
                if source == "exit" || source == "quit" { break; }
                if source.is_empty() { continue; }

                // Pour le REPL, on compile ligne par ligne
                match compiler::compile(source) {
                    Ok(json_ast) => {
                        match loader::parse_block(&json_ast) {
                            Ok(instructions) => {
                                for instr in instructions {
                                    // On garde le même environnement à chaque tour de boucle !
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

fn run_tests(filename: &str) -> Result<(), String> {
    println!("🧪 Lancement des tests pour : {}", filename);

    // 1. On charge et exécute le fichier normalement pour remplir le registre
    let content = fs::read_to_string(filename)
        .map_err(|e| format!("Impossible de lire {}: {}", filename, e))?;

    // Compilation...
    let json_data = if filename.ends_with(".aeg") {
        compiler::compile(&content)?
    } else {
        serde_json::from_str(&content).map_err(|e| e.to_string())?
    };
    
    let instructions = loader::parse_block(&json_data)?;
    let env = Environment::new_global(); // Environnement partagé

    // Exécution du script (pour définir les fonctions et remplir __AEGIS_TESTS)
    for instr in instructions {
        if let Err(e) = interpreter::execute(&instr, env.clone()) {
            return Err(format!("Erreur lors du chargement du script : {}", e));
        }
    }

    // 2. On récupère le registre __AEGIS_TESTS
    // C'est une Value::List
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

    // 3. On exécute chaque fonction de test isolément
    for (i, test_func) in tests.iter().enumerate() {
        print!("Test #{} ... ", i + 1);
        io::stdout().flush().unwrap();

        // On utilise notre helper `apply_func` (il faut qu'il soit public ou accessible)
        // Ou on recrée un petit scope enfant manuellement
        
        // Astuce : Pour capturer l'erreur sans crasher le runner, on utilise le Result de Rust
        // On crée un appel de fonction sans arguments
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
