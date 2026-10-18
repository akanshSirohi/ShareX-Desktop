use crate::{
    model::*,
    server::{bad, require_auth, ApiResult},
    storage::*,
};
use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, Request, State,
    },
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::{collections::HashMap, fs, io::Read, net::SocketAddr, path::Path, time::Duration};
use tower::ServiceExt;
type Params = HashMap<String, String>;
fn id(headers: &HeaderMap) -> String {
    headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .find_map(|v| v.trim().strip_prefix("sharex_desktop="))
        .unwrap_or("")
        .into()
}
fn valid_package(p: &str) -> bool {
    !p.is_empty()
        && p.len() <= 128
        && p.split('.')
            .all(|x| !x.is_empty() && x.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
}
pub fn permission(state: &AppState, headers: &HeaderMap, p: &Params) -> ApiResult {
    let package = p.get("package").ok_or(bad("Missing package"))?;
    let mut r = state.inner.lock().unwrap();
    if !r
        .saved
        .plugins
        .iter()
        .any(|x| x.package == *package && x.enabled)
    {
        return Err((StatusCode::NOT_FOUND, "Plugin unavailable".into()));
    }
    let d = r
        .saved
        .devices
        .iter_mut()
        .find(|d| d.id == id(headers))
        .ok_or(bad("Unknown browser"))?;
    let mut changed = false;
    let answer = if d.plugins.contains(package) {
        "true"
    } else {
        let pending = format!("pending:{package}");
        if !d.plugins.contains(&pending) {
            d.plugins.push(pending);
            changed = true;
        }
        "waiting"
    };
    drop(r);
    if changed {
        state.persist().map_err(bad)?;
    }
    Ok(answer.into_response())
}
pub async fn serve_plugin(
    state: AppState,
    headers: HeaderMap,
    request: Request,
    addr: SocketAddr,
) -> ApiResult {
    let path = request.uri().path().trim_start_matches("/SharexApp/");
    let mut parts = path.splitn(2, '/');
    let uid = parts.next().unwrap_or("");
    let rel = parts.next().unwrap_or("");
    let plugin = state
        .inner
        .lock()
        .unwrap()
        .saved
        .plugins
        .iter()
        .find(|p| p.uid == uid && p.enabled)
        .cloned()
        .ok_or((StatusCode::NOT_FOUND, "Plugin unavailable".into()))?;
    // A direct plugin link must register the browser just like the main portal.
    // Keep plugin files inaccessible until both approval stages are complete.
    let registration = crate::server::auth(&state, &headers, addr).await?;
    if require_auth(&state, &headers).is_err() {
        let mut response = approval_page(&plugin.package).into_response();
        if let Some(cookie) = registration.headers().get(axum::http::header::SET_COOKIE) {
            response
                .headers_mut()
                .insert(axum::http::header::SET_COOKIE, cookie.clone());
        }
        response.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            "no-store".parse().unwrap(),
        );
        return Ok(response);
    }
    let base = state.data.join("plugins").join(uid);
    let target = base.join(if rel.is_empty() { "index.html" } else { rel });
    let target = fs::canonicalize(target).map_err(bad)?;
    if !target.starts_with(fs::canonicalize(base).map_err(bad)?) {
        return Err(bad("Invalid plugin path"));
    }
    let granted = state
        .inner
        .lock()
        .unwrap()
        .saved
        .devices
        .iter()
        .any(|d| d.id == id(&headers) && d.plugins.contains(&plugin.package));
    if !granted {
        let _ = permission(
            &state,
            &headers,
            &HashMap::from([("package".into(), plugin.package.clone())]),
        );
        let mut response = approval_page(&plugin.package).into_response();
        response.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            "no-store".parse().unwrap(),
        );
        return Ok(response);
    }
    if target.extension().is_some_and(|x| x == "html") {
        let text = fs::read_to_string(target).map_err(bad)?;
        let text = text.replace("{% BASE_URL %}", &format!("/SharexApp/{}", plugin.uid));
        return Ok(Html(text).into_response());
    }
    let response = tower_http::services::ServeFile::new(target)
        .oneshot(request)
        .await
        .map_err(bad)?;
    Ok(response.map(Body::new))
}
fn approval_page(package: &str) -> Html<String> {
    Html(include_str!("plugin-approval.html").replace(
        "__SHAREX_PACKAGE__",
        &serde_json::to_string(package).unwrap(),
    ))
}
pub fn install(state: &AppState, path: &Path) -> Result<Plugin, String> {
    let f = fs::File::open(path).map_err(|e| e.to_string())?;
    if f.metadata().map_err(|e| e.to_string())?.len() > 100 * 1024 * 1024 {
        return Err("Plugin ZIP exceeds 100 MiB".into());
    }
    let mut archive = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    if archive.len() > 10000 {
        return Err("Too many plugin files".into());
    }
    let config: Value = {
        let mut file = archive
            .by_name("config.json")
            .map_err(|_| "ZIP needs config.json at root")?;
        if file.size() > 65536 {
            return Err("Plugin config too large".into());
        }
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())?
    };
    let package = config["package"].as_str().ok_or("Missing plugin package")?;
    if !valid_package(package) {
        return Err("Invalid plugin package".into());
    }
    let uid = package.replace('.', "-");
    let staging = state.data.join(format!("staging-{}", token()));
    fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        let mut total = 0u64;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
            total += entry.size();
            if total > 250 * 1024 * 1024 {
                return Err("Expanded plugin exceeds 250 MiB".into());
            }
            let relative = entry.enclosed_name().ok_or("Unsafe ZIP path")?;
            for c in relative.components() {
                if let std::path::Component::Normal(n) = c {
                    safe_name(&n.to_string_lossy())?;
                } else {
                    return Err("Unsafe ZIP path".into());
                }
            }
            if entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
                return Err("Plugin links are not allowed".into());
            }
            let target = staging.join(relative);
            if entry.is_dir() {
                fs::create_dir_all(target).map_err(|e| e.to_string())?
            } else {
                if let Some(p) = target.parent() {
                    fs::create_dir_all(p).map_err(|e| e.to_string())?
                }
                let mut f = fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(target)
                    .map_err(|e| e.to_string())?;
                std::io::copy(&mut entry.by_ref().take(250 * 1024 * 1024 + 1), &mut f)
                    .map_err(|e| e.to_string())?;
            }
        }
        if !staging.join("index.html").is_file() {
            return Err("ZIP needs index.html at root".into());
        }
        Ok(())
    })();
    if let Err(e) = result {
        let _ = fs::remove_dir_all(staging);
        return Err(e);
    }
    let dest = state.data.join("plugins").join(&uid);
    fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
    let backup = state.data.join(format!("backup-{}", token()));
    if dest.exists() {
        fs::rename(&dest, &backup).map_err(|e| e.to_string())?
    }
    if let Err(e) = fs::rename(&staging, &dest) {
        if backup.exists() {
            let _ = fs::rename(&backup, &dest);
        }
        return Err(e.to_string());
    }
    if backup.exists() {
        let _ = fs::remove_dir_all(backup);
    }
    let plugin = Plugin {
        uid,
        name: config["name"].as_str().unwrap_or(package).into(),
        package: package.into(),
        description: config["description"].as_str().unwrap_or("").into(),
        author: config["author"].as_str().unwrap_or("").into(),
        version: config["version"].as_str().unwrap_or("1.0.0").into(),
        version_code: config["versionCode"].as_u64().unwrap_or(1),
        enabled: true,
    };
    {
        let mut r = state.inner.lock().unwrap();
        r.saved.plugins.retain(|p| p.package != package);
        r.saved.plugins.push(plugin.clone());
    }
    state.persist()?;
    Ok(plugin)
}
pub async fn catalog() -> Result<Value, String> {
    reqwest::Client::new()
        .get("https://raw.githubusercontent.com/akanshSirohi/ShareX-Plugins/master/apps.json")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}
