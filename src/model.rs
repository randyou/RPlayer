//! Core data model: tracks, playlists and the persisted library.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::Accessor;
use serde::{Deserialize, Serialize};

pub const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "oga", "m4a", "aac"];

/// A single playable local audio file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: u64,
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub year: Option<u32>,
    pub track_number: Option<u32>,
    pub duration: f64,
    pub added_at: u64,
}

impl Track {
    /// Reads tags and stream properties from disk.
    ///
    /// Files that cannot be parsed by `lofty` are still returned with sensible
    /// filename-based fallbacks, as long as the decoder can potentially play them.
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let tagged = lofty::read_from_path(path)?;
        let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
        let props = tagged.properties();

        let file_stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "未知歌曲".to_string());

        let title = tag
            .and_then(|t| t.title())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or(file_stem);

        let artist = tag
            .and_then(|t| t.artist())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "未知艺术家".to_string());

        let album = tag
            .and_then(|t| t.album())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "未知专辑".to_string());

        let genre = tag
            .and_then(|t| t.genre())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        let year = tag.and_then(|t| t.year());
        let track_number = tag.and_then(|t| t.track());

        Ok(Self {
            id: stable_id(path),
            path: path.to_path_buf(),
            title,
            artist,
            album,
            genre,
            year,
            track_number,
            duration: props.duration().as_secs_f64(),
            added_at: now_millis(),
        })
    }

    pub fn extension(&self) -> String {
        self.path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default()
    }

    /// Short uppercase codec label shown in the table ("FLAC", "MP3").
    pub fn format_label(&self) -> String {
        self.extension().to_ascii_uppercase()
    }
}

/// A user-created, ordered collection of track ids.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: u64,
    pub name: String,
    pub track_ids: Vec<u64>,
    pub created_at: u64,
}

impl Playlist {
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            track_ids: Vec::new(),
            created_at: now_millis(),
        }
    }
}

/// One condition in a smart playlist.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SmartRule {
    pub field: String,
    pub op: String,
    pub value: String,
}

impl Default for SmartRule {
    fn default() -> Self {
        Self {
            field: "rating".to_string(),
            op: ">=".to_string(),
            value: "2".to_string(),
        }
    }
}

/// A saved, rule-driven playlist. It stores no track ids: results are
/// evaluated against the current library and playback statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SmartPlaylist {
    pub id: u64,
    pub name: String,
    pub match_all: bool,
    pub rules: Vec<SmartRule>,
    /// 0 means no limit.
    pub limit: usize,
    pub created_at: u64,
}

impl Default for SmartPlaylist {
    fn default() -> Self {
        Self {
            id: 1,
            name: "智能歌单".to_string(),
            match_all: true,
            rules: vec![SmartRule::default()],
            limit: 0,
            created_at: now_millis(),
        }
    }
}

/// Everything that survives an app restart.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub version: u32,
    pub folders: Vec<PathBuf>,
    pub tracks: Vec<Track>,
    pub playlists: Vec<Playlist>,
    pub smart_playlists: Vec<SmartPlaylist>,
    pub volume: f32,
    /// Track ids the listener has hearted, newest first.
    pub favorites: Vec<u64>,
    /// Recently played track ids, newest first (capped at 100).
    pub recent: Vec<u64>,
    /// Persisted appearance preference.
    pub light_theme: bool,
    /// Theme preset id (paper/sand/mist/forest/midnight/charcoal).
    pub theme_preset: String,
    /// Whether the desktop lyrics overlay should open with the app.
    pub desktop_lyrics: bool,
    /// Background opacity of the desktop lyrics card (0.35–1.0).
    pub desktop_lyrics_opacity: f32,
    /// Desktop lyric font size in points (18–40).
    pub desktop_lyrics_font: f32,
    /// Whether the desktop lyric window position is locked.
    pub desktop_lyrics_locked: bool,
    /// Last desktop lyric window top-left position.
    pub desktop_lyrics_pos: Option<(f32, f32)>,
    /// Mini player layout: "classic" or "compact".
    pub mini_style: String,
    /// Fetch missing lyrics from LRCLIB automatically when a track starts.
    pub auto_lyrics: bool,
    /// Match missing covers through MusicBrainz/Cover Art Archive.
    pub auto_cover: bool,
    /// ReplayGain mode: off / track / album.
    pub replaygain: String,
    /// Ten-band equalizer settings.
    pub eq: crate::eq::EqSettings,
}

