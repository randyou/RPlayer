//! Lyrics loading and LRC parsing.
//!
//! RPlayer first looks for a sidecar `.lrc` file next to the audio file, then
//! falls back to the embedded USLT/©lyr tag. Files without timestamps still get
//! shown as plain lyrics.

use std::fs;
use std::path::Path;

use lofty::file::TaggedFileExt;
use lofty::tag::{ItemKey, Tag};

#[derive(Debug, Clone)]
pub struct LyricLine {
    pub time: f64,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
    /// True when the source had no timestamps at all.
    pub plain: bool,
}

pub struct EditorRow {
    pub time: String,
    pub text: String,
}

pub struct LyricsEditor {
    pub track_id: u64,
    pub title: String,
    pub rows: Vec<EditorRow>,
    pub plain: bool,
}

impl LyricsEditor {
    pub fn from_lyrics(track_id: u64, title: String, lyrics: &Lyrics) -> Self {
        Self {
            track_id,
            title,
            plain: lyrics.plain,
            rows: lyrics
                .lines
                .iter()
                .map(|line| EditorRow {
                    time: if lyrics.plain {
                        String::new()
                    } else {
                        format_timestamp(line.time)
                    },
                    text: line.text.clone(),
                })
                .collect(),
        }
    }

    pub fn build(&self) -> Option<Lyrics> {
        if self.rows.is_empty() {
            return None;
        }
        let mut lines = Vec::new();
        let mut timed = false;
        for row in &self.rows {
            if row.text.trim().is_empty() {
                continue;
            }
            let time = if row.time.trim().is_empty() {
                0.0
            } else {
                let value = parse_time(&row.time)?;
                timed = true;
                value
            };
            lines.push(LyricLine {
                time,
                text: row.text.trim().to_string(),
            });
        }
        if lines.is_empty() {
            return None;
        }
        if timed {
            lines.sort_by(|a, b| a.time.total_cmp(&b.time));
        }
        Some(Lyrics {
            lines,
            plain: !timed,
        })
    }
}

impl Lyrics {
    pub fn to_lrc(&self) -> String {
        if self.plain {
            return self
                .lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
        }
        let mut lines = self.lines.clone();
        lines.sort_by(|a, b| a.time.total_cmp(&b.time));
        lines
            .into_iter()
            .map(|line| format!("{} {}", format_timestamp(line.time), line.text))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Index of the line that should be highlighted at `position` seconds.
    pub fn active_index(&self, position: f64) -> Option<usize> {
        if self.plain || self.lines.is_empty() {
            return None;
        }
        let mut active = None;
        for (index, line) in self.lines.iter().enumerate() {
            if line.time <= position + 0.15 {
                active = Some(index);
            } else {
                break;
            }
        }
        active
    }
}

/// Loads lyrics for a track. Returns `None` when neither source has any text.
pub fn load(path: &Path) -> Option<Lyrics> {
    let sidecar = path.with_extension("lrc");
    if let Ok(bytes) = fs::read(&sidecar) {
        let text = decode_text(&bytes);
        let lyrics = parse_lrc(&text);
        if !lyrics.is_empty() {
            return Some(lyrics);
        }
    }

    if let Ok(tagged) = lofty::read_from_path(path) {
        if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
            if let Some(text) = embedded_lyrics(tag) {
                let lyrics = parse_lrc(text);
                if !lyrics.is_empty() {
                    return Some(lyrics);
                }
            }
        }
    }
    None
}

/// Loads user-edited lyrics; these take precedence over sidecar/cache files.
pub fn load_custom(track_id: u64) -> Option<Lyrics> {
    let bytes = fs::read(crate::storage::custom_lyrics_path(track_id)).ok()?;
    let lyrics = parse_lrc(&decode_text(&bytes));
    if lyrics.is_empty() { None } else { Some(lyrics) }
}

pub fn store_custom(track_id: u64, lyrics: &Lyrics) -> std::io::Result<()> {
    let path = crate::storage::custom_lyrics_path(track_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, lyrics.to_lrc().as_bytes())
}

/// Loads a previously downloaded lyric file from the app cache.
pub fn load_cached(track_id: u64) -> Option<Lyrics> {
    let bytes = fs::read(crate::storage::lyrics_cache_path(track_id)).ok()?;
    let lyrics = parse_lrc(&decode_text(&bytes));
    if lyrics.is_empty() {
        None
    } else {
        Some(lyrics)
    }
}

/// Persists downloaded lyrics for the next launch.
pub fn store_cached(track_id: u64, text: &str) -> std::io::Result<()> {
    let path = crate::storage::lyrics_cache_path(track_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text.as_bytes())
}

