//! Portable, folder-based backups of the local music library state.
//!
//! A backup is a directory with a manifest. Core JSON/SQLite files are copied,
//! and the optional media caches can be included. Restores validate the whole
//! backup before touching live data and always create a pre-restore snapshot
//! so a failed apply can be rolled back.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

const MANIFEST_FILE: &str = "manifest.json";
const BACKUP_SCHEMA: u32 = 1;
const CORE_FILES: [&str; 4] = [
    "library.json",
    "library.db",
    "playback_stats.json",
    "session.json",
];
const MEDIA_DIRS: [&str; 3] = ["covers", "lyrics", "lyrics_custom"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub app: String,
    pub schema: u32,
    pub version: String,
    pub created_at: u64,
    pub include_media: bool,
    pub files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BackupInfo {
    pub manifest: Manifest,
    pub tracks: usize,
}

#[derive(Debug, Clone)]
pub struct RestoreReport {
    pub settings_only: bool,
    pub files: usize,
    pub auto_backup: PathBuf,
}

/// Creates a new backup directory under `dest_parent`.
pub fn create(data: &Path, dest_parent: &Path, include_media: bool) -> Result<PathBuf> {
    fs::create_dir_all(dest_parent)
        .with_context(|| format!("create {}", dest_parent.display()))?;
    let stamp = crate::model::now_millis();
    let mut dir = dest_parent.join(format!("RPlayer-backup-{stamp}"));
    let mut suffix = 1;
    while dir.exists() {
        dir = dest_parent.join(format!("RPlayer-backup-{stamp}-{suffix}"));
        suffix += 1;
    }
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;

    let mut files = Vec::new();
    for name in CORE_FILES {
        let source = data.join(name);
        if !source.is_file() {
            continue;
        }
        let target = dir.join(name);
        if name == "library.db" {
            snapshot_db(&source, &target)?;
        } else {
            fs::copy(&source, &target)
                .with_context(|| format!("copy {}", source.display()))?;
        }
        files.push(name.to_string());
    }

    if include_media {
        for name in MEDIA_DIRS {
            let source = data.join(name);
            if !source.is_dir() {
                continue;
            }
            copy_dir(&source, &dir.join(name))?;
            files.push(name.to_string());
        }
    }

    let manifest = Manifest {
        app: "RPlayer".to_string(),
        schema: BACKUP_SCHEMA,
        version: env!("CARGO_PKG_VERSION").to_string(),
        created_at: stamp,
        include_media,
        files,
    };
    fs::write(
        dir.join(MANIFEST_FILE),
        serde_json::to_vec_pretty(&manifest)?,
    )
    .with_context(|| format!("write {}", dir.join(MANIFEST_FILE).display()))?;
    Ok(dir)
}

/// Reads and validates a backup without changing live data.
pub fn inspect(dir: &Path) -> Result<BackupInfo> {
    let manifest_path = dir.join(MANIFEST_FILE);
    let bytes = fs::read(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let manifest: Manifest = serde_json::from_slice(&bytes).context("备份清单格式无效")?;
    if manifest.app != "RPlayer" {
        bail!("不是 RPlayer 备份");
    }
    if manifest.schema > BACKUP_SCHEMA {
        bail!("备份版本较新，请升级 RPlayer 后再恢复");
    }
    if manifest.files.is_empty() {
        bail!("备份中没有可恢复的数据");
    }

    let mut tracks = 0;
    for name in &manifest.files {
        if name.is_empty()
            || name.contains("..")
            || name.contains('/')
            || name.contains('\\')
            || Path::new(name).is_absolute()
        {
            bail!("备份清单包含非法路径：{name}");
        }
        let path = dir.join(name);
        match name.as_str() {
            "library.json" => validate_library(&path)?,
            "library.db" => tracks = validate_db(&path)?,
            "playback_stats.json" => validate_json::<crate::stats::PlaybackStats>(&path)?,
            "session.json" => validate_json::<crate::session::Session>(&path)?,
            _ if MEDIA_DIRS.contains(&name.as_str()) => {
                if !path.is_dir() {
                    bail!("备份缺少目录：{name}");
                }
            }
            _ => {}
        }
    }
    Ok(BackupInfo { manifest, tracks })
}

/// Restores a validated backup. A full restore replaces library, statistics
/// and session; `settings_only` replaces only `library.json` (settings,
/// playlists and favorites), leaving the current tracks and statistics intact.
pub fn restore(data: &Path, backup: &Path, settings_only: bool) -> Result<RestoreReport> {
    let info = inspect(backup)?;
    if settings_only && !info.manifest.files.iter().any(|name| name == "library.json") {
        bail!("这个备份不包含设置与播放列表");
    }
    let auto_backup = create(data, &data.join("backups"), true)
        .context("恢复前自动备份失败，未修改现有数据")?;

    let result = apply(data, backup, &info, settings_only);
    if let Err(error) = result {
        let rollback = apply(data, &auto_backup, &inspect(&auto_backup)?, false);
        return match rollback {
            Ok(()) => Err(error.context("恢复失败，已回滚到恢复前状态")),
            Err(rollback_error) => {
                Err(error.context(format!("恢复失败，回滚也失败：{rollback_error:#}")))
            }
        };
    }
    Ok(RestoreReport {
        settings_only,
        files: if settings_only { 1 } else { info.manifest.files.len() },
        auto_backup,
    })
}

fn apply(data: &Path, backup: &Path, info: &BackupInfo, settings_only: bool) -> Result<()> {
    let names: Vec<&String> = if settings_only {
        info.manifest
            .files
            .iter()
            .filter(|name| name.as_str() == "library.json")
            .collect()
    } else {
        info.manifest.files.iter().collect()
    };
    for name in names {
        let source = backup.join(name);
        let target = data.join(name);
        if source.is_dir() {
            copy_dir(&source, &target)?;
        } else {
            copy_atomic(&source, &target)?;
        }
    }
    // A restored database must not inherit stale sidecars from the old file.
    if !settings_only && info.manifest.files.iter().any(|name| name == "library.db") {
        for suffix in ["-wal", "-shm"] {
            let _ = fs::remove_file(data.join(format!("library.db{suffix}")));
        }
    }
    Ok(())
}

fn copy_atomic(source: &Path, target: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create {}", parent.display()))?;
    }
    let temp = target.with_extension(format!("restore-{}", crate::model::now_millis()));
    fs::copy(source, &temp)
        .with_context(|| format!("copy {}", source.display()))?;
    if target.exists() {
        fs::remove_file(target).with_context(|| format!("replace {}", target.display()))?;
    }
    fs::rename(&temp, target).with_context(|| format!("write {}", target.display()))?;
    Ok(())
}

fn snapshot_db(source: &Path, target: &Path) -> Result<()> {
    let conn = Connection::open(source)
        .with_context(|| format!("open {}", source.display()))?;
    conn.execute("VACUUM INTO ?1", [target.to_string_lossy().as_ref()])
        .with_context(|| format!("snapshot {}", source.display()))?;
    Ok(())
}

fn validate_library(path: &Path) -> Result<()> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let _: crate::model::Library =
        serde_json::from_slice(&bytes).context("备份中的 library.json 无效")?;
    Ok(())
}

