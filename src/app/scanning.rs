//! scanning side of the app: split out of `mod.rs` to keep files small.

use super::*;

impl RPlayerApp {
    pub(crate) fn sync_watch(&mut self) {
        if self.watch_folders == self.library.folders {
            return;
        }
        self.watch_folders = self.library.folders.clone();
        self.watch = if self.watch_folders.is_empty() {
            None
        } else {
            Some(crate::watch::start(self.watch_folders.clone()))
        };
    }

    pub(crate) fn poll_watch(&mut self) {
        let changed = self
            .watch
            .as_ref()
            .is_some_and(crate::watch::FolderWatcher::poll_changed);
        if !changed {
            return;
        }
        if self.scanning {
            self.watch_pending = true;
            return;
        }
        self.toast("音乐目录有更新，正在刷新…", ToastKind::Info);
        self.start_scan();
    }

    pub(crate) fn start_scan(&mut self) {
        if self.library.folders.is_empty() {
            self.toast("请先添加一个音乐文件夹", ToastKind::Info);
            return;
        }
        self.scanning = true;
        self.scan_total = 0;
        self.scan_done = 0;
        self.scan_seen.clear();
        self.scan_errors.clear();
        self.error_card_dismissed = false;
        let known: std::collections::HashMap<String, (u64, u64)> = self
            .library
            .tracks
            .iter()
            .map(|track| {
                (
                    crate::model::scan_key(track),
                    (track.id, track.modified),
                )
            })
            .collect();
        self.scan_rx = Some(scanner::start_scan(
            self.library.folders.clone(),
            known,
        ));
    }

