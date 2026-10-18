use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc;

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub device_name: String,
    pub port: u16,
    pub root_mode: String,
    pub root_path: String,
    pub private_mode: bool,
    pub private_files: Vec<String>,
    pub show_hidden: bool,
    pub restrict_modify: bool,
    pub require_approval: bool,
    pub password_hash: String,
    pub password_salt: String,
    pub session_hours: u64,
    pub https: bool,
    pub theme: String,
    pub portal_theme: String,
    pub plugin_development: bool,
    pub dev_token: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            device_name: std::env::var("COMPUTERNAME").unwrap_or("My desktop".into()),
            port: 6060,
            root_mode: "folder".into(),
            root_path: String::new(),
            private_mode: false,
            private_files: vec![],
            show_hidden: false,
            restrict_modify: true,
            require_approval: true,
            password_hash: String::new(),
            password_salt: String::new(),
            session_hours: 24,
            https: false,
            theme: "dark".into(),
            portal_theme: "default".into(),
            plugin_development: false,
            dev_token: String::new(),
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Drive {
    pub path: String,
    pub name: String,
    pub total: u64,
    pub free: u64,
    pub eligible: bool,
    pub protection: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub status: String,
    pub remembered: bool,
    pub last_seen: u64,
    pub expires: u64,
    #[serde(default)]
    pub plugins: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transfer {
    pub id: String,
    pub name: String,
    pub direction: String,
    pub size: u64,
    pub transferred: u64,
    pub status: String,
    pub device: String,
    pub timestamp: u64,
    pub elapsed_ms: u64,
    pub path: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub uid: String,
    pub name: String,
    pub package: String,
    pub description: String,
    pub author: String,
    pub version: String,
    #[serde(rename = "versionCode", default)]
    pub version_code: u64,
    pub enabled: bool,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub settings: Settings,
    pub devices: Vec<Device>,
    pub history: Vec<Transfer>,
    pub plugins: Vec<Plugin>,
    pub sent_total: u64,
    pub received_total: u64,
}
pub struct Peer {
    pub package: String,
    pub uuid: String,
    pub public: Value,
    pub tx: mpsc::UnboundedSender<Value>,
}
pub struct Runtime {
    pub saved: Saved,
    pub drives: Vec<Drive>,
    pub roots: Vec<(String, PathBuf)>,
    pub running: bool,
    pub started: u64,
    pub urls: Vec<String>,
    pub server: Option<axum_server::Handle>,
    pub jobs: HashMap<String, Value>,
    pub peers: HashMap<String, Peer>,
    pub login_attempts: HashMap<String, (u32, u64)>,
}
#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<Mutex<Runtime>>,
    pub data: PathBuf,
    pub portal: PathBuf,
}
impl AppState {
    pub fn new(data: PathBuf, portal: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&data);
        let saved = std::fs::read(data.join("state.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self {
            inner: Arc::new(Mutex::new(Runtime {
                saved,
                drives: vec![],
                roots: vec![],
                running: false,
                started: 0,
                urls: vec![],
                server: None,
                jobs: HashMap::new(),
                peers: HashMap::new(),
                login_attempts: HashMap::new(),
            })),
            data,
            portal,
        }
    }
    pub fn persist(&self) -> Result<(), String> {
        let r = self.inner.lock().unwrap();
        let bytes = serde_json::to_vec_pretty(&r.saved).map_err(|e| e.to_string())?;
        let temp = self.data.join("state.tmp");
        std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
        std::fs::rename(temp, self.data.join("state.json")).map_err(|e| e.to_string())
    }
    pub fn snapshot(&self) -> Value {
        let r = self.inner.lock().unwrap();
        let mut settings = serde_json::to_value(&r.saved.settings).unwrap();
        settings["passwordSet"] = json!(!r.saved.settings.password_hash.is_empty());
        settings.as_object_mut().unwrap().remove("passwordHash");
        settings.as_object_mut().unwrap().remove("passwordSalt");
        json!({"settings":settings,"drives":r.drives,"running":r.running,"started":r.started,"urls":r.urls,"devices":r.saved.devices,"history":r.saved.history,"plugins":r.saved.plugins,"totals":{"sent":r.saved.sent_total,"received":r.saved.received_total}})
    }
    pub fn begin_transfer(
        &self,
        name: &str,
        direction: &str,
        size: u64,
        device: &str,
        path: &str,
    ) -> String {
        let id = token();
        let mut r = self.inner.lock().unwrap();
        r.saved.history.insert(
            0,
            Transfer {
                id: id.clone(),
                name: name.into(),
                direction: direction.into(),
                size,
                transferred: 0,
                status: "active".into(),
                device: device.into(),
                timestamp: now(),
                elapsed_ms: 0,
                path: path.into(),
            },
        );
        r.saved.history.truncate(1000);
        id
    }
    pub fn progress(&self, id: &str, bytes: u64, done: Option<&str>) {
        let mut r = self.inner.lock().unwrap();
        let mut delta = 0;
        let mut sent = false;
        if let Some(t) = r.saved.history.iter_mut().find(|t| t.id == id) {
            delta = bytes.saturating_sub(t.transferred);
            sent = t.direction == "sent";
            t.transferred = bytes;
            t.elapsed_ms = now().saturating_sub(t.timestamp);
            if let Some(s) = done {
                t.status = s.into();
            }
        }
        if sent {
            r.saved.sent_total += delta
        } else {
            r.saved.received_total += delta
        }
        drop(r);
        if done.is_some() {
            let _ = self.persist();
        }
    }
}
