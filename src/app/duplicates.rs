//! duplicates side of the app: split out of `mod.rs` to keep files small.

use super::*;

impl RPlayerApp {
    pub(crate) fn open_duplicates(&mut self) {
        self.dialog = Dialog::Duplicates {
            groups: Vec::new(),
        };
        self.refresh_duplicates();
    }

    pub(crate) fn duplicate_groups(&self) -> Vec<crate::duplicates::DuplicateGroup> {
        crate::duplicates::find(&self.library.tracks)
            .into_iter()
            .filter(|group| {
                !self
                    .ignored_duplicate_groups
                    .contains(&crate::duplicates::signature(&group.track_ids))
            })
            .collect()
    }

    pub(crate) fn refresh_duplicates(&mut self) {
        if !matches!(self.dialog, Dialog::Duplicates { .. }) {
            return;
        }
        self.dialog = Dialog::Duplicates {
            groups: self.duplicate_groups(),
        };
    }

    pub(crate) fn ignore_duplicate_group(&mut self, group: &[u64]) {
        self.ignored_duplicate_groups
            .insert(crate::duplicates::signature(group));
        self.refresh_duplicates();
        self.toast("已忽略本组重复歌曲，本次运行不再提示", ToastKind::Info);
    }

    pub(crate) fn keep_duplicate_track(&mut self, keep: u64, group: &[u64]) {
        if !group.contains(&keep) {
            return;
        }
        let keep_title = self
            .library
            .track(keep)
            .map(|track| track.title.clone())
            .unwrap_or_else(|| "这首歌".to_string());
        let remove = crate::duplicates::others(group, keep);
        if remove.is_empty() {
            return;
        }
        let confirmed = rfd::MessageDialog::new()
            .set_title("移除重复歌曲")
            .set_description(format!(
                "保留《{keep_title}》，从曲库移除其余 {} 首？\n\n磁盘文件不会被删除。",
                remove.len()
            ))
            .set_buttons(rfd::MessageButtons::YesNo)
            .show()
            == rfd::MessageDialogResult::Yes;
        if !confirmed {
            return;
        }
        let mut ignored = Vec::new();
        for id in &remove {
            if let Some(track) = self.library.track(*id) {
                ignored.push(crate::model::scan_key(track));
            }
        }
        let before = self.library.tracks.len();
        self.library
            .tracks
            .retain(|track| !remove.contains(&track.id));
        let removed = before.saturating_sub(self.library.tracks.len());
        for key in ignored {
            if !self.library.ignored_tracks.contains(&key) {
                self.library.ignored_tracks.push(key);
            }
        }
        self.library.ignored_tracks.sort();
        self.library.ignored_tracks.dedup();
        self.library.prune_playlists();
        self.prune_transport();
        self.checked.retain(|id| !remove.contains(id));
        self.persist();
        self.refresh_duplicates();
        self.toast(
            format!(
                "已保留《{keep_title}》，从曲库移除 {removed} 首；磁盘文件未删除"
            ),
            ToastKind::Success,
        );
    }

    pub(crate) fn reveal_path(&mut self, path: &std::path::Path) {
        if !path.exists() {
            self.toast("文件不存在，可能已被移动", ToastKind::Error);
            return;
        }
        #[cfg(target_os = "windows")]
        {
            let result = std::process::Command::new("explorer")
                .arg("/select,")
                .arg(path)
                .spawn();
            if result.is_err() {
                self.toast("无法打开资源管理器", ToastKind::Error);
            }
        }
    }

}
