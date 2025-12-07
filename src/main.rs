mod ast;
mod compiler;
mod interpreter;
mod loader;
mod native;

use ast::Environment;
use std::{fs, env, io::{self, Write}};
use serde_json::Value as JsonValue;

fn main() -> Result<(), String> {
    native::init_registry();
    let args: Vec<String> = env::args().collect();
    
    // CAS 1 : Pas d'arguments -> Mode REPL (Interactif)
    if args.len() < 2 {
        println!("Aegis v0.1.0 - Mode Interactif");
        println!("Tapez 'exit' ou 'quit' pour quitter.");
        run_repl();
        return Ok(());
    }

    // CAS 2 : Un fichier est fourni -> Exécution de fichier
    run_file(&args[1])
}

fn run_file(filename: &str) -> Result<(), String> {
    let content = fs::read_to_string(filename).map_err(|e| format!("Impossible de lire {}: {}", filename, e))?;

    // 1. Compilation
    let json_data: JsonValue = if filename.ends_with(".aeg") {
        compiler::compile(&content)?
    } else {
        serde_json::from_str(&content).map_err(|e| e.to_string())?
    };
    
    // 2. Loading
    let instructions = loader::parse_block(&json_data)?;
    
    // 3. Execution
    let global_env = Environment::new_global();

    // println!("--- Début de l'exécution ---");
    for instr in instructions {
        if let Err(e) = interpreter::execute(&instr, global_env.clone()) {
            eprintln!("Erreur d'exécution : {}", e);
            break;
        }
    }
    // println!("--- Fin ---");

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
