use crate::{Value, NativeFn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use lazy_static::lazy_static;

// --- STATE ---
struct SocketState {
    listeners: HashMap<usize, TcpListener>,
    streams: HashMap<usize, TcpStream>,
    next_id: usize,
}

struct ThreadSafeState(SocketState);
unsafe impl Send for ThreadSafeState {}

lazy_static! {
    static ref STATE: Mutex<ThreadSafeState> = Mutex::new(ThreadSafeState(SocketState {
        listeners: HashMap::new(),
        streams: HashMap::new(),
        next_id: 1,
    }));
}

// --- REGISTER ---
pub fn register(map: &mut HashMap<String, NativeFn>) {
    map.insert("sock_bind".to_string(), sock_bind);
    map.insert("sock_accept".to_string(), sock_accept);
    map.insert("sock_connect".to_string(), sock_connect);
    map.insert("sock_read".to_string(), sock_read);
    map.insert("sock_read_bytes".to_string(), sock_read_bytes);
    map.insert("sock_write".to_string(), sock_write);
    map.insert("sock_close".to_string(), sock_close);
}

// --- IMPLEMENTATION ---

// 1. SERVEUR : Bind un port
fn sock_bind(args: Vec<Value>) -> Result<Value, String> {
    if args.len() < 2 { return Err("Args: host, port".into()); }
    
    let host = args[0].as_str()?;
    let port = args[1].as_int()?;
    let addr = format!("{}:{}", host, port);

    let listener = TcpListener::bind(&addr).map_err(|e| e.to_string())?;
    
    // On met le listener en mode non-bloquant ? Non, restons simple (bloquant) pour l'instant.
    // Ou alors on laisse le script gérer ça.
    
    let mut guard = STATE.lock().unwrap();
    let state = &mut guard.0;
    
    let id = state.next_id;
    state.listeners.insert(id, listener);
    state.next_id += 1;

    Ok(Value::Integer(id as i64))
}

// 2. SERVEUR : Accepter une connexion (BLOQUANT)
fn sock_accept(args: Vec<Value>) -> Result<Value, String> {
    let id = args[0].as_int()? as usize;
    
    let mut guard = STATE.lock().unwrap();
    let state = &mut guard.0;
    
    let listener = state.listeners.get(&id).ok_or("Invalid Listener ID")?;
    
    match listener.accept() {
        Ok((stream, _addr)) => {
            // On a une nouvelle connexion (Stream)
            let stream_id = state.next_id;
            state.streams.insert(stream_id, stream);
            state.next_id += 1;
            Ok(Value::Integer(stream_id as i64))
        },
        Err(e) => Err(e.to_string())
    }
}

// 3. CLIENT : Se connecter
fn sock_connect(args: Vec<Value>) -> Result<Value, String> {
    let host = args[0].as_str()?;
    let port = args[1].as_int()?;
    let addr = format!("{}:{}", host, port);

    let stream = TcpStream::connect(&addr).map_err(|e| e.to_string())?;

    let mut guard = STATE.lock().unwrap();
    let state = &mut guard.0;
    
    let id = state.next_id;
    state.streams.insert(id, stream);
    state.next_id += 1;

    Ok(Value::Integer(id as i64))
}

// 4. READ (Lecture de N octets)
fn sock_read(args: Vec<Value>) -> Result<Value, String> {
    let id = args[0].as_int()? as usize;
    let size = args[1].as_int()? as usize; // Nombre d'octets à lire

    let mut guard = STATE.lock().unwrap();
    let state = &mut guard.0;
    
    let stream = state.streams.get_mut(&id).ok_or("Invalid Stream ID")?;
    
    let mut buffer = vec![0; size];
    let bytes_read = stream.read(&mut buffer).map_err(|e| e.to_string())?;
    
    // On tronque si on a lu moins que prévu
    buffer.truncate(bytes_read);
    
    // Conversion en String (Aegis ne gère pas encore les Buffers bruts)
    // On remplace les caractères invalides pour ne pas crasher
    let s = String::from_utf8_lossy(&buffer).to_string();
    
    Ok(Value::String(s))
}

// Retourne les données brutes, parfait pour les images ou l'upload
fn sock_read_bytes(args: Vec<Value>) -> Result<Value, String> {
    let id = args[0].as_int()? as usize;
    let size = args[1].as_int()? as usize; 

    let mut guard = STATE.lock().unwrap();
    let state = &mut guard.0;
    
    let stream = state.streams.get_mut(&id).ok_or("Invalid Stream ID")?;
    
    let mut buffer = vec![0; size];
    let bytes_read = stream.read(&mut buffer).map_err(|e| e.to_string())?;
    
    // On garde uniquement ce qu'on a lu
    buffer.truncate(bytes_read);
    
    // On emballe dans Value::Bytes
    Ok(Value::Bytes(Rc::new(RefCell::new(buffer))))
}

// 5. WRITE
fn sock_write(args: Vec<Value>) -> Result<Value, String> {
    let id = args[0].as_int()? as usize;
    let content = &args[1];

    let mut guard = STATE.lock().unwrap();
    let state = &mut guard.0;
    
    let stream = state.streams.get_mut(&id).ok_or("Invalid Stream ID")?;
    
    let res = match content {
        Value::String(s) => stream.write_all(s.as_bytes()),
        
        // Support du type Bytes : on écrit le buffer brut
        Value::Bytes(b) => stream.write_all(&b.borrow()),
        
        // Fallback (ex: Integer) -> Convertir en string
        _ => stream.write_all(content.to_string().as_bytes()),
    };

    res.map_err(|e| e.to_string())?;
    
    Ok(Value::Null)
}

// 6. CLOSE
fn sock_close(args: Vec<Value>) -> Result<Value, String> {
    let id = args[0].as_int()? as usize;
    let mut guard = STATE.lock().unwrap();
    let state = &mut guard.0;
    
    // On essaie de retirer des deux maps
    state.listeners.remove(&id);
    state.streams.remove(&id);
    
    Ok(Value::Null)
}
