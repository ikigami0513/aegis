# Writing Native Extensions

Aegis is designed to be extensible. You can write high-performance modules in **Rust** and load them dynamically into your Aegis scripts.

## Prerequisites

You need Rust installed. Create a new library project:

```bash
cargo new --lib my_plugin
```

Update your Cargo.toml to produce a dynamic library:
```Ini, TOML
[package]
name = "my_aegis_package"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[[bin]]
name = "package"
path = "scripts/build_package.rs"

[dependencies]
# You must link against the core aegis library definition
aegis_core = { path = "../path/to/aegis_core" } 
```

## The Build Script

Create the file `scripts/build_package.rs` and paste the following code into it. You will need this to compile your package.

```rust
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // 1. Récupération DYNAMIQUE du nom du projet
    let crate_name = env!("CARGO_PKG_NAME");
    
    println!("📦 Packaging du projet : [{}]", crate_name);

    // 2. Détection de l'OS pour l'extension
    let (lib_prefix, lib_ext) = if cfg!(target_os = "windows") {
        ("", "dll")
    } else if cfg!(target_os = "macos") {
        ("lib", "dylib")
    } else {
        ("lib", "so") // Linux
    };

    let lib_filename = format!("{}{}.{}", lib_prefix, crate_name, lib_ext);

    // 3. Lancer la compilation
    println!("⚙️  Compilation en cours (Release)...");
    
    let status = Command::new("cargo")
        .args(&["build", "--release", "--lib"])
        .status()
        .expect("Impossible de lancer cargo");

    if !status.success() {
        eprintln!("❌ Erreur lors de la compilation Rust.");
        std::process::exit(1);
    }

    // 4. Préparation des dossiers (CHANGEMENTS ICI)
    let root_dir = env::current_dir().unwrap();
    let dist_root = root_dir.join("dist");
    
    // On crée le chemin : dist/<nom_du_paquet>/
    let package_out_dir = dist_root.join(crate_name);

    println!("📂 Dossier de sortie : {:?}", package_out_dir);

    // Nettoyage de la version précédente de CE paquet uniquement
    if package_out_dir.exists() {
        fs::remove_dir_all(&package_out_dir).unwrap();
    }
    // Création de l'arborescence complète
    fs::create_dir_all(&package_out_dir).unwrap();

    // 5. Copie du binaire (.dll / .so)
    let target_dir = root_dir.join("target/release");
    let src_lib_path = target_dir.join(&lib_filename);
    
    // Destination dans le sous-dossier
    let dest_lib_path = package_out_dir.join(&lib_filename);

    println!("📄 Copie du binaire : {}", lib_filename);
    if src_lib_path.exists() {
        fs::copy(&src_lib_path, &dest_lib_path)
            .unwrap_or_else(|e| panic!("Erreur copie DLL : {}", e));
    } else {
        eprintln!("❌ Fichier introuvable : {:?}", src_lib_path);
        eprintln!("   Vérifiez le 'name' dans Cargo.toml");
        std::process::exit(1);
    }

    // 6. Copie des scripts Aegis (contenu de /packages)
    let packages_dir = root_dir.join("packages");
    if packages_dir.exists() {
        println!("📂 Copie des scripts Aegis...");
        // On copie VERS le sous-dossier spécifique
        copy_dir_recursive(&packages_dir, &package_out_dir).expect("Erreur copie packages");
    } else {
        println!("⚠️  Aucun dossier 'packages/' trouvé (seul le binaire sera distribué).");
    }

    // 7. Génération du manifeste aegis.toml pour le paquet
    println!("📝 Génération du manifeste de paquet...");
    
    // Noms théoriques des fichiers pour les autres OS
    let lib_name_linux = format!("lib{}.so", crate_name);
    let lib_name_windows = format!("{}.dll", crate_name);
    let lib_name_macos = format!("lib{}.dylib", crate_name);

    let toml_content = format!(r#"[package]
name = "{}"
version = "0.1.0"

[targets]
linux = "{}"
windows = "{}"
macos = "{}"
"#, crate_name, lib_name_linux, lib_name_windows, lib_name_macos);

    let manifest_path = package_out_dir.join("aegis.toml");
    fs::write(&manifest_path, toml_content).expect("Impossible de créer le manifeste");

    println!("\n✅ SUCCÈS ! Votre package est prêt dans : dist/{}/", crate_name);
}

// Fonction utilitaire inchangée
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
```

## The Entry Point

Aegis looks for a specific symbol `_aegis_register` to load your functions.

`src/lib.rs`:

```rust
use aegis_core::{Value, NativeFn};
use std::collections::HashMap;

// This function is called by the VM when loading the plugin
#[no_mangle]
pub extern "C" fn _aegis_register(map: &mut HashMap<String, NativeFn>) {
    // Map an Aegis function name to a Rust function
    map.insert("my_hello".to_string(), hello_world);
    map.insert("my_add".to_string(), add_numbers);
}

// 1. A simple function
fn hello_world(_args: Vec<Value>) -> Result<Value, String> {
    println!("Hello from Rust!");
    Ok(Value::Null)
}

// 2. Handling arguments
fn add_numbers(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments".into());
    }

    // Use helper methods to safely cast values
    let a = args[0].as_int()?;
    let b = args[1].as_int()?;

    Ok(Value::Integer(a + b))
}
```

## Aegis Library File

Create a `packages/my_plugin.aeg` file to make it easy to use:

```aegis
// Wrap in a Namespace
namespace MyPlugin {
    func hello() {
        return my_hello()
    }
    
    func add(a, b) {
        return my_add(a, b)
    }
}
```

## Compiling

Build your plugin in release mode:

```bash
cargo run --bin package
```

This will generate a `dist/<package_name>/` folder which contains the ready-to-use Aegis package:
- Shared Library Files (`.dll` for Windows, `.so` for Linux and `.dylib` for MacOS)
- The Aegis files (`.aeg`) created in the `packages/` folder
- An `aegis.toml` manifest file automatically generated by the build script

## Usage in Aegis

```aegis
import "packages/my_plugin.aeg"

MyPlugin.hello() // Prints: Hello from Rust!
var sum = MyPlugin.add(10, 20)
print sum // 30
```