pub async fn install_catalog(state: &AppState, package: &str) -> Result<Plugin, String> {
    if !valid_package(package) {
        return Err("Invalid package".into());
    }
    let list = catalog().await?;
    if !list
        .as_array()
        .is_some_and(|l| l.iter().any(|p| p["package"] == package))
    {
        return Err("Plugin not in catalog".into());
    }
    let url=format!("https://raw.githubusercontent.com/akanshSirohi/ShareX-Plugins/master/{package}/sharex_dist/{package}.zip");
    let response = reqwest::Client::new()
        .get(url)
        .timeout(Duration::from_secs(90))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > 100 * 1024 * 1024 {
        return Err("Plugin archive exceeds 100 MiB".into());
    }
    let temp = state.data.join(format!("install-{}.zip", token()));
    fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    let result = install(state, &temp);
    let _ = fs::remove_file(temp);
    result
}
pub fn socket_router(state: AppState) -> Router {
    Router::new().fallback(get(upgrade)).with_state(state)
}
async fn upgrade(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<Params>,
    request: Request,
) -> ApiResult {
    let dev = if request.uri().path() == "/__sharex_dev" {
        let r = state.inner.lock().unwrap();
        let s = &r.saved.settings;
        let token = q.get("token").map(String::as_str).unwrap_or("");
        let p = q.get("package").map(String::as_str).unwrap_or("");
        use subtle::ConstantTimeEq;
        if !s.plugin_development
            || token.len() != 64
            || !bool::from(token.as_bytes().ct_eq(s.dev_token.as_bytes()))
            || !valid_package(p)
        {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Invalid development connection".into(),
            ));
        }
        Some(format!("dev.{p}"))
    } else {
        // A plugin page connects to the sharing port plus one.
        if let Some(origin) = headers.get("origin").and_then(|h| h.to_str().ok()) {
            let r = state.inner.lock().unwrap();
            let host = headers
                .get("host")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("")
                .rsplit_once(':')
                .map(|(h, _)| h)
                .unwrap_or("");
            let expected = format!(
                "{}://{host}:{}",
                if r.saved.settings.https {
                    "https"
                } else {
                    "http"
                },
                r.saved.settings.port
            );
            if origin != expected {
                return Err((StatusCode::FORBIDDEN, "Plugin origin is not allowed".into()));
            }
        }
        let mut identity = headers.clone();
        identity.remove("origin");
        identity.remove("referer");
        identity.remove("sec-fetch-site");
        require_auth(&state, &identity)?;
        None
    };
    let browser = id(&headers);
    let dev_key = q.get("token").cloned().unwrap_or_default();
    let ws = WebSocketUpgrade::from_request_parts(&mut request.into_parts().0, &state)
        .await
        .map_err(bad)?;
    Ok(ws
        .max_message_size(10 * 1024 * 1024)
        .on_upgrade(move |socket| connection(state, socket, browser, dev, dev_key))
        .into_response())
}
use axum::extract::FromRequestParts;
async fn connection(
    state: AppState,
    socket: WebSocket,
    browser: String,
    dev: Option<String>,
    dev_key: String,
) {
    let (mut sink, mut source) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Value>();
    let mut uuid = String::new();
    let mut package = String::new();
    let mut interval = tokio::time::interval(Duration::from_secs(2));
    loop {
        tokio::select! {
            _=interval.tick()=>{let r=state.inner.lock().unwrap();let valid=r.running&&if let Some(d)=&dev {r.saved.settings.plugin_development&&r.saved.settings.dev_token==dev_key&&(d==&package||package.is_empty())}else{r.saved.devices.iter().any(|d|d.id==browser&&d.status=="approved"&&(d.remembered||d.expires>now())&&(package.is_empty()||d.plugins.contains(&package)))&&(package.is_empty()||r.saved.plugins.iter().any(|p|p.package==package&&p.enabled))};if !valid{break}},
            Some(message)=rx.recv()=>{if sink.send(Message::Text(message.to_string().into())).await.is_err(){break}},
            incoming=source.next()=>{let Some(Ok(Message::Text(text)))=incoming else{break};let Ok(v)=serde_json::from_str::<Value>(&text)else{break};let action=v["action"].as_str().unwrap_or("");let data=&v["data"];
                if package.is_empty(){if action!="init_user"{break}let pkg=v["package_name"].as_str().unwrap_or("");let u=data["uuid"].as_str().unwrap_or("");if !valid_package(pkg)||uuid::Uuid::parse_str(u).is_err(){break}
                    let mut r=state.inner.lock().unwrap();let allowed=if let Some(d)=&dev {d==pkg}else{r.saved.plugins.iter().any(|p|p.package==pkg&&p.enabled)&&r.saved.devices.iter().any(|d|d.id==browser&&d.plugins.contains(&pkg.to_string()))};if !allowed||r.peers.contains_key(u){break}package=pkg.into();uuid=u.into();let public=data["public_data"].clone();for peer in r.peers.values().filter(|p|p.package==package){let _=peer.tx.send(json!({"action":"user_arrive","user":{"uuid":uuid,"public_data":public}}));}r.peers.insert(uuid.clone(),Peer{package:package.clone(),uuid:uuid.clone(),public,tx:tx.clone()});continue;
                }
                let response=match action{
                    "update_user_data"=>{if let Some(p)=state.inner.lock().unwrap().peers.get_mut(&uuid){p.public=data["public_data"].clone()}None},
                    "get_all_users"=>{let r=state.inner.lock().unwrap();Some(json!({"action":"return_all_users","all_users":r.peers.values().filter(|p|p.package==package).map(|p|json!({"uuid":p.uuid,"public_data":p.public})).collect::<Vec<_>>()}))},
                    "send_msg"=>{if let Some(peer)=state.inner.lock().unwrap().peers.get(data["uuid"].as_str().unwrap_or("")){if peer.package==package{let _=peer.tx.send(json!({"action":"msg_arrive","message":data["msg"]}));}}None},
                    "get_public_data_of_user"=>{state.inner.lock().unwrap().peers.get(data["uuid"].as_str().unwrap_or("")).filter(|p|p.package==package).map(|p|json!({"action":"return_public_data_of_user","public_data":p.public}))},
                    _=>Some(storage_action(&state,&package,action,data))
                };if let Some(response)=response{let _=tx.send(response);}
            }
        }
    }
    let mut r = state.inner.lock().unwrap();
    r.peers.remove(&uuid);
    for peer in r.peers.values().filter(|p| p.package == package) {
        let _ = peer.tx.send(json!({"action":"user_left","uuid":uuid}));
    }
}
pub(crate) fn storage_action(state: &AppState, package: &str, action: &str, data: &Value) -> Value {
    let _storage_lock = state.inner.lock().unwrap();
    let result = (|| -> Result<Value, String> {
        let folder = state.data.join("plugin-data").join(package);
        fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let filename = if action.starts_with("db_action_") {
            format!("{}.json", data["db_name"].as_str().unwrap_or(""))
        } else {
            data["filename"].as_str().unwrap_or("").into()
        };
        safe_name(&filename)?;
        let file = folder.join(filename);
        if action == "create_json_file" {
            if file.exists() {
                return Ok(json!(false));
            }
            fs::write(file, data["data"].to_string()).map_err(|e| e.to_string())?;
            return Ok(json!(true));
        }
        if action == "read_json_file" {
            return Ok(json!(fs::read_to_string(file).unwrap_or("{}".into())));
        }
        if action == "db_action_init_db" {
            if !file.exists() {
                fs::write(file, "{}").map_err(|e| e.to_string())?;
            }
            return Ok(json!("success"));
        }
        let mut db: Value = serde_json::from_slice(&fs::read(&file).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let collection = data["collection"].as_str().ok_or("Missing collection")?;
        if db.get(collection).is_none() {
            db[collection] = json!([])
        }
        let rows = db[collection].as_array_mut().ok_or("Invalid collection")?;
        let result = match action {
            "db_action_insert_data" | "db_action_insert_data_bulk" => {
                let new: Value = if let Some(s) = data["new_data"].as_str() {
                    serde_json::from_str(s).map_err(|e| e.to_string())?
                } else {
                    data["new_data"].clone()
                };
                let mut ids = vec![];
                let incoming = if action.ends_with("_bulk") {
                    new.as_array().cloned().ok_or("Invalid data")?
                } else {
                    vec![new]
                };
                for mut v in incoming {
                    if !v.is_object() {
                        return Err("Data must be an object".into());
                    }
                    if v.get("_uuid").is_none() {
                        v["_uuid"] = json!(uuid::Uuid::new_v4().to_string())
                    }
                    ids.push(v["_uuid"].clone());
                    rows.push(v)
                }
                json!({"status":"success","inserted_uuid":ids})
            }
            "db_action_get_all_data" => json!({"status":"success","data":rows}),
            "db_action_get_data" => {
                let array = json!(rows);
                let found = jsonpath_lib::select(&array, data["query"].as_str().unwrap_or("$[*]"))
                    .map_err(|e| e.to_string())?;
                json!({"status":"success","data":serde_json::to_string(&found).unwrap()})
            }
            "db_action_update_data" | "db_action_delete_data" => {
                let array = json!(rows);
                let found =
                    jsonpath_lib::select(&array, data["query"].as_str().ok_or("Missing query")?)
                        .map_err(|e| e.to_string())?;
                let selected: Vec<Value> = found.into_iter().cloned().collect();
                if action.ends_with("delete_data") {
                    rows.retain(|r| !selected.contains(r));
                } else {
                    for row in rows.iter_mut().filter(|r| selected.contains(r)) {
                        for update in data["update"].as_array().ok_or("Invalid update")? {
                            let (field, value) = update
                                .as_str()
                                .and_then(|s| s.split_once(':'))
                                .ok_or("Invalid update field")?;
                            let mut target = &mut *row;
                            let keys: Vec<_> = field.split('.').collect();
                            if keys.len() > 64 {
                                return Err("Update field nesting is too deep".into());
                            }
                            for (i, key) in keys.iter().enumerate() {
                                if !target.is_object() {
                                    *target = json!({})
                                }
                                if i == keys.len() - 1 {
                                    target[*key] = json!(value)
                                } else {
                                    if target.get(*key).is_none() {
                                        target[*key] = json!({})
                                    }
                                    target = &mut target[*key];
                                }
                            }
                        }
                    }
                }
                json!({"status":"success"})
            }
            _ => return Err("Unknown database action".into()),
        };
        let bytes = serde_json::to_vec(&db).map_err(|e| e.to_string())?;
        if bytes.len() > 10 * 1024 * 1024 {
            return Err("Plugin database exceeds 10 MiB".into());
        }
        fs::write(file, bytes).map_err(|e| e.to_string())?;
        Ok(json!(result.to_string()))
    })();
    let value = result.unwrap_or_else(|e| json!(json!({"status":"fail","message":e}).to_string()));
    if action == "create_json_file" {
        json!({"action":"return_create_json_file","result":value})
    } else if action == "read_json_file" {
        json!({"action":"return_read_json_file","data":value})
    } else {
        let suffix = if action == "db_action_insert_data_bulk" {
            "db_action_insert_data"
        } else {
            action
        };
        json!({"action":format!("{suffix}_result"),"data":value})
    }
}
