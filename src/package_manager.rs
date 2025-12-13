use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use serde::Deserialize;
use reqwest::blocking::{Client, multipart};
use std::env; // To detect OS/Arch

const REGISTRY_URL: &str = "http://127.0.0.1:8000/api";

#[derive(Deserialize, Debug)]
struct PackageInfo {
    version: String,
    url: String, // The registry returns the URL for the specific asset (binary or source)
}

#[derive(Deserialize)]
struct Manifest {
    project: ProjectInfo,
}

#[derive(Deserialize)]
struct ProjectInfo {
    name: String,
    version: String,
}

// --- UTILS ---

fn get_credentials_path() -> PathBuf {
    dirs::home_dir().unwrap().join(".aegis").join("credentials")
}

fn get_token() -> Result<String, String> {
    let path = get_credentials_path();
    fs::read_to_string(&path).map_err(|_| "Not logged in. Run 'aegis login <token>'".to_string())
}

// Detects the current system information to match Django choices
fn get_system_info() -> (String, String) {
    let os = match env::consts::OS {
        "linux" => "linux",
        "macos" => "macos", 
        "windows" => "windows",
        _ => "any", // Fallback
    };

    let arch = match env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "arm64", // Rust uses aarch64, Django choices use arm64
        _ => "any",
    };

    (os.to_string(), arch.to_string())
}

fn find_library_in_dir(dir: &Path) -> Option<PathBuf> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                // We prioritize specific libs, but this logic assumes only one lib per package
                if ext == "dll" || ext == "so" || ext == "dylib" {
                    return Some(path);
                }
            }
        }
    }
    None
}

fn update_toml_dependency(name: &str, _path: &str) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open("aegis.toml")
        .map_err(|e| e.to_string())?;

    writeln!(file, "{} = \"*\"", name).map_err(|e| e.to_string())?;
    Ok(())
}

fn create_zip_of_directory(src_dir: &Path, dst_file: &Path) -> Result<(), String> {
    let file = File::create(dst_file).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for entry in WalkDir::new(src_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        
        // Ignore git, target, and the zip itself
        if path.to_string_lossy().contains("target") 
           || path.to_string_lossy().contains(".git") 
           || path == dst_file {
            continue;
        }

        if path.is_file() {
            // Calculate relative path for zip structure
            let name = path.strip_prefix(src_dir).unwrap();
            let name_str = name.to_str().unwrap().replace("\\", "/"); // Windows fix
            
            zip.start_file(name_str, options).map_err(|e| e.to_string())?;
            let mut f = File::open(path).map_err(|e| e.to_string())?;
            io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
        }
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
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
    // 1. Detect OS & Arch
    let (os, arch) = get_system_info();
    
    // 2. Query API with platform info
    // The Django view should use these params to return the correct file URL in the JSON
    // e.g., if on Linux, return URL for linux-x86_64.zip. If not found, return source.zip.
    let url = format!("{}/packages/{}/latest/?os={}&architecture={}", REGISTRY_URL, name, os, arch);
    println!("🔍 Searching for {} ({}/{})...", name, os, arch);

    let client = Client::new();
    let resp = client.get(&url).send().map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Package not found or server error ({})", resp.status()));
    }

    let info: PackageInfo = resp.json().map_err(|e| format!("JSON Error: {}", e))?;
    println!("⬇️  Downloading version {}...", info.version);

    // 3. Download the asset
    let zip_resp = client.get(&info.url).send().map_err(|e| e.to_string())?;
    let zip_bytes = zip_resp.bytes().map_err(|e| e.to_string())?;

    // 4. Unzip
    let packages_dir = Path::new("packages").join(name);
    if packages_dir.exists() {
        fs::remove_dir_all(&packages_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&packages_dir).map_err(|e| e.to_string())?;

    let reader = std::io::Cursor::new(zip_bytes);
    let mut zip = zip::ZipArchive::new(reader).map_err(|e| e.to_string())?;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        // Security check needed here against ZipSlip vulnerability in production
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

    // 5. Update toml logic
    // If it's a binary package (contains .so/.dll), use that.
    // If it's a source package (contains .aeg), allow standard import.
    if let Some(lib_path) = find_library_in_dir(&packages_dir) {
        // It's a native module
        update_toml_dependency(name, lib_path.to_str().unwrap())?;
        println!("✅ Native package {} installed successfully!", name);
    } else {
        // It's likely a pure Aegis source package
        update_toml_dependency(name, "")?;
        println!("✅ Source package {} installed successfully!", name);
    }
    
    Ok(())
}

// Updated publish signature to accept platform flags
// Usage: aegis publish --os linux --arch x86_64 (for binary)
// Usage: aegis publish (defaults to source/any)
pub fn publish(target_os: Option<String>, target_arch: Option<String>) -> Result<(), String> {
    let content = fs::read_to_string("aegis.toml").map_err(|_| "aegis.toml not found")?;
    let manifest: Manifest = toml::from_str(&content).map_err(|e| format!("TOML Error: {}", e))?;

    let token = get_token()?;
    
    // Determine platform metadata
    let os_val = target_os.unwrap_or("any".to_string());
    let arch_val = target_arch.unwrap_or("any".to_string());

    println!("🚀 Publishing {} v{} for {}/{}...", 
        manifest.project.name, 
        manifest.project.version,
        os_val,
        arch_val
    );

    // 2. Zip directory
    let zip_path = Path::new("package.zip");
    create_zip_of_directory(Path::new("."), zip_path)?;

    // 3. Send via Multipart
    let url = format!("{}/packages/{}/publish/", REGISTRY_URL, manifest.project.name);

    // We add the new fields 'os' and 'architecture' to the form
    let form = multipart::Form::new()
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

    if res.status().is_success() {
        println!("✅ Published successfully!");
        Ok(())
    } else {
        let err_text = res.text().unwrap_or_default();
        Err(format!("Publish failed: {}", err_text))
    }
}
