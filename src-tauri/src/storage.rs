use crate::model::*;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

pub fn hidden_command(program: &str) -> Command {
    let mut c = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000);
    }
    c
}
pub fn commit_upload(temp: &Path, target: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let from: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
        // MoveFileW, unlike MoveFileEx with REPLACE_EXISTING, never overwrites a target.
        if unsafe { windows_sys::Win32::Storage::FileSystem::MoveFileW(from.as_ptr(), to.as_ptr()) }
            == 0
        {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        std::fs::hard_link(temp, target)?;
        std::fs::remove_file(temp)
    }
}
pub fn scan_drives() -> Result<Vec<Drive>, String> {
    #[cfg(windows)]
    {
        // Query Windows' volume property without requesting elevation. Unknown status is excluded.
        let script = r#"$ErrorActionPreference='Stop'; $s=New-Object -ComObject Shell.Application; $result=@([System.IO.DriveInfo]::GetDrives() | ForEach-Object { $d=$_; $p=$null; try { $p=$s.NameSpace(17).ParseName($d.Name).ExtendedProperty('System.Volume.BitLockerProtection') } catch {}; $ready=$d.IsReady; $n=$d.Name; $t=0; $f=0; if($ready){ $n=$d.VolumeLabel; $t=$d.TotalSize; $f=$d.AvailableFreeSpace }; [pscustomobject]@{path=$d.Name;name=$n;total=$t;free=$f;eligible=($ready -and $null -ne $p -and [int]$p -eq 0 -and $d.DriveType -ne 'Network');protection=$(if($null -eq $p){'Unknown'}elseif([int]$p -eq 0){'Not encrypted'}else{'BitLocker protected'})} }); ConvertTo-Json -InputObject $result -Compress"#;
        let output = hidden_command("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(
                "Windows could not inspect drive protection. Refresh drives and try again.".into(),
            );
        }
        serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        Ok(vec![Drive {
            path: "/".into(),
            name: "Filesystem".into(),
            total: 0,
            free: 0,
            eligible: true,
            protection: "Not applicable".into(),
        }])
    }
}
pub fn prepare_roots(
    settings: &Settings,
    drives: &[Drive],
) -> Result<Vec<(String, PathBuf)>, String> {
    if settings.private_mode {
        for file in &settings.private_files {
            let p = fs::canonicalize(file).map_err(|e| e.to_string())?;
            check_drive(&p, drives)?;
            if !p.is_file() {
                return Err("Private selection must contain files".into());
            }
        }
        return Ok(vec![]);
    }
    if settings.root_mode == "all" {
        let roots: Vec<_> = drives
            .iter()
            .filter(|d| d.eligible)
            .map(|d| {
                (
                    d.path.trim_end_matches(['\\', '/']).replace(':', ""),
                    PathBuf::from(&d.path),
                )
            })
            .collect();
        if roots.is_empty() {
            return Err("No eligible unencrypted drives found".into());
        }
        return Ok(roots);
    }
    if !["drive", "folder"].contains(&settings.root_mode.as_str()) {
        return Err("Choose a valid root mode".into());
    }
    let p =
        fs::canonicalize(&settings.root_path).map_err(|_| "Choose an existing folder or drive")?;
    check_drive(&p, drives)?;
    if !p.is_dir() {
        return Err("Root must be a directory".into());
    }
    if settings.root_mode == "drive"
        && !drives
            .iter()
            .any(|d| fs::canonicalize(&d.path).ok().as_ref() == Some(&p))
    {
        return Err("Choose a drive root".into());
    }
    Ok(vec![("Storage".into(), p)])
}
fn check_drive(p: &Path, drives: &[Drive]) -> Result<(), String> {
    for d in drives {
        if let Ok(base) = fs::canonicalize(&d.path) {
            if p.starts_with(base) {
                return if d.eligible {
                    Ok(())
                } else {
                    Err(format!("{} is excluded: {}", d.path, d.protection))
                };
            }
        }
    }
    Err("Drive protection could not be verified. Network shares are excluded.".into())
}
pub fn safe_name(name: &str) -> Result<&str, String> {
    let base = name.split('.').next().unwrap_or("").to_uppercase();
    if name.is_empty()
        || name.len() > 240
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || [
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ]
        .contains(&base.as_str())
        || name == "."
        || name == ".."
    {
        return Err("Invalid file name".into());
    }
    Ok(name)
}
pub fn resolve(state: &AppState, location: &str) -> Result<PathBuf, String> {
    let r = state.inner.lock().unwrap();
    let s = &r.saved.settings;
    if s.private_mode {
        let index = location
            .strip_prefix("private/")
            .and_then(|v| v.parse::<usize>().ok())
            .ok_or("File is outside private selection")?;
        let selected = s
            .private_files
            .get(index)
            .ok_or("File is not shared")
            .and_then(|p| fs::canonicalize(p).map_err(|_| "File unavailable"))
            .map_err(str::to_string)?;
        if fs::canonicalize(&state.data).is_ok_and(|private| selected.starts_with(private)) {
            return Err("ShareX application data is private".into());
        }
        return Ok(selected);
    }
    let normalized = location.replace('\\', "/");
    let parts: Vec<_> = normalized.split('/').filter(|x| !x.is_empty()).collect();
    if parts
        .iter()
        .any(|s| *s == ".." || *s == "." || s.contains(':') || s.contains('\0'))
    {
        return Err("Path is outside shared root".into());
    }
    let (root, relative) = if s.root_mode == "all" {
        let drive = parts.first().ok_or("Choose a drive")?;
        let root = r
            .roots
            .iter()
            .find(|(n, _)| n == drive)
            .ok_or("Drive is not shared")?
            .1
            .clone();
        (root, parts[1..].join("/"))
    } else {
        (
            r.roots
                .first()
                .ok_or("Choose a shared root first")?
                .1
                .clone(),
            parts.join("/"),
        )
    };
    let base = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let result = fs::canonicalize(base.join(relative)).map_err(|e| e.to_string())?;
    if !result.starts_with(&base) {
        return Err("Path is outside shared root".into());
    }
    if fs::canonicalize(&state.data).is_ok_and(|private| result.starts_with(private)) {
        return Err("ShareX application data is private".into());
    }
    // Reject junctions, symlinks, and mounted volumes, including ones pointing back inside.
    let rel = result.strip_prefix(&base).map_err(|_| "Invalid path")?;
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err("Invalid path".into());
    }
    let mut cursor = base;
    for part in parts.iter().skip(if s.root_mode == "all" { 1 } else { 0 }) {
        cursor.push(part);
        let meta = fs::symlink_metadata(&cursor).map_err(|e| e.to_string())?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                return Err("Linked folders are excluded".into());
            }
        }
        if meta.file_type().is_symlink() {
            return Err("Linked folders are excluded".into());
        }
    }
    Ok(result)
}
pub fn is_hidden(path: &Path) -> bool {
    if path
        .file_name()
        .is_some_and(|n| n.to_string_lossy().starts_with('.'))
    {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if let Ok(m) = fs::metadata(path) {
            return m.file_attributes() & (2 | 4) != 0;
        }
    }
    false
}
pub fn info(path: &Path, location: &str) -> Result<Value, String> {
    let m = fs::metadata(path).map_err(|e| e.to_string())?;
    Ok(
        json!({"name":path.file_name().unwrap_or_default().to_string_lossy(),"location":location,"directory":m.is_dir(),"size":if m.is_dir(){0}else{m.len()},"modified":m.modified().ok().and_then(|d|d.duration_since(std::time::UNIX_EPOCH).ok()).map(|d|d.as_millis() as u64).unwrap_or(0),"hidden":is_hidden(path),"mime":if m.is_dir(){"inode/directory".to_string()}else{mime_guess::from_path(path).first_or_octet_stream().to_string()}}),
    )
}
pub fn listing(state: &AppState, location: &str) -> Result<Value, String> {
    let (s, roots) = {
        let r = state.inner.lock().unwrap();
        (r.saved.settings.clone(), r.roots.clone())
    };
    let mut items = vec![];
    if s.private_mode {
        for (i, p) in s.private_files.iter().enumerate() {
            if let Ok(v) = info(Path::new(p), &format!("private/{i}")) {
                items.push(v)
            }
        }
    } else if s.root_mode == "all" && location.is_empty() {
        for (name, path) in roots {
            let drive = state
                .inner
                .lock()
                .unwrap()
                .drives
                .iter()
                .find(|d| Path::new(&d.path) == path)
                .cloned();
            items.push(json!({"name":drive.map(|d|format!("{} ({})",if d.name.is_empty(){"Local disk"}else{&d.name},d.path)).unwrap_or(name.clone()),"location":name,"directory":true,"size":0,"modified":0,"mime":"inode/directory","hidden":false}));
        }
    } else {
        let path = resolve(state, location)?;
        for entry in fs::read_dir(path).map_err(|e| e.to_string())?.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !s.show_hidden && is_hidden(&entry.path()) {
                continue;
            }
            let loc = if location.is_empty() {
                name
            } else {
                format!("{location}/{name}")
            };
            if resolve(state, &loc).is_err() {
                continue;
            }
            if let Ok(v) = info(&entry.path(), &loc) {
                items.push(v)
            }
        }
    }
    items.sort_by(|a, b| {
        b["directory"]
            .as_bool()
            .cmp(&a["directory"].as_bool())
            .then_with(|| {
                a["name"]
                    .as_str()
                    .unwrap_or("")
                    .to_lowercase()
                    .cmp(&b["name"].as_str().unwrap_or("").to_lowercase())
            })
    });
    Ok(json!({"items":items,"privateMode":s.private_mode}))
}
pub fn writable(state: &AppState) -> Result<(), String> {
    let r = state.inner.lock().unwrap();
    if r.saved.settings.private_mode || r.saved.settings.restrict_modify {
        Err("File changes are restricted".into())
    } else {
        Ok(())
    }
}
pub fn protect_root(state: &AppState, path: &Path) -> Result<(), String> {
    if fs::canonicalize(&state.data)
        .is_ok_and(|private| path.starts_with(&private) || private.starts_with(path))
    {
        return Err("ShareX application data and its parent folders are protected".into());
    }
    if state
        .inner
        .lock()
        .unwrap()
        .roots
        .iter()
        .any(|(_, p)| fs::canonicalize(p).ok().as_deref() == Some(path))
    {
        Err("Cannot modify a shared root".into())
    } else {
        Ok(())
    }
}
pub fn copy_tree(source: &Path, dest: &Path) -> Result<u64, String> {
    if dest.exists() {
        return Err("Destination already exists".into());
    }
    let meta = fs::symlink_metadata(source).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err("Linked files are excluded".into());
        }
    }
    if meta.file_type().is_symlink() {
        return Err("Linked files are excluded".into());
    }
    if meta.is_file() {
        let mut src = fs::File::open(source).map_err(|e| e.to_string())?;
        let mut dst = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dest)
            .map_err(|e| e.to_string())?;
        return std::io::copy(&mut src, &mut dst).map_err(|e| e.to_string());
    }
    fs::create_dir(dest).map_err(|e| e.to_string())?;
    let mut bytes = 0;
    for entry in fs::read_dir(source).map_err(|e| e.to_string())?.flatten() {
        bytes += copy_tree(&entry.path(), &dest.join(entry.file_name()))?
    }
    Ok(bytes)
}
