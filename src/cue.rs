//! CUE Sheet parsing for single-file albums.
//!
//! This module only parses metadata and INDEX positions. Scanner/playback
//! integration is deliberately separate so the parser can be tested alone.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct CueTrack {
    pub number: u32,
    pub title: String,
    pub performer: String,
    /// Start offset in seconds from the beginning of the audio file.
    pub start: f64,
    /// End offset, filled from the next track's start.
    pub end: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CueSheet {
    pub title: String,
    pub performer: String,
    pub file: PathBuf,
    pub tracks: Vec<CueTrack>,
}

/// Reads a CUE sheet from disk, accepting UTF-8, UTF-16 BOM and GBK.
pub fn load(path: &Path) -> Result<CueSheet, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("读取 CUE 失败：{error}"))?;
    let text = decode_text(&bytes);
    parse(&text, path)
}

fn decode_text(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).to_string();
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => {
            let (text, _, _) = encoding_rs::GBK.decode(bytes);
            text.to_string()
        }
    }
}

/// Parses a CUE sheet. `cue_path` is used to resolve the FILE path.
pub fn parse(text: &str, cue_path: &Path) -> Result<CueSheet, String> {
    let mut album_title = String::new();
    let mut album_performer = String::new();
    let mut file: Option<PathBuf> = None;
    let mut tracks: Vec<CueTrack> = Vec::new();
    let mut current: Option<CueTrack> = None;
    let mut current_track_title = String::new();
    let mut current_track_performer = String::new();

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("REM ") {
            continue;
        }
        let Some((keyword, rest)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let keyword = keyword.to_ascii_uppercase();
        let rest = rest.trim();
        match keyword.as_str() {
            "FILE" => {
                let name = rest
                    .strip_prefix('"')
                    .and_then(|value| value.split_once('"').map(|(name, _)| name))
                    .or_else(|| rest.split_whitespace().next())
                    .unwrap_or("");
                if !name.is_empty() {
                    let path = Path::new(name);
                    file = Some(if path.is_absolute() {
                        path.to_path_buf()
                    } else {
                        cue_path.parent().unwrap_or(Path::new(".")).join(path)
                    });
                }
            }
            "TRACK" => {
                if let Some(track) = current.take() {
                    if track.start >= 0.0 {
                        tracks.push(track);
                    }
                }
                let number = rest
                    .split_whitespace()
                    .next()
                    .and_then(|value| value.parse::<u32>().ok())
                    .ok_or_else(|| "CUE TRACK 缺少编号".to_string())?;
                current = Some(CueTrack {
                    number,
                    title: String::new(),
                    performer: String::new(),
                    start: -1.0,
                    end: None,
                });
                current_track_title.clear();
                current_track_performer.clear();
            }
            "TITLE" => {
                let value = unquote(rest);
                if current.is_some() {
                    current_track_title = value;
                } else {
                    album_title = value;
                }
            }
            "PERFORMER" => {
                let value = unquote(rest);
                if current.is_some() {
                    current_track_performer = value;
                } else {
                    album_performer = value;
                }
            }
            "INDEX" => {
                let mut parts = rest.split_whitespace();
                let index = parts.next().unwrap_or("");
                let stamp = parts.next().unwrap_or("");
                if index == "01" {
                    if let Some(track) = &mut current {
                        track.start = parse_stamp(stamp)
                            .ok_or_else(|| format!("CUE INDEX 时间无效：{stamp}"))?;
                    }
                }
            }
            _ => {}
        }
        if let Some(track) = &mut current {
            track.title = current_track_title.clone();
            track.performer = if current_track_performer.is_empty() {
                album_performer.clone()
            } else {
                current_track_performer.clone()
            };
        }
    }
    if let Some(track) = current.take() {
        if track.start >= 0.0 {
            tracks.push(track);
        }
    }
    if tracks.is_empty() {
        return Err("CUE 中没有找到带 INDEX 01 的曲目".to_string());
    }
    let file = file.ok_or_else(|| "CUE 缺少 FILE 声明".to_string())?;
    for index in 0..tracks.len() {
        if index + 1 < tracks.len() {
            tracks[index].end = Some(tracks[index + 1].start);
        }
    }
    if album_title.is_empty() {
        album_title = file
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_else(|| "未知专辑".to_string());
    }
    Ok(CueSheet {
        title: album_title,
        performer: album_performer,
        file,
        tracks,
    })
}

fn unquote(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(value)
        .to_string()
}

/// Parses `mm:ss:ff` where ff is 1/75 second.
fn parse_stamp(value: &str) -> Option<f64> {
    let mut parts = value.split(':');
    let minutes = parts.next()?.parse::<u32>().ok()?;
    let seconds = parts.next()?.parse::<u32>().ok()?;
    let frames = parts.next()?.parse::<u32>().ok()?;
    if seconds >= 60 || frames >= 75 {
        return None;
    }
    Some(f64::from(minutes) * 60.0 + f64::from(seconds) + f64::from(frames) / 75.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHEET: &str = r#"
PERFORMER "周杰伦"
TITLE "叶惠美"
FILE "album.flac" WAVE
  TRACK 01 AUDIO
    TITLE "以父之名"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "懦夫"
    PERFORMER "周杰伦"
    INDEX 01 05:41:50
"#;

    #[test]
    fn parses_album_and_track_metadata() {
        let sheet = parse(SHEET, Path::new(r"D:\Music\album.cue")).expect("sheet");
        assert_eq!(sheet.title, "叶惠美");
        assert_eq!(sheet.performer, "周杰伦");
        assert_eq!(sheet.file, PathBuf::from(r"D:\Music\album.flac"));
        assert_eq!(sheet.tracks.len(), 2);
        assert_eq!(sheet.tracks[0].title, "以父之名");
        assert_eq!(sheet.tracks[0].end, Some(5.0 * 60.0 + 41.0 + 50.0 / 75.0));
    }

    #[test]
    fn rejects_sheet_without_tracks() {
        let error = parse("FILE \"x.flac\" WAVE
", Path::new("x.cue")).unwrap_err();
        assert!(error.contains("INDEX"));
    }
}