impl Default for Library {
    fn default() -> Self {
        Self {
            version: 5,
            folders: Vec::new(),
            tracks: Vec::new(),
            playlists: Vec::new(),
            smart_playlists: Vec::new(),
            volume: 0.72,
            favorites: Vec::new(),
            recent: Vec::new(),
            light_theme: true,
            theme_preset: "paper".to_string(),
            desktop_lyrics: false,
            desktop_lyrics_opacity: 0.85,
            desktop_lyrics_font: 25.0,
            desktop_lyrics_locked: false,
            desktop_lyrics_pos: None,
            mini_style: "classic".to_string(),
            auto_lyrics: true,
            auto_cover: true,
            replaygain: "off".to_string(),
            eq: crate::eq::EqSettings::default(),
        }
    }
}

impl Library {
    pub fn track(&self, id: u64) -> Option<&Track> {
        self.tracks.iter().find(|t| t.id == id)
    }

    pub fn playlist(&self, id: u64) -> Option<&Playlist> {
        self.playlists.iter().find(|p| p.id == id)
    }

    pub fn playlist_mut(&mut self, id: u64) -> Option<&mut Playlist> {
        self.playlists.iter_mut().find(|p| p.id == id)
    }

    pub fn create_playlist(&mut self, name: impl Into<String>) -> u64 {
        let mut id = now_millis().max(1);
        while self.playlists.iter().any(|p| p.id == id) {
            id += 1;
        }
        let playlist = Playlist::new(id, name);
        self.playlists.push(playlist);
        id
    }

    pub fn remove_playlist(&mut self, id: u64) {
        self.playlists.retain(|p| p.id != id);
    }

    pub fn smart_playlist(&self, id: u64) -> Option<&SmartPlaylist> {
        self.smart_playlists.iter().find(|list| list.id == id)
    }

    pub fn create_smart_playlist(
        &mut self,
        name: impl Into<String>,
        match_all: bool,
        rules: Vec<SmartRule>,
        limit: usize,
    ) -> u64 {
        let mut id = now_millis().max(1);
        while self.playlists.iter().any(|list| list.id == id)
            || self.smart_playlists.iter().any(|list| list.id == id)
        {
            id += 1;
        }
        self.smart_playlists.push(SmartPlaylist {
            id,
            name: name.into(),
            match_all,
            rules,
            limit,
            created_at: now_millis(),
        });
        id
    }

    pub fn remove_smart_playlist(&mut self, id: u64) {
        self.smart_playlists.retain(|list| list.id != id);
    }

    /// Adds a track to a playlist, keeping order unique.
    /// Returns `false` when the track is already present.
    pub fn add_to_playlist(&mut self, playlist_id: u64, track_id: u64) -> bool {
        if self.track(track_id).is_none() {
            return false;
        }
        let Some(playlist) = self.playlist_mut(playlist_id) else {
            return false;
        };
        if playlist.track_ids.contains(&track_id) {
            return false;
        }
        playlist.track_ids.push(track_id);
        true
    }

    pub fn remove_from_playlist(&mut self, playlist_id: u64, track_id: u64) {
        if let Some(playlist) = self.playlist_mut(playlist_id) {
            playlist.track_ids.retain(|id| *id != track_id);
        }
    }

    /// Drops references to tracks that are no longer in the library.
    pub fn prune_playlists(&mut self) {
        let known: std::collections::HashSet<u64> = self.tracks.iter().map(|t| t.id).collect();
        for playlist in &mut self.playlists {
            playlist.track_ids.retain(|id| known.contains(id));
        }
        self.favorites.retain(|id| known.contains(id));
        self.recent.retain(|id| known.contains(id));
    }

    pub fn is_favorite(&self, track_id: u64) -> bool {
        self.favorites.contains(&track_id)
    }

    /// Hearts or unhearts a track. Returns the new favorite state.
    pub fn toggle_favorite(&mut self, track_id: u64) -> bool {
        if self.track(track_id).is_none() {
            return false;
        }
        if let Some(index) = self.favorites.iter().position(|id| *id == track_id) {
            self.favorites.remove(index);
            false
        } else {
            self.favorites.insert(0, track_id);
            true
        }
    }

