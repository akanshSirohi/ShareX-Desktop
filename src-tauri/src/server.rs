use crate::{model::*, storage::*};
use axum::{
    body::Body,
    extract::{ConnectInfo, DefaultBodyLimit, Multipart, Query, Request, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    time::Duration,
};
use subtle::ConstantTimeEq;
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};
type Params = HashMap<String, String>;
struct TransferGuard {
    state: AppState,
    id: String,
    bytes: u64,
    finished: bool,
    temp: Option<PathBuf>,
}
impl TransferGuard {
    fn update(&mut self, bytes: u64, status: Option<&str>) {
        self.bytes = bytes;
        self.finished = status.is_some();
        self.state.progress(&self.id, bytes, status);
    }
}
impl Drop for TransferGuard {
    fn drop(&mut self) {
        if !self.finished {
            self.state.progress(&self.id, self.bytes, Some("cancelled"));
        }
        if let Some(path) = &self.temp {
            let _ = std::fs::remove_file(path);
        }
    }
}
pub type ApiResult = Result<Response, (StatusCode, String)>;
pub fn bad(e: impl ToString) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, e.to_string())
}
fn param<'a>(p: &'a Params, key: &str) -> &'a str {
    p.get(key).map(String::as_str).unwrap_or("")
}
pub fn password_hash(password: &str, salt: &str) -> String {
    let mut h = Sha256::new();
    h.update(salt);
    h.update(password);
    format!("{:x}", h.finalize())
}
fn cookie(headers: &HeaderMap) -> String {
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .find_map(|v| v.trim().strip_prefix("sharex_desktop="))
        .unwrap_or("")
        .to_string()
}
fn authorized(state: &AppState, headers: &HeaderMap) -> bool {
    let id = cookie(headers);
    let r = state.inner.lock().unwrap();
    r.running
        && r.saved
            .devices
            .iter()
            .any(|d| d.id == id && d.status == "approved" && (d.remembered || d.expires > now()))
}
fn origin_ok(headers: &HeaderMap) -> bool {
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if headers.get("sec-fetch-site").and_then(|v| v.to_str().ok()) == Some("cross-site") {
        return false;
    }
    if let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        return origin == format!("http://{host}") || origin == format!("https://{host}");
    }
    if let Some(referer) = headers.get(header::REFERER).and_then(|v| v.to_str().ok()) {
        return referer.starts_with(&format!("http://{host}/"))
            || referer.starts_with(&format!("https://{host}/"));
    }
    true
}
pub fn require_auth(state: &AppState, headers: &HeaderMap) -> Result<(), (StatusCode, String)> {
    if !origin_ok(headers) {
        return Err((
            StatusCode::FORBIDDEN,
            "Cross-origin requests are blocked".into(),
        ));
    }
    if !authorized(state, headers) {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Browser access requires approval".into(),
        ));
    }
    Ok(())
}
fn browser_name(headers: &HeaderMap) -> String {
    let ua = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let browser = if ua.contains("Edg/") {
        "Edge"
    } else if ua.contains("Firefox/") {
        "Firefox"
    } else if ua.contains("Chrome/") {
        "Chrome"
    } else if ua.contains("Safari/") {
        "Safari"
    } else {
        "Browser"
    };
    let os = if ua.contains("Android") {
        "Android"
    } else if ua.contains("iPhone") {
        "iPhone"
    } else if ua.contains("Windows") {
        "Windows"
    } else if ua.contains("Mac") {
        "macOS"
    } else {
        "device"
    };
    format!("{browser} on {os}")
}
pub(crate) async fn auth(state: &AppState, headers: &HeaderMap, addr: SocketAddr) -> ApiResult {
    if !origin_ok(headers) {
        return Err((StatusCode::FORBIDDEN, "Cross-origin request".into()));
    }
    let supplied = cookie(headers);
    let mut id = supplied.clone();
    let answer;
    {
        let mut r = state.inner.lock().unwrap();
        let s = r.saved.settings.clone();
        let existing = r.saved.devices.iter().position(|d| d.id == id);
        let index = if let Some(i) = existing {
            i
        } else {
            id = token();
            if r.saved.devices.len() >= 256 {
                r.saved
                    .devices
                    .retain(|d| d.remembered || d.status == "approved");
                if r.saved.devices.len() >= 256 {
                    return Err((StatusCode::TOO_MANY_REQUESTS, "Too many browsers".into()));
                }
            }
            r.saved.devices.push(Device {
                id: id.clone(),
                name: browser_name(headers),
                ip: addr.ip().to_string(),
                status: if !s.require_approval && s.password_hash.is_empty() {
                    "approved"
                } else {
                    "pending"
                }
                .into(),
                remembered: false,
                last_seen: now(),
                expires: now() + s.session_hours * 3600000,
                plugins: vec![],
            });
            r.saved.devices.len() - 1
        };
        let d = &mut r.saved.devices[index];
        d.last_seen = now();
        d.ip = addr.ip().to_string();
        if d.status == "approved" && !d.remembered && d.expires <= now() {
            d.status = "pending".into()
        }
        answer = if d.status == "denied" {
            "denied"
        } else if d.status == "approved" {
            "true"
        } else if !s.password_hash.is_empty() {
            "password"
        } else {
            "waiting"
        };
    }
    let _ = state.persist();
    let mut response = answer.into_response();
    if id != supplied {
        let secure = if state.inner.lock().unwrap().saved.settings.https {
            "; Secure"
        } else {
            ""
        };
        response.headers_mut().insert(
            header::SET_COOKIE,
            format!(
                "sharex_desktop={id}; Path=/; HttpOnly; SameSite=Strict; Max-Age=31536000{secure}"
            )
            .parse()
            .unwrap(),
        );
    }
    Ok(response)
}
async fn password(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> ApiResult {
    if !origin_ok(&headers) {
        return Err((StatusCode::FORBIDDEN, "Cross-origin request".into()));
    }
    let id = cookie(&headers);
    let pass = body["password"].as_str().unwrap_or("");
    let mut r = state.inner.lock().unwrap();
    let attempt = r
        .login_attempts
        .entry(addr.ip().to_string())
        .or_insert((0, now()));
    if now() - attempt.1 > 60000 {
        *attempt = (0, now())
    }
    if attempt.0 >= 5 {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "Too many attempts. Wait one minute.".into(),
        ));
    }
    attempt.0 += 1;
    let s = r.saved.settings.clone();
    let actual = password_hash(pass, &s.password_salt);
    if pass.len() > 128
        || s.password_hash.is_empty()
        || !bool::from(actual.as_bytes().ct_eq(s.password_hash.as_bytes()))
    {
        return Err((StatusCode::UNAUTHORIZED, "Incorrect password".into()));
    }
    let d = r
        .saved
        .devices
        .iter_mut()
        .find(|d| d.id == id)
        .ok_or((StatusCode::UNAUTHORIZED, "Refresh this page first".into()))?;
    if d.status == "denied" {
        return Err((StatusCode::FORBIDDEN, "Browser access revoked".into()));
    }
    d.status = "approved".into();
    d.expires = now() + s.session_hours * 3600000;
    drop(r);
    let _ = state.persist();
    Ok("Unlocked".into_response())
}
pub fn portal_state(state: &AppState) -> Value {
    let r = state.inner.lock().unwrap();
    let s = &r.saved.settings;
    let total: u64 = r
        .drives
        .iter()
        .filter(|d| d.eligible)
        .map(|d| d.total)
        .sum();
    let free: u64 = r.drives.iter().filter(|d| d.eligible).map(|d| d.free).sum();
    json!({"deviceName":s.device_name,"battery":-1,"charging":false,"privateMode":s.private_mode,"restrictModify":s.restrict_modify,"appsAllowed":false,"theme":s.portal_theme,"webVersion":"v4_8","storageTotal":total,"storageFree":free,"storageUsed":total-free,"connected":r.running,"sessionElapsedMs":now().saturating_sub(r.started)})
}
async fn api(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(p): Query<Params>,
    headers: HeaderMap,
    request: Request,
) -> ApiResult {
    if param(&p, "action") == "auth" {
        return auth(&state, &headers, addr).await;
    }
    require_auth(&state, &headers)?;
    match param(&p, "action") {
        "getState" => Ok(Json(portal_state(&state)).into_response()),
        "listFiles" => {
            Ok(Json(listing(&state, param(&p, "location")).map_err(bad)?).into_response())
        }
        "listFolders" => {
            let mut v = listing(&state, param(&p, "location")).map_err(bad)?;
            v["items"]
                .as_array_mut()
                .unwrap()
                .retain(|i| i["directory"] == true);
            Ok(Json(v).into_response())
        }
        "openFile" | "previewFile" | "viewImage" | "thumbImage" => {
            let path = resolve(&state, param(&p, "location")).map_err(bad)?;
            if !path.is_file() {
                return Err((StatusCode::NOT_FOUND, "File not found".into()));
            }
            let download = param(&p, "action") == "openFile";
            serve_download(state, path, request, download, addr.ip().to_string()).await
        }
        "downloadFiles" => download_zip(state, p, request, addr.ip().to_string()).await,
        "newF" | "renF" | "delFiles" | "transferFiles" => {
            let state2 = state.clone();
            let result = tokio::task::spawn_blocking(move || operation(&state2, &p))
                .await
                .map_err(bad)?
                .map_err(bad)?;
            Ok(Json(result).into_response())
        }
        "fileOperationStatus" => {
            let r = state.inner.lock().unwrap();
            Ok(Json(
                r.jobs
                    .get(param(&p, "id"))
                    .cloned()
                    .ok_or((StatusCode::NOT_FOUND, "Operation not found".into()))?,
            )
            .into_response())
        }
        "getInstalledPlugins" => Ok(Json(
            state
                .inner
                .lock()
                .unwrap()
                .saved
                .plugins
                .iter()
                .filter(|p| p.enabled)
                .cloned()
                .collect::<Vec<_>>(),
        )
        .into_response()),
        "listApps" => Ok(Json(json!({"items":[],"allowed":false})).into_response()),
        "getPrivateMode" => Ok(state
            .inner
            .lock()
            .unwrap()
            .saved
            .settings
            .private_mode
            .to_string()
            .into_response()),
        "getUploadLocation" => Ok("Shared storage".into_response()),
        "getInfo" => Ok("ShareX Desktop · Tauri v2".into_response()),
        "pluginPermission" => crate::plugins::permission(&state, &headers, &p),
        _ => Err((StatusCode::NOT_FOUND, "Unknown action".into())),
    }
}
pub fn operation(state: &AppState, p: &Params) -> Result<Value, String> {
    writable(state)?;
    match param(p, "action") {
        "newF" => {
            let parent = resolve(state, param(p, "parent"))?;
            let name = safe_name(param(p, "name"))?;
            std::fs::create_dir(parent.join(name)).map_err(|e| e.to_string())?;
            Ok(json!("Folder created"))
        }
        "renF" => {
            let old = resolve(state, param(p, "old_n"))?;
            protect_root(state, &old)?;
            let name = safe_name(param(p, "new_n"))?;
            let dest = old.parent().ok_or("Missing parent")?.join(name);
            if dest.exists() {
                return Err("Name already exists".into());
            }
            std::fs::rename(old, dest).map_err(|e| e.to_string())?;
            Ok(json!("Renamed successfully"))
        }
        "delFiles" => {
            let data: Vec<String> =
                serde_json::from_str(param(p, "data")).map_err(|e| e.to_string())?;
            for loc in &data {
                let path = resolve(state, loc)?;
                protect_root(state, &path)?;
            }
            for loc in &data {
                let path = resolve(state, loc)?;
                if path.is_dir() {
                    std::fs::remove_dir_all(path)
                } else {
                    std::fs::remove_file(path)
                }
                .map_err(|e| e.to_string())?
            }
            Ok(json!(format!("{} items deleted", data.len())))
        }
        "transferFiles" => {
            let data: Vec<String> =
                serde_json::from_str(param(p, "data")).map_err(|e| e.to_string())?;
            if data.is_empty() || data.len() > 500 {
                return Err("Select 1–500 items".into());
            }
            let mode = param(p, "mode");
            if !["copy", "move"].contains(&mode) {
                return Err("Invalid transfer mode".into());
            }
            let dest = resolve(state, param(p, "destination"))?;
            if !dest.is_dir() {
                return Err("Destination must be a folder".into());
            }
            let mut sources = vec![];
            for loc in data {
                let src = resolve(state, &loc)?;
                protect_root(state, &src)?;
                if dest.starts_with(&src) {
                    return Err("Cannot transfer a folder into itself".into());
                }
                let target = dest.join(src.file_name().ok_or("Invalid file")?);
                if target.exists() {
                    return Err("Destination already contains an item with this name".into());
                }
                sources.push((loc, src, target));
            }
            let id = token();
            let job = json!({"id":id,"state":"queued","total":0,"processed":0,"completed":[],"errors":[]});
            {
                let mut r = state.inner.lock().unwrap();
                if r.jobs
                    .values()
                    .filter(|j| j["state"] == "running" || j["state"] == "queued")
                    .count()
                    >= 2
                {
                    return Err("File operations are busy".into());
                }
                r.jobs
                    .retain(|_, j| j["state"] == "running" || j["state"] == "queued");
                r.jobs.insert(id.clone(), job.clone());
            }
            let state = state.clone();
            let moving = mode == "move";
            std::thread::spawn(move || {
                let mut completed = vec![];
                let mut errors = vec![];
                let mut processed = 0;
                state.inner.lock().unwrap().jobs.get_mut(&id).unwrap()["state"] = json!("running");
                for (loc, src, dst) in sources {
                    match copy_tree(&src, &dst) {
                        Ok(bytes) => {
                            processed += bytes;
                            if moving {
                                let result = if src.is_dir() {
                                    std::fs::remove_dir_all(&src)
                                } else {
                                    std::fs::remove_file(&src)
                                };
                                if let Err(e) = result {
                                    errors.push(e.to_string());
                                    continue;
                                }
                            }
                            completed.push(loc)
                        }
                        Err(e) => errors.push(e),
                    }
                }
                state.inner.lock().unwrap().jobs.insert(id.clone(),json!({"id":id,"state":if errors.is_empty(){"completed"}else{"failed"},"processed":processed,"total":processed,"completed":completed,"errors":errors}));
            });
            Ok(job)
        }
        _ => Err("Unknown file operation".into()),
    }
}
async fn serve_download(
    state: AppState,
    path: PathBuf,
    request: Request,
    download: bool,
    device: String,
) -> ApiResult {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let resp = ServeFile::new(&path).oneshot(request).await.map_err(bad)?;
    let (mut parts, body) = resp.into_parts();
    if download {
        let ascii: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || " ._-".contains(c) {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        parts.headers.insert(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{ascii}\"").parse().unwrap(),
        );
    }
    if !download {
        parts.headers.insert(header::CONTENT_SECURITY_POLICY,"sandbox; default-src 'none'; style-src 'unsafe-inline'; img-src data:; media-src 'self'".parse().unwrap());
    }
    parts
        .headers
        .insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    if !download || !parts.status.is_success() {
        return Ok(Response::from_parts(parts, Body::new(body)));
    }
    let size = parts
        .headers
        .get(header::CONTENT_LENGTH)
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    use futures_util::Stream;
    let id = state.begin_transfer(&name, "sent", size, &device, &path.to_string_lossy());
    let mut guard = TransferGuard {
        state: state.clone(),
        id: id.clone(),
        bytes: 0,
        finished: false,
        temp: None,
    };
    let mut stream = Box::pin(Body::new(body).into_data_stream());
    let mut sent = 0;
    let tracked = futures_util::stream::poll_fn(move |cx| match stream.as_mut().poll_next(cx) {
        std::task::Poll::Ready(Some(Ok(chunk))) => {
            sent += chunk.len() as u64;
            guard.update(
                sent,
                if sent >= size {
                    Some("completed")
                } else {
                    None
                },
            );
            std::task::Poll::Ready(Some(Ok::<_, std::io::Error>(chunk)))
        }
        std::task::Poll::Ready(Some(Err(e))) => {
            guard.update(sent, Some("failed"));
            std::task::Poll::Ready(Some(Err(std::io::Error::other(e))))
        }
        std::task::Poll::Ready(None) => {
            guard.update(sent, Some("completed"));
            std::task::Poll::Ready(None)
        }
        std::task::Poll::Pending => std::task::Poll::Pending,
    });
    Ok(Response::from_parts(parts, Body::from_stream(tracked)))
}
async fn upload(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(p): Query<Params>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> ApiResult {
    require_auth(&state, &headers)?;
    let settings = state.inner.lock().unwrap().saved.settings.clone();
    if settings.restrict_modify && !settings.private_mode {
        return Err((StatusCode::FORBIDDEN, "Uploads are restricted".into()));
    }
    let dest = if settings.private_mode {
        let path = state.data.join("Received");
        std::fs::create_dir_all(&path).map_err(bad)?;
        path
    } else {
        resolve(&state, param(&p, "location")).map_err(bad)?
    };
    let mut count = 0;
    while let Some(mut field) = multipart.next_field().await.map_err(bad)? {
        let name = field
            .file_name()
            .ok_or(bad("Missing filename"))?
            .to_string();
        safe_name(&name).map_err(bad)?;
        let target = dest.join(&name);
        if target.exists() {
            return Err((StatusCode::CONFLICT, format!("{name} already exists")));
        }
        let temp = dest.join(format!(".sharex-{}.part", token()));
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .await
            .map_err(bad)?;
        let id = state.begin_transfer(
            &name,
            "received",
            0,
            &addr.ip().to_string(),
            &target.to_string_lossy(),
        );
        let mut bytes = 0;
        let mut guard = TransferGuard {
            state: state.clone(),
            id: id.clone(),
            bytes: 0,
            finished: false,
            temp: Some(temp.clone()),
        };
        let result: Result<(), String> = async {
            use tokio::io::AsyncWriteExt;
            while let Some(chunk) = field.chunk().await.map_err(|e| e.to_string())? {
                bytes += chunk.len() as u64;
                guard.bytes = bytes;
                if bytes > 10 * 1024 * 1024 * 1024 {
                    return Err("File exceeds 10 GiB".into());
                }
                file.write_all(&chunk).await.map_err(|e| e.to_string())?;
                state.progress(&id, bytes, None)
            }
            file.flush().await.map_err(|e| e.to_string())?;
            Ok(())
        }
        .await;
        drop(file);
        if let Err(e) = result {
            let _ = tokio::fs::remove_file(&temp).await;
            state.progress(&id, bytes, Some("failed"));
            guard.finished = true;
            return Err(bad(e));
        }
        // Commit without replacing existing destinations, including on FAT/exFAT drives.
        if let Err(e) = commit_upload(&temp, &target) {
            let _ = tokio::fs::remove_file(&temp).await;
            state.progress(&id, bytes, Some("failed"));
            guard.finished = true;
            return Err(bad(e));
        }
        let _ = tokio::fs::remove_file(&temp).await;
        {
            let mut r = state.inner.lock().unwrap();
            if let Some(t) = r.saved.history.iter_mut().find(|t| t.id == id) {
                t.size = bytes
            }
        }
        state.progress(&id, bytes, Some("completed"));
        guard.finished = true;
        count += 1;
    }
    Ok(format!("{count} files uploaded").into_response())
}
async fn download_zip(state: AppState, p: Params, request: Request, device: String) -> ApiResult {
    let locations: Vec<String> = serde_json::from_str(param(&p, "data")).map_err(bad)?;
    if locations.is_empty() || locations.len() > 500 {
        return Err(bad("Select 1–500 files"));
    }
    let paths: Vec<_> = locations
        .iter()
        .map(|l| resolve(&state, l))
        .collect::<Result<_, _>>()
        .map_err(bad)?;
    if paths.len() == 1 && paths[0].is_file() {
        return serve_download(state, paths[0].clone(), request, true, device).await;
    }
    let requested_name = param(&p, "filename");
    let archive_name = if requested_name.is_empty() {
        "ShareX.zip".to_string()
    } else if requested_name.to_lowercase().ends_with(".zip") {
        requested_name.to_string()
    } else {
        format!("{requested_name}.zip")
    };
    safe_name(&archive_name).map_err(bad)?;
    let temp = state.data.join(format!("download-{}.zip", token()));
    let output = temp.clone();
    let private_data = std::fs::canonicalize(&state.data).map_err(bad)?;
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let f = std::fs::File::create(&output).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipWriter::new(f);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        let mut total = 0u64;
        for path in paths {
            let parent = path.parent().ok_or("Invalid path")?;
            for entry in walkdir::WalkDir::new(&path)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| !e.path().starts_with(&private_data))
            {
                let entry = entry.map_err(|e| e.to_string())?;
                let m = std::fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if m.file_attributes() & 0x400 != 0 {
                        return Err("Linked files are excluded".into());
                    }
                }
                if m.file_type().is_symlink() {
                    return Err("Linked files are excluded".into());
                }
                let n = entry
                    .path()
                    .strip_prefix(parent)
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                if m.is_dir() {
                    zip.add_directory(n, options).map_err(|e| e.to_string())?
                } else {
                    total += m.len();
                    if total > 10 * 1024 * 1024 * 1024 {
                        return Err("Archive exceeds 10 GiB. Download files individually.".into());
                    }
                    zip.start_file(n, options).map_err(|e| e.to_string())?;
                    let mut file = std::fs::File::open(entry.path()).map_err(|e| e.to_string())?;
                    std::io::copy(&mut file, &mut zip).map_err(|e| e.to_string())?;
                }
            }
        }
        zip.finish().map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(bad)?
    .map_err(bad)?;
    let mut response = serve_download(state.clone(), temp.clone(), request, true, device).await?;
    let ascii: String = archive_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || " ._-".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{ascii}\"").parse().unwrap(),
    );
    if let Some(t) = state
        .inner
        .lock()
        .unwrap()
        .saved
        .history
        .iter_mut()
        .find(|t| t.path == temp.to_string_lossy())
    {
        t.name = archive_name;
    }
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(3600)).await;
        let _ = tokio::fs::remove_file(temp).await;
    });
    Ok(response)
}
async fn static_portal(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    request: Request,
) -> ApiResult {
    if request.uri().path().starts_with("/SharexApp/") {
        return crate::plugins::serve_plugin(state, headers, request, addr).await;
    }
    let path = if request.uri().path() == "/" {
        "/index.html"
    } else {
        request.uri().path()
    };
    let mut req = Request::new(Body::empty());
    *req.uri_mut() = path.parse().map_err(bad)?;
    let response = ServeDir::new(&state.portal)
        .oneshot(req)
        .await
        .map_err(bad)?;
    Ok(response.map(Body::new))
}
pub async fn start(state: AppState) -> Result<(), String> {
    if state.inner.lock().unwrap().running {
        return Ok(());
    }
    let drives = tokio::task::spawn_blocking(scan_drives)
        .await
        .map_err(|e| e.to_string())??;
    let settings = state.inner.lock().unwrap().saved.settings.clone();
    if settings.port < 1024 || settings.port == 65535 {
        return Err("Use a port between 1024 and 65534".into());
    }
    let roots = prepare_roots(&settings, &drives)?;
    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, settings.port));
    let listener =
        std::net::TcpListener::bind(addr).map_err(|e| format!("Cannot start sharing: {e}"))?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let socket_listener =
        std::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, settings.port + 1)))
            .map_err(|e| format!("Plugin port unavailable: {e}"))?;
    socket_listener
        .set_nonblocking(true)
        .map_err(|e| e.to_string())?;
    let app = Router::new()
        .route("/ShareX", get(api))
        .route("/ShareX/password", post(password))
        .route("/ShareX/uploadFile", post(upload))
        .fallback(static_portal)
        .layer(DefaultBodyLimit::disable())
        .with_state(state.clone());
    let socket_app = crate::plugins::socket_router(state.clone());
    let handle = axum_server::Handle::new();
    let config = if settings.https {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let cert_path = state.data.join("certificate.pem");
        let key_path = state.data.join("certificate-key.pem");
        if !cert_path.exists() || !key_path.exists() {
            let mut names = vec!["localhost".into(), "127.0.0.1".into()];
            if let Ok(ip) = local_ip_address::local_ip() {
                names.push(ip.to_string())
            }
            let cert = rcgen::generate_simple_self_signed(names).map_err(|e| e.to_string())?;
            std::fs::write(&cert_path, cert.cert.pem()).map_err(|e| e.to_string())?;
            std::fs::write(&key_path, cert.signing_key.serialize_pem())
                .map_err(|e| e.to_string())?;
        }
        Some(
            axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path)
                .await
                .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };
    let mut ips = crate::network::addresses();
    if ips.is_empty() {
        ips.push("127.0.0.1".into())
    }
    {
        let mut r = state.inner.lock().unwrap();
        r.drives = drives;
        r.roots = roots;
        r.running = true;
        r.started = now();
        r.urls = ips
            .iter()
            .map(|ip| {
                format!(
                    "{}://{ip}:{}",
                    if settings.https { "https" } else { "http" },
                    settings.port
                )
            })
            .collect();
        r.server = Some(handle.clone());
        r.saved.devices.retain(|d| d.remembered);
    }
    let state2 = state.clone();
    let handle2 = handle.clone();
    let config2 = config.clone();
    tokio::spawn(async move {
        let result = if let Some(c) = config {
            axum_server::from_tcp_rustls(listener, c)
                .handle(handle)
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await
        } else {
            axum_server::from_tcp(listener)
                .handle(handle)
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await
        };
        if result.is_err() {
            stop(&state2)
        }
    });
    tokio::spawn(async move {
        if let Some(c) = config2 {
            let _ = axum_server::from_tcp_rustls(socket_listener, c)
                .handle(handle2)
                .serve(socket_app.into_make_service_with_connect_info::<SocketAddr>())
                .await;
        } else {
            let _ = axum_server::from_tcp(socket_listener)
                .handle(handle2)
                .serve(socket_app.into_make_service_with_connect_info::<SocketAddr>())
                .await;
        }
    });
    // Recheck protection and mount identities throughout this sharing session.
    let generation = state.inner.lock().unwrap().started;
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            let ended = {
                let r = state.inner.lock().unwrap();
                !r.running || r.started != generation
            };
            if ended {
                break;
            }
            let scan = tokio::task::spawn_blocking(scan_drives).await;
            let ended = {
                let r = state.inner.lock().unwrap();
                !r.running || r.started != generation
            };
            if ended {
                break;
            }
            let current = match scan {
                Ok(Ok(d)) => d,
                _ => {
                    stop(&state);
                    break;
                }
            };
            let (s, old) = {
                let r = state.inner.lock().unwrap();
                (r.saved.settings.clone(), r.roots.clone())
            };
            match prepare_roots(&s, &current) {
                Ok(new) if new == old => state.inner.lock().unwrap().drives = current,
                _ => {
                    stop(&state);
                    break;
                }
            }
        }
    });
    Ok(())
}
pub fn stop(state: &AppState) {
    let mut r = state.inner.lock().unwrap();
    if let Some(h) = r.server.take() {
        h.shutdown()
    }
    r.running = false;
    r.urls.clear();
    r.peers.clear();
    for t in r.saved.history.iter_mut().filter(|t| t.status == "active") {
        t.status = "cancelled".into()
    }
    drop(r);
    let _ = state.persist();
}
