//! Conservative single-track tag editing with an append-only backup journal.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use lofty::config::WriteOptions;
use lofty::file::TaggedFileExt;
use lofty::tag::{ItemKey, TagExt};
use serde::{Deserialize, Serialize};

use crate::model::Track;

#[derive(Debug, Clone, Serialize)]
pub struct TagBackup {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub year: Option<u32>,
    pub saved_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagEdit {
    pub id: u64,
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub year: String,
    pub original: Track,
}

impl PartialEq for TagEdit {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl TagEdit {
    pub fn from_track(track: &Track) -> Self {
        Self {
            id: track.id,
            path: track.path.clone(),
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            genre: track.genre.clone(),
            year: track
                .year
                .map(|year| year.to_string())
                .unwrap_or_default(),
            original: track.clone(),
        }
    }
}

pub fn save(edit: &TagEdit) -> Result<Track> {
    let mut tagged =
        lofty::read_from_path(&edit.path).with_context(|| format!("read {}", edit.path.display()))?;
    let has_primary = tagged.primary_tag().is_some();
    let tag = if has_primary {
        tagged.primary_tag_mut()
    } else {
        tagged.first_tag_mut()
    }
    .context("文件没有可写的标签")?;

    backup(edit)?;
    insert(tag, ItemKey::TrackTitle, &edit.title)?;
    insert(tag, ItemKey::TrackArtist, &edit.artist)?;
    insert(tag, ItemKey::AlbumTitle, &edit.album)?;
    insert(tag, ItemKey::Genre, &edit.genre)?;
    if let Ok(year) = edit.year.trim().parse::<u32>() {
        insert(tag, ItemKey::Year, &year.to_string())?;
    }
    tag.save_to_path(&edit.path, WriteOptions::default())
        .with_context(|| format!("write {}", edit.path.display()))?;

    let mut updated = Track::from_path(&edit.path)?;
    updated.added_at = edit.original.added_at;
    Ok(updated)
}

fn insert(tag: &mut lofty::tag::Tag, key: ItemKey, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Ok(());
    }
    tag.insert_text(key, value.trim().to_string());
    Ok(())
}

fn backup(edit: &TagEdit) -> Result<()> {
    let path = crate::storage::data_dir().join("tag_backups.jsonl");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let backup = TagBackup {
        path: edit.path.clone(),
        title: edit.original.title.clone(),
        artist: edit.original.artist.clone(),
        album: edit.original.album.clone(),
        genre: edit.original.genre.clone(),
        year: edit.original.year,
        saved_at: crate::model::now_millis(),
    };
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?;
    writeln!(file, "{}", serde_json::to_string(&backup)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lofty::tag::Accessor;

    #[test]
    fn rewrites_a_temporary_copy() {
        let source = PathBuf::from(
            r"C:\Users\Randy\Downloads\徐薇+-+Last+Dance_EG.flac",
        );
        if !source.is_file() {
            return;
        }
        let target = std::env::temp_dir().join("rplayer-tag-edit-test.flac");
        std::fs::copy(&source, &target).unwrap();
        let track = Track::from_path(&target).unwrap();
        let mut edit = TagEdit::from_track(&track);
        edit.title = "RPlayer Tag Test".to_string();
        edit.artist = "RPlayer".to_string();
        let updated = save(&edit).unwrap();
        assert_eq!(updated.title, "RPlayer Tag Test");
        assert_eq!(updated.artist, "RPlayer");
        let reread = Track::from_path(&target).unwrap();
        assert_eq!(reread.title, "RPlayer Tag Test");
        assert_eq!(reread.artist, "RPlayer");
        let restore = TagEdit::from_track(&edit.original);
        let restored = save(&restore).unwrap();
        assert_eq!(restored.title, edit.original.title);
        assert_eq!(restored.artist, edit.original.artist);
        let _ = std::fs::remove_file(target);
    }

    #[test]
    fn ignores_empty_values() {
        let mut tag = lofty::tag::Tag::new(lofty::tag::TagType::Id3v2);
        insert(&mut tag, ItemKey::TrackTitle, "  ").unwrap();
        assert!(tag.is_empty());
        insert(&mut tag, ItemKey::TrackTitle, "Title").unwrap();
        assert_eq!(tag.title().as_deref(), Some("Title"));
    }
}