    /// Records a play, moving the track to the front of 最近播放.
    pub fn mark_played(&mut self, track_id: u64) {
        if self.track(track_id).is_none() {
            return;
        }
        self.recent.retain(|id| *id != track_id);
        self.recent.insert(0, track_id);
        self.recent.truncate(100);
    }

    pub fn total_duration(&self) -> f64 {
        self.tracks.iter().map(|t| t.duration).sum()
    }

    pub fn track_count_for_playlist(&self, playlist_id: u64) -> usize {
        self.playlist(playlist_id)
            .map(|p| p.track_ids.len())
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Title,
    Artist,
    Album,
    Duration,
    Added,
}

impl SortField {
    pub fn label(self) -> &'static str {
        match self {
            Self::Title => "标题",
            Self::Artist => "艺术家",
            Self::Album => "专辑",
            Self::Duration => "时长",
            Self::Added => "添加时间",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortState {
    pub field: SortField,
    pub ascending: bool,
}

impl Default for SortState {
    fn default() -> Self {
        Self {
            field: SortField::Title,
            ascending: true,
        }
    }
}

/// Stable id derived from the file path (case-insensitive on Windows).
pub fn stable_id(path: &Path) -> u64 {
    let key = path_key(path);
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in key.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

/// Normalized key used for de-duplicating paths in the library.
pub fn path_key(path: &Path) -> String {
    let mut s = path.to_string_lossy().replace('\\', "/");
    #[cfg(windows)]
    {
        s = s.to_lowercase();
    }
    while s.ends_with('/') && s.len() > 1 {
        s.pop();
    }
    s
}

pub fn same_path(a: &Path, b: &Path) -> bool {
    path_key(a) == path_key(b)
}

/// True when the path is a supported audio file.
pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .map(|ext| {
            let ext = ext.to_string_lossy().to_ascii_lowercase();
            SUPPORTED_EXTENSIONS.contains(&ext.as_str())
        })
        .unwrap_or(false)
}

/// Returns the first embedded cover image, if any.
pub fn read_cover(path: &Path) -> Option<Vec<u8>> {
    let tagged = lofty::read_from_path(path).ok()?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag())?;
    tag.pictures().first().map(|p| p.data().to_vec())
}

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// `3:42`, or `1:02:07` for long tracks.
pub fn format_duration(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "0:00".to_string();
    }
    let total = seconds.round() as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// `12 首歌曲 · 48 分钟` style summary.
pub fn format_library_summary(count: usize, seconds: f64) -> String {
    if count == 0 {
        return "还没有歌曲".to_string();
    }
    let minutes = (seconds / 60.0).round() as u64;
    if minutes >= 60 {
        format!(
            "{count} 首歌曲 · {} 小时 {} 分钟",
            minutes / 60,
            minutes % 60
        )
    } else {
        format!("{count} 首歌曲 · {minutes} 分钟")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_ids_are_deterministic() {
        let path = Path::new("/music/Album/01 Song.flac");
        assert_eq!(stable_id(path), stable_id(path));
        assert_ne!(
            stable_id(path),
            stable_id(Path::new("/music/Album/02 Song.flac"))
        );
    }

    #[test]
    fn duration_formatting() {
        assert_eq!(format_duration(0.0), "0:00");
        assert_eq!(format_duration(65.4), "1:05");
        assert_eq!(format_duration(3725.0), "1:02:05");
        assert_eq!(format_duration(f64::NAN), "0:00");
    }

    #[test]
    fn playlist_add_is_unique_and_prunes() {
        let mut lib = Library::default();
        let track = Track {
            id: 7,
            path: PathBuf::from("seven.mp3"),
            title: "Seven".into(),
            artist: "A".into(),
            album: "B".into(),
            genre: String::new(),
            year: None,
            track_number: None,
            duration: 10.0,
            added_at: 1,
        };
        lib.tracks.push(track);
        let pid = lib.create_playlist("Road trip");
        assert!(lib.add_to_playlist(pid, 7));
        assert!(!lib.add_to_playlist(pid, 7));
        assert_eq!(lib.track_count_for_playlist(pid), 1);
        lib.tracks.clear();
        lib.prune_playlists();
        assert_eq!(lib.track_count_for_playlist(pid), 0);
    }
}
