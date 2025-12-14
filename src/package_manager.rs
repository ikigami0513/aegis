use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;
use serde::Deserialize;
use reqwest::blocking::{Client, multipart};
use std::env;

// Import toml_edit for safe TOML manipulation
use toml_edit::{DocumentMut, value, Item, Table};

const REGISTRY_URL: &str = "https://aegis.foxvoid.com/api";

#[derive(Deserialize)]
struct CargoPackage {
    name: String,
}

#[derive(Deserialize)]
struct CargoManifest {
    package: CargoPackage,
}

#[derive(Deserialize, Debug)]
struct PackageInfo {
    version: String,
    url: String, 
}

#[derive(Deserialize)]
struct Manifest {
    project: ProjectInfo,
}

#[derive(Deserialize)]
struct ProjectInfo {
    name: String,
    version: String,
    exclude: Option<Vec<String>>,
}

// --- UTILS ---

fn get_credentials_path() -> PathBuf {
    dirs::home_dir().unwrap().join(".aegis").join("credentials")
}

fn get_token() -> Result<String, String> {
    let path = get_credentials_path();
    let content = fs::read_to_string(&path)
        .map_err(|_| "Non connecté. Faites 'aegis login <token>'".to_string())?;
    
    Ok(content.trim().to_string()) 
}

fn get_system_info() -> (String, String) {
    let os = match env::consts::OS {
        "linux" => "linux",
        "macos" => "macos", 
        "windows" => "windows",
        _ => "any", 
    };

    let arch = match env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "arm64", 
        _ => "any",
    };

    (os.to_string(), arch.to_string())
}

fn find_library_in_dir(dir: &Path) -> Option<PathBuf> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "dll" || ext == "so" || ext == "dylib" {
                    return Some(path);
                }
            }
        }
    }
    None
}

// --- UPDATED FUNCTION USING TOML_EDIT ---
fn update_toml_dependency(name: &str, _path: &str) -> Result<(), String> {
    let toml_path = "aegis.toml";
    
    // 1. Read existing content or create empty if missing
    let content = fs::read_to_string(toml_path).unwrap_or_default();
    
    // 2. Parse into a mutable Document (preserves comments and formatting)
    let mut doc = content.parse::<DocumentMut>()
        .map_err(|e| format!("Failed to parse aegis.toml: {}", e))?;

    // 3. Ensure [dependencies] table exists
    // as_table_mut returns None if the key doesn't exist or isn't a table
    if doc.get("dependencies").is_none() {
        // We insert a standard table (with [brackets]), not an inline table
        doc["dependencies"] = Item::Table(Table::new());
    }

    // 4. Add or update the dependency
    // We strictly use `doc["dependencies"]` now that we know it exists/is created
    doc["dependencies"][name] = value("*");

    // 5. Write back to file
    fs::write(toml_path, doc.to_string()).map_err(|e| e.to_string())?;
    
    Ok(())
}

fn create_zip_of_directory(src_dir: &Path, dst_file: &Path, excludes: &[String]) -> Result<(), String> {
    let file = File::create(dst_file).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755)
        .large_file(true);

    // On récupère le nom du fichier zip de destination pour l'exclure sûrement
    let dst_filename = dst_file.file_name().unwrap().to_string_lossy();

    for entry in WalkDir::new(src_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let path_str = path.to_string_lossy();

        // 1. Exclusions Système Robustes
        // On exclut le fichier zip LUI-MÊME par son nom, peu importe le chemin ./
        if let Some(name) = path.file_name() {
            if name.to_string_lossy() == dst_filename {
                continue;
            }
        }
        
        if path_str.contains("target") || path_str.contains(".git") {
            continue;
        }
        
        // Sécurité supplémentaire : on évite d'inclure le dossier 'packages'
        // car on ne veut pas republier nos dépendances installées.
        if path.starts_with("./packages") || path.starts_with("packages") {
            continue;
        }

        // 2. Exclusions Utilisateur (aegis.toml)
        if let Ok(relative) = path.strip_prefix(src_dir) {
            let relative_str = relative.to_string_lossy();
            let mut is_excluded = false;

            for pattern in excludes {
                if relative_str.starts_with(pattern) || relative_str == *pattern {
                    is_excluded = true;
                    break;
                }
            }

            if is_excluded {
                continue;
            }
        }

        if path.is_file() {
            let name = path.strip_prefix(src_dir).unwrap();
            let name_str = name.to_str().unwrap().replace("\\", "/");
            
            println!("   📦 Zipping: {}", name_str); // Debug visuel utile
            zip.start_file(name_str, options).map_err(|e| e.to_string())?;
            let mut f = File::open(path).map_err(|e| e.to_string())?;
            io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
        }
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

fn build_native_package(aegis_project_name: &str) -> Result<String, String> {
    println!("⚙️  Compiling native code (Cargo)...");

    let cargo_content = fs::read_to_string("Cargo.toml")
        .map_err(|_| "Cargo.toml not found. Is this a Rust project?".to_string())?;
    
    let cargo_manifest: CargoManifest = toml::from_str(&cargo_content)
        .map_err(|e| format!("Failed to parse Cargo.toml: {}", e))?;
    
    let cargo_name = cargo_manifest.package.name; 

    let status = Command::new("cargo")
        .args(["build", "--release"])
        .status()
        .map_err(|_| "Failed to run cargo. Is it installed?")?;

    if !status.success() {
        return Err("Cargo compilation failed.".to_string());
    }

    let clean_cargo_name = cargo_name.replace("-", "_");
    let clean_aegis_name = aegis_project_name.replace("-", "_");
    
    let (prefix, suffix) = if cfg!(target_os = "windows") {
        ("", ".dll")
    } else if cfg!(target_os = "macos") {
        ("lib", ".dylib")
    } else {
        ("lib", ".so") 
    };

    let src_filename = format!("{}{}{}", prefix, clean_cargo_name, suffix);
    let src_path = Path::new("target").join("release").join(&src_filename);

    let dst_filename = format!("{}{}{}", prefix, clean_aegis_name, suffix);
    let dst_path = Path::new(&dst_filename);

    if !src_path.exists() {
        return Err(format!("Compiled file not found at: {:?}", src_path));
    }

    fs::copy(&src_path, dst_path).map_err(|e| format!("Failed to copy binary: {}", e))?;

    println!("✅ Binary generated and renamed: {} -> {}", src_filename, dst_filename);
    
    Ok(dst_filename)
}

// --- PUBLIC COMMANDS ---

pub fn login(token: &str) -> Result<(), String> {
    let cred_path = get_credentials_path();
    if let Some(parent) = cred_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&cred_path, token).map_err(|e| format!("Error writing token: {}", e))?;
    println!("✅ Token saved in {:?}", cred_path);
    Ok(())
}