fn embedded_lyrics(tag: &Tag) -> Option<&str> {
    tag.get_string(&ItemKey::Lyrics)
        .or_else(|| tag.get_string(&ItemKey::Comment).filter(|text| text.contains('[')))
}

fn decode_text(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let units: Vec<u16> = rest
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let units: Vec<u16> = rest
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_owned();
    }
    // Mainland Chinese .lrc files are commonly GBK/GB18030 rather than UTF-8.
    let (text, _, _) = encoding_rs::GBK.decode(bytes);
    text.into_owned()
}

pub fn format_timestamp(time: f64) -> String {
    let total = time.max(0.0);
    let minutes = (total / 60.0).floor() as u32;
    let seconds = total - f64::from(minutes) * 60.0;
    format!("[{minutes:02}:{seconds:05.2}]")
}

/// Accepts `mm:ss.xx` and `hh:mm:ss.xx`, with or without brackets.
pub fn parse_time(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    let without_brackets = trimmed
        .strip_prefix('[')
        .and_then(|text| text.strip_suffix(']'))
        .unwrap_or(trimmed);
    parse_timestamp(without_brackets)
}

pub fn parse_lrc(text: &str) -> Lyrics {
    let mut timed: Vec<LyricLine> = Vec::new();
    let mut plain: Vec<String> = Vec::new();

    for raw in text.lines() {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let mut rest = raw;
        let mut times = Vec::new();
        while rest.starts_with('[') {
            let Some(end) = rest.find(']') else { break };
            let inside = &rest[1..end];
            if let Some(time) = parse_timestamp(inside) {
                times.push(time);
            }
            rest = rest[end + 1..].trim_start();
        }
        let content = rest.trim();
        if content.is_empty() {
            continue;
        }
        if times.is_empty() {
            plain.push(content.to_string());
        } else {
            for time in times {
                timed.push(LyricLine {
                    time,
                    text: content.to_string(),
                });
            }
        }
    }

    if timed.is_empty() {
        return Lyrics {
            plain: true,
            lines: plain
                .into_iter()
                .map(|text| LyricLine { time: 0.0, text })
                .collect(),
        };
    }

    timed.sort_by(|a, b| a.time.total_cmp(&b.time));
    timed.dedup_by(|a, b| (a.time - b.time).abs() < 0.01 && a.text == b.text);
    Lyrics {
        lines: timed,
        plain: false,
    }
}

fn parse_timestamp(value: &str) -> Option<f64> {
    let parts: Vec<&str> = value.split(':').collect();
    let (hours, minutes, seconds) = match parts.as_slice() {
        [minutes, seconds] => (0.0, minutes.parse::<f64>().ok()?, parse_seconds(seconds)?),
        [hours, minutes, seconds] => (
            hours.parse::<f64>().ok()?,
            minutes.parse::<f64>().ok()?,
            parse_seconds(seconds)?,
        ),
        _ => return None,
    };
    Some(hours * 3600.0 + minutes * 60.0 + seconds)
}

fn parse_seconds(value: &str) -> Option<f64> {
    let cleaned = value.trim().replace(',', ".");
    let seconds: f64 = cleaned.parse().ok()?;
    if (0.0..60.0).contains(&seconds) {
        Some(seconds)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn parses_timed_lrc_and_picks_active_line() {
        let lyrics = parse_lrc("[ar:RPlayer]\n[00:01.00]第一行\n[00:05.50][00:10.00]副歌\n");
        assert!(!lyrics.plain);
        assert_eq!(lyrics.lines.len(), 3);
        assert_eq!(lyrics.lines[0].text, "第一行");
        assert_eq!(lyrics.active_index(0.5), None);
        assert_eq!(lyrics.active_index(1.2), Some(0));
        assert_eq!(lyrics.active_index(6.0), Some(1));
        assert_eq!(lyrics.active_index(12.0), Some(2));
    }

    #[test]
    fn decodes_gbk_sidecar_lyrics() {
        let (encoded, _, _) = encoding_rs::GBK.encode("[00:01.00]你好\n");
        let text = decode_text(&encoded);
        assert!(text.contains("你好"));
        assert_eq!(parse_lrc(&text).lines[0].text, "你好");
    }

    #[test]
    fn keeps_plain_lyrics_without_timestamps() {
        let lyrics = parse_lrc("第一行\n第二行\n");
        assert!(lyrics.plain);
        assert_eq!(lyrics.lines.len(), 2);
        assert_eq!(lyrics.active_index(2.0), None);
    }
}
