use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use serde::Deserialize;
use reqwest::blocking::{Client, multipart};

const REGISTRY_URL: &str = "http://127.0.0.1:8000/api";

#[derive(Deserialize)]
struct PackageInfo {
    version: String,
    url: String, // URL du .zip
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

// --- UTILITAIRES ---
fn get_credentials_path() -> PathBuf {
    dirs::home_dir().unwrap().join(".aegis").join("credentials")
}

fn get_token() -> Result<String, String> {
    let path = get_credentials_path();
    fs::read_to_string(&path).map_err(|_| "Non connecté. Faites 'aegis login <token>'".to_string())
}

// Cherche un fichier .dll, .so ou .dylib
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

// Ajoute la dépendance de manière "brute" à la fin du fichier (pour ne pas casser le formatage)
fn update_toml_dependency(name: &str, _path: &str) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open("aegis.toml")
        .map_err(|e| e.to_string())?;

    // Cela assume que le téléchargement a bien mis les fichiers dans packages/<name>
    writeln!(file, "{} = \"*\"", name).map_err(|e| e.to_string())?;
    
    Ok(())
}

fn create_zip_of_directory(src_dir: &Path, dst_file: &Path) -> Result<(), String> {
    let file = File::create(dst_file).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // On parcourt tout, mais on ignore .git, target, et le fichier zip lui-même
    for entry in WalkDir::new(src_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        
        // Filtrage basique
        if path.to_string_lossy().contains("target") || path.to_string_lossy().contains(".git") || path == dst_file {
            continue;
        }

        if path.is_file() {
            let name = path.strip_prefix(src_dir).unwrap();
            let name_str = name.to_str().unwrap();
            
            zip.start_file(name_str, options).map_err(|e| e.to_string())?;
            let mut f = File::open(path).map_err(|e| e.to_string())?;
            io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
        }
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

// --- COMMANDES PUBLIQUES ---

pub fn login(token: &str) -> Result<(), String> {
    let cred_path = get_credentials_path();

    if let Some(parent) = cred_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    fs::write(&cred_path, token).map_err(|e| format!("Erreur écriture token: {}", e))?;
    println!("✅ Token sauvegardé dans {:?}", cred_path);
    Ok(())
}

pub fn install(name: &str, _version: Option<String>) -> Result<(), String> {
    // 1. Interroger l'API pour obtenir l'URL de téléchargement
    // Note: On prend toujours "latest" pour simplifier ici
    let url = format!("{}/packages/{}/latest/", REGISTRY_URL, name);
    println!("🔍 Recherche de {}...", url);

    let client = Client::new();
    let resp = client.get(&url).send().map_err(|e| format!("Erreur réseau: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Paquet introuvable ou erreur serveur ({})", resp.status()));
    }

    let info: PackageInfo = resp.json().map_err(|e| format!("Erreur lecture JSON: {}", e))?;
    println!("⬇️  Téléchargement de la version {}...", info.version);

    // 2. Télécharger le ZIP
    let zip_resp = client.get(&info.url).send().map_err(|e| e.to_string())?;
    let zip_bytes = zip_resp.bytes().map_err(|e| e.to_string())?;

    // 3. Décompresser dans plugins/<name>
    let plugins_dir = Path::new("packages").join(name);
    if plugins_dir.exists() {
        fs::remove_dir_all(&plugins_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&plugins_dir).map_err(|e| e.to_string())?;

    let reader = std::io::Cursor::new(zip_bytes);
    let mut zip = zip::ZipArchive::new(reader).map_err(|e| e.to_string())?;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        let outpath = plugins_dir.join(file.mangled_name());

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

    // 4. Mettre à jour aegis.toml
    // Astuce : On détecte le fichier .dll/.so/.dylib dans le dossier extrait
    let lib_path = find_library_in_dir(&plugins_dir).ok_or("Aucune librairie (.dll/.so) trouvée dans le paquet")?;
    
    // On ajoute la ligne au fichier TOML
    update_toml_dependency(name, lib_path.to_str().unwrap())?;

    println!("✅ Paquet {} installé avec succès !", name);
    Ok(())
}

pub fn publish() -> Result<(), String> {
    // 1. Lire le manifest pour avoir nom/version
    let content = fs::read_to_string("aegis.toml").map_err(|_| "aegis.toml introuvable")?;
    let manifest: Manifest = toml::from_str(&content).map_err(|e| format!("Erreur TOML: {}", e))?;

    let token = get_token()?;
    println!("🚀 Publication de {} v{}...", manifest.project.name, manifest.project.version);

    // 2. Créer une archive ZIP en mémoire
    let zip_path = Path::new("package.zip");
    create_zip_of_directory(Path::new("."), zip_path)?;

    // 3. Envoyer via l'API
    let url = format!("{}/packages/{}/publish/", REGISTRY_URL, manifest.project.name);

    let form = multipart::Form::new()
        .text("version", manifest.project.version)
        .file("file", zip_path).map_err(|e| e.to_string())?;

    let client = Client::new();
    let res = client.post(&url)
        .header("Authorization", format!("Token {}", token))
        .multipart(form)
        .send()
        .map_err(|e| e.to_string())?;

    // Nettoyage
    let _ = fs::remove_file(zip_path);

    if res.status().is_success() {
        println!("✅ Publié avec succès !");
        Ok(())
    } else {
        let err_text = res.text().unwrap_or_default();
        Err(format!("Echec publication : {}", err_text))
    }
}