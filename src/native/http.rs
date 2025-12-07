use crate::ast::Value;
use std::collections::HashMap;

pub fn register(map: &mut HashMap<String, super::NativeFn>) {
    map.insert("http_get".to_string(), http_get);
    map.insert("http_post".to_string(), http_post);
}

fn http_get(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("http_get attend une url".into());
    }

    let url = args[0].as_str()?;

    // 1. Création d'un client avec User-Agent (Indispensable pour beaucoup d'API)
    let client = reqwest::blocking::Client::builder()
        .user_agent("Aegis-Lang/1.0")
        .build()
        .map_err(|e| format!("Erreur création client HTTP: {}", e))?;

    // 2. Envoi de la requête
    let response = client.get(&url)
        .send()
        .map_err(|e| format!("Erreur connexion: {}", e))?;

    // 3. Vérification du statut HTTP (200 OK ?)
    if !response.status().is_success() {
        return Err(format!("Erreur API: Code {}", response.status()));
    }

    // 4. Lecture du corps
    let text = response.text()
        .map_err(|e| format!("Erreur lecture body: {}", e))?;
                                    
    Ok(Value::String(text))
}

fn http_post(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 3 { 
        return Err("http_post attend 3 arguments (url, body, content_type)".into()); 
    }

    let url = args[0].as_str()?;
    let body = args[1].as_str()?;
    let content_type = args[2].as_str()?;
                                
    let client = reqwest::blocking::Client::new();
    let res = client.post(&url)
        .header("Content-Type", content_type)
        .body(body)
        .send()
        .map_err(|e| format!("Erreur Post: {}", e))?;
                                    
    if !res.status().is_success() {
        return Err(format!("Erreur API: {}", res.status()));
    }
                                
    Ok(Value::String(res.text().unwrap_or_default()))
}