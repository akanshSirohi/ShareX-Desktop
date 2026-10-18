mod model;
mod network;
mod plugins;
mod server;
mod storage;
use model::*;
use serde_json::Value;
use std::collections::HashMap;
use tauri::Manager;

#[tauri::command]
fn snapshot(state: tauri::State<AppState>) -> Value {
    state.snapshot()
}
#[tauri::command]
fn network_addresses() -> Vec<String> {
    network::addresses()
}
#[tauri::command]
async fn refresh_drives(state: tauri::State<'_, AppState>) -> Result<Value, String> {
    let drives = tauri::async_runtime::spawn_blocking(storage::scan_drives)
        .await
        .map_err(|e| e.to_string())??;
    state.inner.lock().unwrap().drives = drives;
    Ok(state.snapshot())
}
#[tauri::command]
async fn save_settings(
    state: tauri::State<'_, AppState>,
    settings: Value,
    password: Option<String>,
) -> Result<Value, String> {
    if state.inner.lock().unwrap().running {
        return Err("Stop sharing before changing settings".into());
    }
    let mut next: Settings = serde_json::from_value(settings).map_err(|e| e.to_string())?;
    {
        let r = state.inner.lock().unwrap();
        next.password_hash = r.saved.settings.password_hash.clone();
        next.password_salt = r.saved.settings.password_salt.clone();
        next.dev_token = r.saved.settings.dev_token.clone();
    }
    if next.device_name.trim().is_empty() || next.device_name.len() > 80 {
        return Err("Device name must contain 1–80 characters".into());
    }
    if next.port < 1024 || next.port == 65535 {
        return Err("Use a port between 1024 and 65534".into());
    }
    if ![1, 6, 12, 24, 48, 168].contains(&next.session_hours) {
        return Err("Choose a valid session duration".into());
    }
    if let Some(p) = password {
        if p.len() > 128 {
            return Err("Password exceeds 128 characters".into());
        }
        if !p.is_empty() && p.len() < 8 {
            return Err("Use at least 8 password characters".into());
        }
        next.password_salt = token();
        next.password_hash = if p.is_empty() {
            String::new()
        } else {
            server::password_hash(&p, &next.password_salt)
        };
    }
    if next.plugin_development {
        if next.dev_token.is_empty() {
            next.dev_token = token()
        }
    } else {
        next.dev_token.clear()
    }
    let drives = tauri::async_runtime::spawn_blocking(storage::scan_drives)
        .await
        .map_err(|e| e.to_string())??;
    let roots = if next.root_path.is_empty() && next.root_mode != "all" && !next.private_mode {
        vec![]
    } else {
        storage::prepare_roots(&next, &drives)?
    };
    {
        let mut r = state.inner.lock().unwrap();
        r.saved.settings = next;
        r.drives = drives;
        r.roots = roots;
    }
    state.persist()?;
    Ok(state.snapshot())
}
#[tauri::command]
async fn set_sharing(state: tauri::State<'_, AppState>, enabled: bool) -> Result<Value, String> {
    if enabled {
        server::start(state.inner().clone()).await?
    } else {
        server::stop(&state)
    }
    Ok(state.snapshot())
}
#[tauri::command]
async fn list_files(state: tauri::State<'_, AppState>, location: String) -> Result<Value, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || storage::listing(&state, &location))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn file_operation(
    state: tauri::State<'_, AppState>,
    parameters: HashMap<String, String>,
) -> Result<Value, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || server::operation(&state, &parameters))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn operation_status(state: tauri::State<AppState>, id: String) -> Result<Value, String> {
    state
        .inner
        .lock()
        .unwrap()
        .jobs
        .get(&id)
        .cloned()
        .ok_or("Operation not found".into())
}
#[tauri::command]
fn device_action(
    state: tauri::State<AppState>,
    id: String,
    action: String,
    remember: bool,
    package: Option<String>,
) -> Result<Value, String> {
    {
        let mut r = state.inner.lock().unwrap();
        let hours = r.saved.settings.session_hours;
        let d = r
            .saved
            .devices
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or("Device not found")?;
        match action.as_str() {
            "approve" => {
                d.status = "approved".into();
                d.remembered = remember;
                d.expires = now() + hours * 3600000
            }
            "revoke" => {
                d.status = "denied".into();
                d.remembered = false;
                d.plugins.clear()
            }
            "plugin" => {
                let p = package.ok_or("Missing plugin")?;
                d.plugins.retain(|s| s != &format!("pending:{p}"));
                if !d.plugins.contains(&p) {
                    d.plugins.push(p)
                }
            }
            "forget" => {
                r.saved.devices.retain(|d| d.id != id);
            }
            _ => return Err("Invalid device action".into()),
        };
    }
    state.persist()?;
    Ok(state.snapshot())
}
#[tauri::command]
fn clear_history(state: tauri::State<AppState>, id: Option<String>) -> Result<Value, String> {
    {
        let mut r = state.inner.lock().unwrap();
        r.saved
            .history
            .retain(|t| t.status == "active" || id.as_ref().is_some_and(|id| id != &t.id));
    }
    state.persist()?;
    Ok(state.snapshot())
}
#[tauri::command]
async fn install_plugin(
    state: tauri::State<'_, AppState>,
    path: Option<String>,
    package: Option<String>,
) -> Result<Value, String> {
    if let Some(p) = path {
        let state = state.inner().clone();
        tauri::async_runtime::spawn_blocking(move || {
            plugins::install(&state, std::path::Path::new(&p))
        })
        .await
        .map_err(|e| e.to_string())??;
    } else if let Some(p) = package {
        plugins::install_catalog(&state, &p).await?;
    } else {
        return Err("Choose a plugin ZIP or catalog package".into());
    }
    Ok(state.snapshot())
}
#[tauri::command]
async fn plugin_catalog() -> Result<Value, String> {
    plugins::catalog().await
}
#[tauri::command]
fn plugin_action(
    state: tauri::State<AppState>,
    uid: String,
    action: String,
) -> Result<Value, String> {
    let mut r = state.inner.lock().unwrap();
    let p = r
        .saved
        .plugins
        .iter_mut()
        .find(|p| p.uid == uid)
        .ok_or("Plugin not found")?;
    match action.as_str() {
        "toggle" => p.enabled = !p.enabled,
        "uninstall" => {
            let dir = state.data.join("plugins").join(&p.uid);
            std::fs::remove_dir_all(dir).map_err(|e| e.to_string())?;
            r.saved.plugins.retain(|p| p.uid != uid);
        }
        _ => return Err("Invalid action".into()),
    };
    drop(r);
    state.persist()?;
    Ok(state.snapshot())
}
#[tauri::command]
fn reset_development_key(state: tauri::State<AppState>) -> Result<Value, String> {
    let mut r = state.inner.lock().unwrap();
    r.saved.settings.dev_token = if r.saved.settings.plugin_development {
        token()
    } else {
        String::new()
    };
    r.peers.retain(|_, p| !p.package.starts_with("dev."));
    drop(r);
    state.persist()?;
    Ok(state.snapshot())
}
#[tauri::command]
fn reveal_file(state: tauri::State<AppState>, location: String) -> Result<(), String> {
    let path = storage::resolve(&state, &location)?;
    tauri_plugin_opener::reveal_item_in_dir(path).map_err(|e| e.to_string())
}
#[tauri::command]
async fn read_preview(
    state: tauri::State<'_, AppState>,
    location: String,
) -> Result<Value, String> {
    let path = storage::resolve(&state, &location)?;
    tauri::async_runtime::spawn_blocking(move||{
        let meta=std::fs::metadata(&path).map_err(|e|e.to_string())?;
        if !meta.is_file(){return Err("Choose a file".into())}
        let mime=mime_guess::from_path(&path).first_or_octet_stream().to_string();
        let name=path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let ext=path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
        if mime.starts_with("image/")&&ext!="svg"&&meta.len()<=20*1024*1024{
            use base64::Engine;let bytes=std::fs::read(path).map_err(|e|e.to_string())?;
            Ok(serde_json::json!({"name":name,"kind":"image","content":format!("data:{mime};base64,{}",base64::engine::general_purpose::STANDARD.encode(bytes))}))
        }else if (mime.starts_with("text/")||["json","md","csv","rs","ts","tsx","jsx","log","toml","yaml","yml","xml","svg"].contains(&ext.as_str()))&&meta.len()<=2*1024*1024 {
            let text=std::fs::read_to_string(path).map_err(|_|"File is not UTF-8 text")?;
            Ok(serde_json::json!({"name":name,"kind":"text","content":text}))
        }else{Ok(serde_json::json!({"name":name,"kind":"browser","content":""}))}
    }).await.map_err(|e|e.to_string())?
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            let resource = app.path().resource_dir()?.join("portal");
            let portal = if resource.join("index.html").is_file() {
                resource
            } else {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("portal")
            };
            let state = AppState::new(data, portal);
            {
                let mut r = state.inner.lock().unwrap();
                r.saved.devices.retain(|d| d.remembered);
                for t in r.saved.history.iter_mut().filter(|t| t.status == "active") {
                    t.status = "interrupted".into()
                }
            }
            let state2 = state.clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(Ok(drives)) =
                    tauri::async_runtime::spawn_blocking(storage::scan_drives).await
                {
                    let mut r = state2.inner.lock().unwrap();
                    r.roots =
                        storage::prepare_roots(&r.saved.settings, &drives).unwrap_or_default();
                    r.drives = drives;
                }
            });
            app.manage(state);
            use tauri::menu::{Menu, MenuItem};
            let show = MenuItem::with_id(app, "show", "Open ShareX", true, None::<&str>)?;
            let stop = MenuItem::with_id(app, "stop", "Stop sharing", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &stop, &quit])?;
            tauri::tray::TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("ShareX Desktop")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "stop" => server::stop(&app.state::<AppState>()),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            network_addresses,
            refresh_drives,
            save_settings,
            set_sharing,
            list_files,
            file_operation,
            operation_status,
            device_action,
            clear_history,
            install_plugin,
            plugin_catalog,
            plugin_action,
            reset_development_key,
            reveal_file,
            read_preview
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.state::<AppState>().inner.lock().unwrap().running {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("Failed to initialize ShareX")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                server::stop(&app.state::<AppState>());
            }
        });
}

#[cfg(test)]
mod tests;