    pub(crate) fn poll_scan(&mut self) {
        let mut events = Vec::new();
        let mut disconnected = false;
        if let Some(rx) = &self.scan_rx {
            loop {
                match rx.try_recv() {
                    Ok(event) => events.push(event),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }

        let mut finished = false;
        for event in events {
            match event {
                ScanEvent::Started { total } => {
                    self.scan_total = total;
                    self.scan_done = 0;
                }
                ScanEvent::Track(track) => {
                    let key = crate::model::scan_key(&track);
                    if self.library.ignored_tracks.iter().any(|item| item == &key) {
                        self.scan_seen.insert(track.id);
                        continue;
                    }
                    self.scan_seen.insert(track.id);
                    if let Some(existing) =
                        self.library.tracks.iter_mut().find(|t| t.id == track.id)
                    {
                        // Keep the original "added" timestamp across rescans.
                        let added_at = existing.added_at;
                        *existing = *track;
                        existing.added_at = added_at;
                    } else {
                        self.library.push_track(*track);
                    }
                }
                ScanEvent::Unchanged(id) => {
                    self.scan_seen.insert(id);
                }
                ScanEvent::Progress { scanned, total } => {
                    self.scan_done = scanned;
                    self.scan_total = total;
                }
                ScanEvent::Finished { errors } => {
                    self.scan_errors = errors;
                    self.scan_finished_at = Some(std::time::Instant::now());
                    self.error_card_open = false;
                    self.error_card_dismissed = false;
                    finished = true;
                }
            }
        }

        if finished || disconnected {
            if !finished {
                self.scan_errors.push("扫描线程意外结束".to_string());
            }
            self.scan_rx = None;
            self.finish_scan();
        }
    }

    pub(crate) fn finish_scan(&mut self) {
        self.scanning = false;
        let seen = &self.scan_seen;
        let before = self.library.tracks.len();
        self.library.tracks.retain(|track| seen.contains(&track.id));
        self.library.refresh_index();
        let removed = before.saturating_sub(self.library.tracks.len());
        self.library.prune_playlists();

        let current_missing = self
            .current
            .map(|id| self.library.track(id).is_none())
            .unwrap_or(false);
        if current_missing {
            if let Some(audio) = &mut self.audio {
                audio.stop();
            }
            self.current = None;
            self.cover = None;
            self.cover_for = None;
            self.cover_image = None;
            self.cover_source = None;
            self.cover_tint = None;
        }

        let library = &self.library;
        self.queue.retain(|id| library.track(*id).is_some());
        if self.queue.is_empty() {
            self.queue_index = 0;
        } else if self.queue_index >= self.queue.len() {
            self.queue_index = self.queue.len() - 1;
        }

        let count = self.library.tracks.len();
        let errors = self.scan_errors.len();
        self.persist();

        let mut message = format!("扫描完成 · 共 {count} 首歌曲");
        if removed > 0 {
            message.push_str(&format!("，移除 {removed} 首失效记录"));
        }
        if errors > 0 {
            message.push_str(&format!("，{errors} 个文件无法读取"));
        }
        self.toast(message, ToastKind::Success);
        if self.watch_pending {
            self.watch_pending = false;
            self.start_scan();
        }
    }

    pub(crate) fn add_folder(&mut self, folder: PathBuf) {
        if self
            .library
            .folders
            .iter()
            .any(|existing| model::same_path(existing, &folder))
        {
            self.toast("这个文件夹已经在音乐库中", ToastKind::Info);
            return;
        }
        self.library.folders.push(folder);
        self.persist();
        self.start_scan();
    }

    pub(crate) fn open_folder_picker(&mut self) {
        if let Some(folder) = rfd::FileDialog::new()
            .set_title("选择音乐文件夹")
            .pick_folder()
        {
            self.add_folder(folder);
        }
    }

    pub(crate) fn import_m3u(&mut self) {
        let Some(file) = rfd::FileDialog::new()
            .set_title("导入 M3U 播放列表")
            .add_filter("M3U 播放列表", &["m3u", "m3u8"])
            .pick_file()
        else {
            return;
        };
        let paths = match crate::m3u::parse_file(&file) {
            Ok(paths) => paths,
            Err(err) => {
                self.toast(format!("读取 M3U 失败：{err}"), ToastKind::Error);
                return;
            }
        };
        let mut ids = Vec::new();
        let mut missing = 0usize;
        for path in paths {
            let exact = self
                .library
                .tracks
                .iter()
                .find(|track| model::same_path(&track.path, &path))
                .map(|track| track.id);
            let fallback = || {
                let name = path.file_name()?;
                let mut matches = self
                    .library
                    .tracks
                    .iter()
                    .filter(|track| track.path.file_name() == Some(name));
                let first = matches.next()?;
                if matches.next().is_some() {
                    None
                } else {
                    Some(first.id)
                }
            };
            if let Some(id) = exact.or_else(fallback) {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            } else {
                missing += 1;
            }
        }
        if ids.is_empty() {
            self.toast("M3U 中没有找到曲库内的歌曲", ToastKind::Error);
            return;
        }
        let name = file
            .file_stem()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "导入的播放列表".to_string());
        let id = self.library.create_playlist(name.clone());
        for track_id in &ids {
            self.library.add_to_playlist(id, *track_id);
        }
        self.view = View::Playlist(id);
        self.persist();
        self.toast(
            format!(
                "已导入「{name}」：{} 首{}",
                ids.len(),
                if missing > 0 {
                    format!("，跳过 {missing} 首缺失")
                } else {
                    String::new()
                }
            ),
            ToastKind::Success,
        );
    }

    pub(crate) fn export_m3u(&mut self) {
        let ids = self.visible_ids();
        if ids.is_empty() {
            self.toast("当前视图没有可导出的歌曲", ToastKind::Info);
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .set_title("导出 M3U 播放列表")
            .set_file_name("RPlayer.m3u8")
            .add_filter("M3U8 播放列表", &["m3u8", "m3u"])
            .save_file()
        else {
            return;
        };
        let paths: Vec<PathBuf> = ids
            .iter()
            .filter_map(|id| self.library.track(*id))
            .map(|track| track.path.clone())
            .collect();
        let base = path.parent().map(Path::to_path_buf);
        match crate::m3u::write_file(&path, &paths, base.as_deref()) {
            Ok(()) => self.toast(
                format!("已导出 {} 首歌曲", paths.len()),
                ToastKind::Success,
            ),
            Err(err) => self.toast(format!("导出 M3U 失败：{err}"), ToastKind::Error),
        }
    }

    pub(crate) fn remove_folder(&mut self, folder: &Path) {
        let Some(index) = self
            .library
            .folders
            .iter()
            .position(|f| model::same_path(f, folder))
        else {
            return;
        };
        let removed_folder = self.library.folders.remove(index);

        self.library
            .tracks
            .retain(|track| !path_is_under(&removed_folder, &track.path));
        // Removing and re-adding a folder is the explicit way to bring back
        // songs that duplicate cleanup previously ignored.
        let prefix = model::path_key(&removed_folder);
        let child_prefix = format!("{prefix}/");
        self.library.ignored_tracks.retain(|key| {
            let raw = key.strip_prefix("cue:").unwrap_or(key);
            let path = raw.split('#').next().unwrap_or(raw);
            path != prefix && !path.starts_with(&child_prefix)
        });
        self.library.prune_playlists();
        self.prune_transport();

        self.persist();
        if !self.library.folders.is_empty() {
            self.start_scan();
        } else {
            self.scanning = false;
            self.scan_rx = None;
            self.toast("已移除全部音乐文件夹", ToastKind::Info);
        }
    }

    pub(crate) fn prune_transport(&mut self) {
        let current_missing = self
            .current
            .map(|id| self.library.track(id).is_none())
            .unwrap_or(false);
        if current_missing {
            if let Some(audio) = &mut self.audio {
                audio.stop();
            }
            self.current = None;
            self.cover = None;
            self.cover_for = None;
            self.cover_image = None;
            self.cover_source = None;
            self.cover_tint = None;
        }
        if self
            .selected
            .map(|id| self.library.track(id).is_none())
            .unwrap_or(false)
        {
            self.selected = None;
        }

        let library = &self.library;
        self.queue.retain(|id| library.track(*id).is_some());
        if self.queue.is_empty() {
            self.queue_index = 0;
        } else if self.queue_index >= self.queue.len() {
            self.queue_index = self.queue.len() - 1;
        }
    }

    pub(crate) fn ingest_dropped(&mut self, paths: Vec<PathBuf>) {
        let mut folders = Vec::new();
        for path in paths {
            if path.is_dir() {
                folders.push(path);
            } else if model::is_audio_file(&path) {
                if let Some(parent) = path.parent() {
                    folders.push(parent.to_path_buf());
                }
            }
        }

        let mut added = 0usize;
        for folder in folders {
            if self
                .library
                .folders
                .iter()
                .any(|existing| model::same_path(existing, &folder))
            {
                continue;
            }
            self.library.folders.push(folder);
            added += 1;
        }

        if added == 0 {
            self.toast("没有发现新的音乐文件夹", ToastKind::Info);
            return;
        }
        self.persist();
        self.start_scan();
        self.toast(
            format!("已添加 {added} 个文件夹，正在扫描音乐…"),
            ToastKind::Info,
        );
    }

    // ---------------------------------------------------------------------
    // Playback
    // ---------------------------------------------------------------------

}