pub fn install(name: &str, _version: Option<String>) -> Result<(), String> {
    let (os, arch) = get_system_info();
    
    let url = format!("{}/packages/{}/latest/?os={}&architecture={}", REGISTRY_URL, name, os, arch);
    println!("🔍 Searching for {} ({}/{})...", name, os, arch);

    let client = Client::new();
    let resp = client.get(&url).send().map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Package not found or server error ({})", resp.status()));
    }

    let info: PackageInfo = resp.json().map_err(|e| format!("JSON Error: {}", e))?;
    println!("⬇️  Downloading version {}...", info.version);

    let zip_resp = client.get(&info.url).send().map_err(|e| e.to_string())?;
    let zip_bytes = zip_resp.bytes().map_err(|e| e.to_string())?;

    let packages_dir = Path::new("packages").join(name);
    if packages_dir.exists() {
        fs::remove_dir_all(&packages_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&packages_dir).map_err(|e| e.to_string())?;

    let reader = std::io::Cursor::new(zip_bytes);
    let mut zip = zip::ZipArchive::new(reader).map_err(|e| e.to_string())?;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        let outpath = packages_dir.join(file.mangled_name());

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).unwrap();
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() { fs::create_dir_all(p).unwrap(); }
            }
            let mut outfile = File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }
    }

    if let Some(lib_path) = find_library_in_dir(&packages_dir) {
        update_toml_dependency(name, lib_path.to_str().unwrap())?;
        println!("✅ Native package {} installed successfully!", name);
    } else {
        update_toml_dependency(name, "")?;
        println!("✅ Source package {} installed successfully!", name);
    }
    
    Ok(())
}

pub fn publish(mut target_os: Option<String>, mut target_arch: Option<String>) -> Result<(), String> {
    let content = fs::read_to_string("aegis.toml").map_err(|_| "aegis.toml not found")?;
    let manifest: Manifest = toml::from_str(&content).map_err(|e| format!("TOML Error: {}", e))?;

    let token = get_token()?;

    let is_native_build = target_os.is_some() || target_arch.is_some();

    let mut generated_binary = None;
    if is_native_build {
        let bin_name = build_native_package(&manifest.project.name)?;
        generated_binary = Some(bin_name);
        
        if target_os.is_none() { target_os = Some(std::env::consts::OS.to_string()); }
        if target_arch.is_none() { target_arch = Some(std::env::consts::ARCH.to_string()); }
    }
    
    let os_val = target_os.unwrap_or("any".to_string());
    let arch_val = target_arch.unwrap_or("any".to_string());

    println!("🚀 Publishing {} v{} for {}/{}...", 
        manifest.project.name, 
        manifest.project.version,
        os_val,
        arch_val
    );

    let zip_path = Path::new("package.zip");
    let user_excludes = manifest.project.exclude.unwrap_or_default();
    create_zip_of_directory(Path::new("."), zip_path, &user_excludes)?;

    let url = format!("{}/packages/publish/", REGISTRY_URL);

    let form = multipart::Form::new()
        .text("name", manifest.project.name.to_string())
        .text("version", manifest.project.version)
        .text("os", os_val)
        .text("architecture", arch_val)
        .file("file", zip_path).map_err(|e| e.to_string())?;

    let client = Client::new();
    let res = client.post(&url)
        .header("Authorization", format!("Token {}", token))
        .multipart(form)
        .send()
        .map_err(|e| e.to_string())?;

    let _ = fs::remove_file(zip_path);

    if let Some(bin_name) = generated_binary {
        let _ = fs::remove_file(bin_name);
    }

    if res.status().is_success() {
        println!("✅ Published successfully!");
        Ok(())
    } else {
        let err_text = res.text().unwrap_or_default();
        Err(format!("Publish failed: {}", err_text))
    }
}
