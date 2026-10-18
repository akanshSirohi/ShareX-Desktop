use super::*;
use crate::storage::*;
use std::{fs, io::Write, path::Path};

fn fixture() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("shared");
    fs::create_dir(&root).unwrap();
    let state = AppState::new(
        dir.path().join("app"),
        Path::new(env!("CARGO_MANIFEST_DIR")).join("portal"),
    );
    {
        let mut r = state.inner.lock().unwrap();
        r.saved.settings.root_path = root.to_string_lossy().into();
        r.saved.settings.restrict_modify = false;
        r.roots = vec![("Storage".into(), root)];
    }
    (dir, state)
}
fn free_server_port() -> u16 {
    // Both the HTTP port and the adjacent plugin socket port must be available.
    loop {
        let http = std::net::TcpListener::bind("0.0.0.0:0").unwrap();
        let port = http.local_addr().unwrap().port();
        if port < 65535 && std::net::TcpListener::bind(("0.0.0.0", port + 1)).is_ok() {
            return port;
        }
    }
}
fn operation(state: &AppState, params: &[(&str, &str)]) -> Result<Value, String> {
    server::operation(
        state,
        &params
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    )
}
#[test]
fn roots_are_contained_and_roots_cannot_be_modified() {
    let (_dir, state) = fixture();
    let root = Path::new(&state.inner.lock().unwrap().saved.settings.root_path).to_path_buf();
    fs::write(root.join("file.txt"), "hello").unwrap();
    assert!(resolve(&state, "file.txt").is_ok());
    for p in [
        "../outside",
        "C:/Windows",
        "/../secret",
        "folder/../../secret",
    ] {
        assert!(resolve(&state, p).is_err(), "{p}")
    }
    assert!(protect_root(&state, &fs::canonicalize(root).unwrap()).is_err());
}
#[test]
fn drive_selection_excludes_bitlocker_and_unknown() {
    let (_dir, state) = fixture();
    let path = state.inner.lock().unwrap().saved.settings.root_path.clone();
    let good = Drive {
        path: path.clone(),
        name: "safe".into(),
        total: 100,
        free: 50,
        eligible: true,
        protection: "Not encrypted".into(),
    };
    let mut protected = good.clone();
    protected.path = "Z:\\".into();
    protected.eligible = false;
    protected.protection = "BitLocker protected".into();
    let mut unknown = protected.clone();
    unknown.path = "Y:\\".into();
    unknown.protection = "Unknown".into();
    let mut settings = Settings {
        root_mode: "all".into(),
        ..Settings::default()
    };
    let roots = prepare_roots(&settings, &[good.clone(), protected.clone(), unknown]).unwrap();
    assert_eq!(roots.len(), 1);
    assert!(prepare_roots(&settings, &[protected]).is_err());
    settings.root_mode = "folder".into();
    settings.root_path = path;
    let mut denied = good;
    denied.eligible = false;
    assert!(prepare_roots(&settings, &[denied]).is_err());
}
#[test]
fn windows_file_names_are_validated() {
    for name in [
        "..",
        ".",
        "CON",
        "NUL.txt",
        "COM1.txt",
        "LPT9",
        "a:b",
        "a/b",
        "a\\b",
        "trailing.",
        "trailing ",
        "bad\0",
    ] {
        assert!(safe_name(name).is_err(), "{name}")
    }
    for name in ["notes.txt", "my photos", "résumé.pdf"] {
        assert!(safe_name(name).is_ok())
    }
}
#[test]
fn hidden_private_and_modify_restrictions() {
    let (_dir, state) = fixture();
    let root = Path::new(&state.inner.lock().unwrap().saved.settings.root_path).to_path_buf();
    fs::write(root.join("public.txt"), "yes").unwrap();
    fs::write(root.join(".secret"), "no").unwrap();
    assert_eq!(
        listing(&state, "").unwrap()["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    {
        let mut r = state.inner.lock().unwrap();
        r.saved.settings.show_hidden = true;
    }
    assert_eq!(
        listing(&state, "").unwrap()["items"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    {
        let mut r = state.inner.lock().unwrap();
        r.saved.settings.private_mode = true;
        r.saved.settings.private_files = vec![root.join("public.txt").to_string_lossy().into()];
    }
    assert_eq!(
        listing(&state, "").unwrap()["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(resolve(&state, "public.txt").is_err());
    assert!(resolve(&state, "private/0").is_ok());
    assert!(resolve(&state, "private/1").is_err());
    assert!(operation(&state, &[("action", "newF"), ("name", "Denied")]).is_err());
}
#[test]
fn file_operations_refuse_collisions_and_root_deletion() {
    let (_dir, state) = fixture();
    operation(&state, &[("action", "newF"), ("name", "Docs")]).unwrap();
    assert!(operation(&state, &[("action", "newF"), ("name", "Docs")]).is_err());
    operation(
        &state,
        &[("action", "renF"), ("old_n", "Docs"), ("new_n", "Renamed")],
    )
    .unwrap();
    assert!(resolve(&state, "Renamed").is_ok());
    assert!(operation(&state, &[("action", "delFiles"), ("data", "[\"\"]")]).is_err());
    operation(&state, &[("action", "delFiles"), ("data", "[\"Renamed\"]")]).unwrap();
    assert!(resolve(&state, "Renamed").is_err());
}
#[test]
fn copy_does_not_overwrite_or_follow_links() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let dest = dir.path().join("dest");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("item.txt"), "abc").unwrap();
    assert_eq!(copy_tree(&source, &dest).unwrap(), 3);
    assert!(copy_tree(&source, &dest).is_err());
}
#[test]
fn upload_commit_refuses_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let temp = dir.path().join("temp");
    let target = dir.path().join("target");
    fs::write(&temp, "incoming").unwrap();
    fs::write(&target, "original").unwrap();
    assert!(commit_upload(&temp, &target).is_err());
    assert_eq!(fs::read_to_string(&target).unwrap(), "original");
    fs::remove_file(&target).unwrap();
    commit_upload(&temp, &target).unwrap();
    assert_eq!(fs::read_to_string(target).unwrap(), "incoming");
}
#[test]
fn plugin_database_round_trip_and_namespace_isolation() {
    let (_dir, state) = fixture();
    let action = |pkg, act, data: Value| plugins::storage_action(&state, pkg, act, &data);
    assert_eq!(
        action(
            "test.plugin",
            "db_action_init_db",
            serde_json::json!({"db_name":"notes"})
        )["data"],
        "success"
    );
    let result = action(
        "test.plugin",
        "db_action_insert_data",
        serde_json::json!({"db_name":"notes","collection":"notes","new_data":{"text":"Hello"}}),
    );
    let parsed: Value = serde_json::from_str(result["data"].as_str().unwrap()).unwrap();
    assert_eq!(parsed["status"], "success");
    let result = action(
        "test.plugin",
        "db_action_get_all_data",
        serde_json::json!({"db_name":"notes","collection":"notes"}),
    );
    let parsed: Value = serde_json::from_str(result["data"].as_str().unwrap()).unwrap();
    assert_eq!(parsed["data"][0]["text"], "Hello");
    let other = action(
        "other.plugin",
        "read_json_file",
        serde_json::json!({"filename":"notes.json"}),
    );
    assert_eq!(other["data"], "{}");
    assert!(
        action(
            "test.plugin",
            "create_json_file",
            serde_json::json!({"filename":"../escape","data":{}})
        )["result"]
            != true
    );
}
#[test]
fn plugins_validate_archive_paths_and_keep_data_on_update() {
    let (dir, state) = fixture();
    let archive = dir.path().join("plugin.zip");
    let f = fs::File::create(&archive).unwrap();
    let mut z = zip::ZipWriter::new(f);
    let opt = zip::write::SimpleFileOptions::default();
    z.start_file("config.json", opt).unwrap();
    z.write_all(br#"{"name":"Test","package":"sharex.test.plugin","version":"1.0.0"}"#)
        .unwrap();
    z.start_file("index.html", opt).unwrap();
    z.write_all(b"<h1>Test</h1>").unwrap();
    z.finish().unwrap();
    let p = plugins::install(&state, &archive).unwrap();
    assert_eq!(p.package, "sharex.test.plugin");
    assert!(state
        .data
        .join("plugins")
        .join(p.uid)
        .join("index.html")
        .exists());
    let f = fs::File::create(&archive).unwrap();
    let mut z = zip::ZipWriter::new(f);
    z.start_file("config.json", opt).unwrap();
    z.write_all(br#"{"name":"Bad","package":"sharex.bad.plugin"}"#)
        .unwrap();
    z.start_file("../escape.txt", opt).unwrap();
    z.write_all(b"bad").unwrap();
    z.finish().unwrap();
    assert!(plugins::install(&state, &archive).is_err());
    assert!(!state.data.join("escape.txt").exists());
}
#[tokio::test]
async fn real_server_auth_stream_upload_range_and_restrictions() {
    let (_dir, state) = fixture();
    let root = Path::new(&state.inner.lock().unwrap().saved.settings.root_path).to_path_buf();
    fs::write(root.join("hello.txt"), "hello world").unwrap();
    let port = free_server_port();
    state.inner.lock().unwrap().saved.settings.port = port;
    server::start(state.clone()).await.unwrap();
    let base = format!("http://127.0.0.1:{port}");
    let c = reqwest::Client::new();
    let auth = c
        .get(format!("{base}/ShareX?action=auth"))
        .send()
        .await
        .unwrap();
    let cookie = auth.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    assert_eq!(auth.text().await.unwrap(), "waiting");
    assert_eq!(
        c.get(format!("{base}/ShareX?action=listFiles"))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    {
        let mut r = state.inner.lock().unwrap();
        r.saved.devices[0].status = "approved".into();
    }
    let list: Value = c
        .get(format!("{base}/ShareX?action=listFiles"))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(list["items"][0]["name"], "hello.txt");
    let partial = c
        .get(format!(
            "{base}/ShareX?action=previewFile&location=hello.txt"
        ))
        .header("cookie", &cookie)
        .header("range", "bytes=0-4")
        .send()
        .await
        .unwrap();
    assert_eq!(partial.status(), 206);
    assert_eq!(partial.text().await.unwrap(), "hello");
    let download = c
        .get(format!("{base}/ShareX?action=openFile&location=hello.txt"))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(download.text().await.unwrap(), "hello world");
    let boundary = "ShareXTestBoundary";
    let body=format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file_0\"; filename=\"received.txt\"\r\nContent-Type: text/plain\r\n\r\nreceived content\r\n--{boundary}--\r\n");
    let upload = c
        .post(format!("{base}/ShareX/uploadFile?location="))
        .header("cookie", &cookie)
        .header(
            "Content-Type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(body)
        .send()
        .await
        .unwrap();
    assert!(
        upload.status().is_success(),
        "{}",
        upload.text().await.unwrap()
    );
    assert_eq!(
        fs::read_to_string(root.join("received.txt")).unwrap(),
        "received content"
    );
    let single = c
        .get(format!("{base}/ShareX"))
        .query(&[("action", "downloadFiles"), ("data", "[\"hello.txt\"]")])
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(single.text().await.unwrap(), "hello world");
    let archive = c
        .get(format!("{base}/ShareX"))
        .query(&[
            ("action", "downloadFiles"),
            ("data", "[\"hello.txt\",\"received.txt\"]"),
            ("filename", "My files"),
        ])
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(
        archive.headers()["content-disposition"],
        "attachment; filename=\"My files.zip\""
    );
    let bytes = archive.bytes().await.unwrap();
    let zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    assert_eq!(zip.len(), 2);
    let cross = c
        .get(format!(
            "{base}/ShareX?action=delFiles&data=%5B%22hello.txt%22%5D"
        ))
        .header("cookie", &cookie)
        .header("origin", "https://evil.example")
        .send()
        .await
        .unwrap();
    assert_eq!(cross.status(), 403);
    assert!(root.join("hello.txt").exists());
    state.inner.lock().unwrap().saved.settings.restrict_modify = true;
    let denied = c
        .get(format!("{base}/ShareX?action=newF&name=Denied"))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert!(!denied.status().is_success());
    {
        let r = state.inner.lock().unwrap();
        assert!(r
            .saved
            .history
            .iter()
            .any(|t| t.direction == "sent" && t.status == "completed"));
        assert!(r
            .saved
            .history
            .iter()
            .any(|t| t.direction == "received" && t.status == "completed"));
    }
    server::stop(&state);
    assert!(!state.inner.lock().unwrap().running);
}

#[tokio::test]
async fn https_password_and_revocation() {
    let (_dir, state) = fixture();
    let port = free_server_port();
    {
        let mut r = state.inner.lock().unwrap();
        r.saved.settings.port = port;
        r.saved.settings.https = true;
        r.saved.settings.password_salt = token();
        r.saved.settings.password_hash =
            server::password_hash("test-password", &r.saved.settings.password_salt);
    }
    server::start(state.clone()).await.unwrap();
    let c = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
    let base = format!("https://127.0.0.1:{port}");
    let auth = c
        .get(format!("{base}/ShareX?action=auth"))
        .send()
        .await
        .unwrap();
    let cookie = auth.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    assert_eq!(auth.text().await.unwrap(), "password");
    assert_eq!(
        c.post(format!("{base}/ShareX/password"))
            .header("cookie", &cookie)
            .json(&serde_json::json!({"password":"wrong"}))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert!(c
        .post(format!("{base}/ShareX/password"))
        .header("cookie", &cookie)
        .json(&serde_json::json!({"password":"test-password"}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    assert!(c
        .get(format!("{base}/ShareX?action=getState"))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    state.inner.lock().unwrap().saved.devices[0].status = "denied".into();
    assert_eq!(
        c.get(format!("{base}/ShareX?action=getState"))
            .header("cookie", cookie)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    server::stop(&state);
}

#[tokio::test]
async fn direct_plugin_link_registers_browser_and_waits_for_both_approvals() {
    for password_required in [false, true] {
        let (_dir, state) = fixture();
        let port = free_server_port();
        let plugin_dir = state.data.join("plugins/sharex-test-plugin");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("index.html"),
            "<h1>Private plugin content</h1>",
        )
        .unwrap();
        {
            let mut r = state.inner.lock().unwrap();
            r.saved.settings.port = port;
            if password_required {
                r.saved.settings.password_salt = token();
                r.saved.settings.password_hash =
                    server::password_hash("test-password", &r.saved.settings.password_salt);
            }
            r.saved.plugins.push(Plugin {
                uid: "sharex-test-plugin".into(),
                package: "sharex.test.plugin".into(),
                name: "Test".into(),
                description: String::new(),
                author: String::new(),
                version: "1".into(),
                version_code: 1,
                enabled: true,
            });
        }
        server::start(state.clone()).await.unwrap();
        let base = format!("http://127.0.0.1:{port}");
        let url = format!("{base}/SharexApp/sharex-test-plugin/");
        let c = reqwest::Client::new();
        let cross_origin = c
            .get(&url)
            .header("Origin", "http://untrusted.example")
            .send()
            .await
            .unwrap();
        assert_eq!(cross_origin.status(), 403);
        assert!(state.snapshot()["devices"].as_array().unwrap().is_empty());
        let first = c.get(&url).send().await.unwrap();
        assert_eq!(first.status(), 200);
        assert_eq!(first.headers()["cache-control"], "no-store");
        let cookie = first.headers()["set-cookie"]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string();
        let page = first.text().await.unwrap();
        assert!(page.contains("Waiting for browser approval"));
        assert!(page.contains("/ShareX/password"));
        assert!(!page.contains("Private plugin content"));
        let devices = state.snapshot()["devices"].as_array().unwrap().clone();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0]["status"], "pending");
        assert_eq!(devices[0]["ip"], "127.0.0.1");
        // Reloads and polling must keep one request and one browser identity.
        c.get(&url).header("cookie", &cookie).send().await.unwrap();
        assert_eq!(state.snapshot()["devices"].as_array().unwrap().len(), 1);
        let permission =
            format!("{base}/ShareX?action=pluginPermission&package=sharex.test.plugin");
        assert_eq!(
            c.get(&permission)
                .header("cookie", &cookie)
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        if password_required {
            assert_eq!(
                c.get(format!("{base}/ShareX?action=auth"))
                    .header("cookie", &cookie)
                    .send()
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap(),
                "password"
            );
            assert!(c
                .post(format!("{base}/ShareX/password"))
                .header("cookie", &cookie)
                .json(&serde_json::json!({"password":"test-password"}))
                .send()
                .await
                .unwrap()
                .status()
                .is_success());
        } else {
            state.inner.lock().unwrap().saved.devices[0].status = "approved".into();
        }
        let waiting = c
            .get(&url)
            .header("cookie", &cookie)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(!waiting.contains("Private plugin content"));
        assert_eq!(
            state.snapshot()["devices"][0]["plugins"],
            serde_json::json!(["pending:sharex.test.plugin"])
        );
        // Plugin requests persist and repeated permission checks do not duplicate them.
        assert_eq!(
            c.get(&permission)
                .header("cookie", &cookie)
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap(),
            "waiting"
        );
        let saved: Saved =
            serde_json::from_slice(&fs::read(state.data.join("state.json")).unwrap()).unwrap();
        assert_eq!(saved.devices[0].plugins, vec!["pending:sharex.test.plugin"]);
        state.inner.lock().unwrap().saved.devices[0].plugins = vec!["sharex.test.plugin".into()];
        assert_eq!(
            c.get(&url)
                .header("cookie", &cookie)
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap(),
            "<h1>Private plugin content</h1>"
        );
        state.inner.lock().unwrap().saved.devices[0].status = "denied".into();
        let denied = c
            .get(&url)
            .header("cookie", &cookie)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(!denied.contains("Private plugin content"));
        assert_eq!(state.snapshot()["devices"][0]["status"], "denied");
        server::stop(&state);
    }
}

#[tokio::test]
async fn plugin_websocket_sdk_presence_storage_and_key_revocation() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};
    let (_dir, state) = fixture();
    let port = free_server_port();
    let browser = token();
    let key = token();
    {
        let mut r = state.inner.lock().unwrap();
        r.saved.settings.port = port;
        r.saved.settings.plugin_development = true;
        r.saved.settings.dev_token = key.clone();
        r.saved.plugins.push(Plugin {
            uid: "sharex-test-plugin".into(),
            package: "sharex.test.plugin".into(),
            name: "Test".into(),
            description: String::new(),
            author: String::new(),
            version: "1".into(),
            version_code: 1,
            enabled: true,
        });
        r.saved.devices.push(Device {
            id: browser.clone(),
            name: "test".into(),
            ip: "127.0.0.1".into(),
            status: "approved".into(),
            remembered: true,
            last_seen: now(),
            expires: now() + 3600000,
            plugins: vec!["sharex.test.plugin".into()],
        });
    }
    server::start(state.clone()).await.unwrap();
    let mut request = format!("ws://127.0.0.1:{}/", port + 1)
        .into_client_request()
        .unwrap();
    request.headers_mut().insert(
        "Origin",
        format!("http://127.0.0.1:{port}").parse().unwrap(),
    );
    request.headers_mut().insert(
        "Cookie",
        format!("sharex_desktop={browser}").parse().unwrap(),
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    let uuid = uuid::Uuid::new_v4().to_string();
    socket.send(Message::Text(serde_json::json!({"action":"init_user","package_name":"sharex.test.plugin","data":{"uuid":uuid,"public_data":{"name":"Desktop test"}}}).to_string().into())).await.unwrap();
    socket
        .send(Message::Text(
            serde_json::json!({"action":"get_all_users"})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let result: Value = serde_json::from_str(result.to_text().unwrap()).unwrap();
    assert_eq!(result["all_users"][0]["uuid"], uuid);
    socket
        .send(Message::Text(
            serde_json::json!({"action":"db_action_init_db","data":{"db_name":"test"}})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let result = socket.next().await.unwrap().unwrap();
    let result: Value = serde_json::from_str(result.to_text().unwrap()).unwrap();
    assert_eq!(result["action"], "db_action_init_db_result");
    assert_eq!(result["data"], "success");
    socket.close(None).await.unwrap();
    let url = format!(
        "ws://127.0.0.1:{}/__sharex_dev?token={key}&package=sharex.test.plugin",
        port + 1
    );
    let (mut development, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    development.send(Message::Text(serde_json::json!({"action":"init_user","package_name":"dev.sharex.test.plugin","data":{"uuid":uuid::Uuid::new_v4().to_string(),"public_data":{}}}).to_string().into())).await.unwrap();
    state.inner.lock().unwrap().saved.settings.dev_token = token();
    let closed = tokio::time::timeout(std::time::Duration::from_secs(4), development.next()).await;
    assert!(closed.is_ok());
    assert!(tokio_tungstenite::connect_async(&url).await.is_err());
    server::stop(&state);
}