fn validate_db(path: &Path) -> Result<usize> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("open {}", path.display()))?;
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .context("检查备份数据库失败")?;
    if integrity != "ok" {
        bail!("备份数据库损坏：{integrity}");
    }
    conn.query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))
        .context("备份数据库缺少 tracks 表")
        .map(|count: i64| count.max(0) as usize)
}

fn validate_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<()> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let _: T = serde_json::from_slice(&bytes)
        .with_context(|| format!("备份中的 {} 无效", path.display()))?;
    Ok(())
}

fn copy_dir(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target).with_context(|| format!("create {}", target.display()))?;
    for entry in walkdir::WalkDir::new(source) {
        let entry = entry?;
        let relative = entry
            .path()
            .strip_prefix(source)
            .unwrap_or(entry.path());
        let destination = target.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&destination)
                .with_context(|| format!("create {}", destination.display()))?;
        } else {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &destination)
                .with_context(|| format!("copy {}", entry.path().display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rplayer-backup-{name}-{}-{}",
            std::process::id(),
            crate::model::now_millis()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn prepare_data(dir: &Path, tracks: i64) {
        let library = crate::model::Library::default();
        fs::write(
            dir.join("library.json"),
            serde_json::to_vec_pretty(&library).unwrap(),
        )
        .unwrap();
        let conn = Connection::open(dir.join("library.db")).unwrap();
        conn.execute_batch(
            "CREATE TABLE tracks (id INTEGER PRIMARY KEY, payload TEXT NOT NULL);",
        )
        .unwrap();
        for id in 0..tracks {
            conn.execute(
                "INSERT INTO tracks (id, payload) VALUES (?1, ?2)",
                rusqlite::params![id, format!("track-{id}")],
            )
            .unwrap();
        }
        fs::write(
            dir.join("playback_stats.json"),
            serde_json::to_vec_pretty(&crate::stats::PlaybackStats::default()).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("session.json"),
            serde_json::to_vec_pretty(&crate::session::Session::default()).unwrap(),
        )
        .unwrap();
    }

    fn track_count(path: &Path) -> usize {
        let conn = Connection::open(path).unwrap();
        conn.query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get::<_, i64>(0))
            .unwrap() as usize
    }

    #[test]
    fn backup_restores_core_state_and_keeps_a_snapshot() {
        let data = temp_dir("data");
        let backups = temp_dir("dest");
        prepare_data(&data, 2);
        let backup = create(&data, &backups, false).unwrap();
        let info = inspect(&backup).unwrap();
        assert_eq!(info.tracks, 2);
        assert!(info.manifest.files.contains(&"library.json".to_string()));

        // Damage the live state, then restore the backup.
        fs::write(data.join("library.json"), b"{}").unwrap();
        let conn = Connection::open(data.join("library.db")).unwrap();
        conn.execute("DELETE FROM tracks", []).unwrap();
        drop(conn);
        let report = restore(&data, &backup, false).unwrap();
        assert_eq!(report.files, 4);
        assert!(report.auto_backup.join("manifest.json").is_file());
        assert_eq!(track_count(&data.join("library.db")), 2);
        assert!(serde_json::from_slice::<crate::model::Library>(
            &fs::read(data.join("library.json")).unwrap()
        )
        .is_ok());
    }

    #[test]
    fn settings_only_restore_leaves_tracks_alone() {
        let data = temp_dir("settings-data");
        let backups = temp_dir("settings-dest");
        prepare_data(&data, 2);
        let backup = create(&data, &backups, false).unwrap();

        let conn = Connection::open(data.join("library.db")).unwrap();
        conn.execute("DELETE FROM tracks", []).unwrap();
        drop(conn);
        let report = restore(&data, &backup, true).unwrap();
        assert!(report.settings_only);
        assert_eq!(track_count(&data.join("library.db")), 0);
    }

    #[test]
    fn corrupt_backup_is_rejected_before_touching_data() {
        let data = temp_dir("corrupt-data");
        let backups = temp_dir("corrupt-dest");
        prepare_data(&data, 2);
        let backup = create(&data, &backups, false).unwrap();
        fs::write(backup.join("library.json"), b"not json").unwrap();

        assert!(inspect(&backup).is_err());
        assert!(restore(&data, &backup, false).is_err());
        assert_eq!(track_count(&data.join("library.db")), 2);
        assert!(!data.join("backups").exists() || {
            fs::read_dir(data.join("backups")).unwrap().next().is_none()
        });
    }

    #[test]
    fn manifest_path_traversal_is_rejected() {
        let data = temp_dir("traversal-data");
        let backups = temp_dir("traversal-dest");
        prepare_data(&data, 1);
        let backup = create(&data, &backups, false).unwrap();
        let mut manifest: Manifest = serde_json::from_slice(
            &fs::read(backup.join(MANIFEST_FILE)).unwrap(),
        )
        .unwrap();
        manifest.files.push("../escape.json".to_string());
        fs::write(
            backup.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(inspect(&backup).is_err());
    }

    #[test]
    fn media_is_only_copied_when_requested() {
        let data = temp_dir("media-data");
        let backups = temp_dir("media-dest");
        prepare_data(&data, 1);
        fs::create_dir_all(data.join("covers")).unwrap();
        fs::write(data.join("covers").join("a.img"), b"cover").unwrap();

        let plain = create(&data, &backups, false).unwrap();
        assert!(!plain.join("covers").exists());
        let with_media = create(&data, &backups, true).unwrap();
        assert!(with_media.join("covers").join("a.img").is_file());
        assert!(inspect(&with_media).unwrap().manifest.include_media);
    }
}
