//! Free lyric lookup through the LRCLIB open database.
//!
//! LRCLIB does not require an API key. Requests run on a worker thread and use
//! the system `curl` binary so RPlayer stays light and does not pull a full TLS
//! stack into the desktop build.

use std::fmt;
use std::process::Command;

use serde_json::Value;

use crate::model::Track;

const API_BASE: &str = "https://lrclib.net/api/search";
const USER_AGENT: &str = "RPlayer/0.4 (local desktop music player)";

#[derive(Debug)]
pub struct FetchResult {
    /// Raw LRC text (timed when the source had timestamps).
    pub text: String,
    pub synced: bool,
    pub source: &'static str,
}

#[derive(Debug)]
pub enum FetchError {
    NotFound,
    Network(String),
    Parse(String),
}

impl fmt::Display for FetchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(formatter, "没有找到匹配的歌词"),
            Self::Network(message) => write!(formatter, "网络请求失败：{message}"),
            Self::Parse(message) => write!(formatter, "歌词数据解析失败：{message}"),
        }
    }
}

/// Looks up `track` on LRCLIB. Tries artist+album, then artist, then title.
pub fn fetch(track: &Track) -> Result<FetchResult, FetchError> {
    let title = track.title.trim();
    if title.is_empty() {
        return Err(FetchError::NotFound);
    }

    let mut queries = Vec::new();
    queries.push(format!(
        "{API_BASE}?track_name={}&artist_name={}&album_name={}",
        encode(title),
        encode(track.artist.trim()),
        encode(track.album.trim())
    ));
    if !track.artist.trim().is_empty() {
        queries.push(format!(
            "{API_BASE}?track_name={}&artist_name={}",
            encode(title),
            encode(track.artist.trim())
        ));
    }
    queries.push(format!("{API_BASE}?track_name={}", encode(title)));

    let mut last_error = None;
    for url in queries {
        match http_get(&url) {
            Ok(body) => {
                if serde_json::from_str::<Value>(&body).is_err() {
                    return Err(FetchError::Parse("LRCLIB 返回了无效的 JSON".to_string()));
                }
                match select_from_json(&body, track.duration) {
                    Some(result) => return Ok(result),
                    None => continue,
                }
            }
            Err(error) => last_error = Some(error),
        }
    }

    if let Some(message) = last_error {
        Err(FetchError::Network(message))
    } else {
        Err(FetchError::NotFound)
    }
}

fn http_get(url: &str) -> Result<String, String> {
    let mut command = Command::new("curl");
    command.args([
        "--silent",
        "--show-error",
        "--fail",
        "--location",
        "--compressed",
        "--max-time",
        "35",
        "--user-agent",
        USER_AGENT,
        url,
    ]);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }

    let output = command
        .output()
        .map_err(|error| format!("无法启动 curl：{error}"))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if message.is_empty() {
            format!("curl 退出码 {}", output.status.code().unwrap_or(-1))
        } else {
            message
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub id: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
    pub synced: bool,
    pub text: String,
}

/// Returns all usable candidates for the user to choose from.
pub fn search_candidates(
    title: &str,
    artist: &str,
    album: &str,
    duration: f64,
) -> Result<Vec<Candidate>, FetchError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(FetchError::NotFound);
    }
    let mut queries = Vec::new();
    if !artist.trim().is_empty() && !album.trim().is_empty() {
        queries.push(format!(
            "{API_BASE}?track_name={}&artist_name={}&album_name={}",
            encode(title),
            encode(artist.trim()),
            encode(album.trim())
        ));
    }
    if !artist.trim().is_empty() {
        queries.push(format!(
            "{API_BASE}?track_name={}&artist_name={}",
            encode(title),
            encode(artist.trim())
        ));
    }
    queries.push(format!("{API_BASE}?track_name={}", encode(title)));

    for url in queries {
        let Ok(body) = http_get(&url) else { continue; };
        let mut candidates = candidates_from_json(&body);
        if !candidates.is_empty() {
            candidates.sort_by(|a, b| {
                let a_diff = (a.duration - duration).abs();
                let b_diff = (b.duration - duration).abs();
                b.synced
                    .cmp(&a.synced)
                    .then_with(|| a_diff.total_cmp(&b_diff))
            });
            candidates.dedup_by_key(|candidate| candidate.id);
            candidates.truncate(20);
            return Ok(candidates);
        }
    }
    Err(FetchError::NotFound)
}

pub(crate) fn candidates_from_json(body: &str) -> Vec<Candidate> {
    let Ok(items) = serde_json::from_str::<Value>(body) else {
        return Vec::new();
    };
    let Some(array) = items.as_array() else {
        return Vec::new();
    };
    let mut output = Vec::new();
    for item in array {
        if item.get("instrumental").and_then(Value::as_bool).unwrap_or(false) {
            continue;
        }
        let synced = item
            .get("syncedLyrics")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty());
        let plain = item
            .get("plainLyrics")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty());
        let (text, is_synced) = match (synced, plain) {
            (Some(text), _) => (text.to_string(), true),
            (None, Some(text)) => (text.to_string(), false),
            _ => continue,
        };
        output.push(Candidate {
            id: item.get("id").and_then(Value::as_u64).unwrap_or(0),
            title: item
                .get("trackName")
                .and_then(Value::as_str)
                .unwrap_or("未知歌曲")
                .to_string(),
            artist: item
                .get("artistName")
                .and_then(Value::as_str)
                .unwrap_or("未知艺术家")
                .to_string(),
            album: item
                .get("albumName")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            duration: item.get("duration").and_then(Value::as_f64).unwrap_or(0.0),
            synced: is_synced,
            text,
        });
    }
    output
}

/// Picks the best candidate from an LRCLIB search response. Split out for tests.
pub(crate) fn select_from_json(body: &str, duration: f64) -> Option<FetchResult> {
    let items: Value = serde_json::from_str(body).ok()?;
    let array = items.as_array()?;
    let mut best: Option<(i64, FetchResult)> = None;

    for item in array {
        if item
            .get("instrumental")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            continue;
        }
        let synced = item
            .get("syncedLyrics")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty());
        let plain = item
            .get("plainLyrics")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty());
        let (text, is_synced) = match (synced, plain) {
            (Some(text), _) => (text.to_string(), true),
            (None, Some(text)) => (text.to_string(), false),
            _ => continue,
        };

        let item_duration = item
            .get("duration")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let mut score = if is_synced { 1000 } else { 0 };
        if duration > 0.0 && item_duration > 0.0 {
            let diff = (duration - item_duration).abs();
            if diff <= 3.0 {
                score += 200;
            } else if diff <= 8.0 {
                score += 90;
            } else if diff <= 20.0 {
                score += 20;
            } else {
                score -= 80;
            }
        }
        if best.as_ref().is_none_or(|(best_score, _)| score > *best_score) {
            best = Some((
                score,
                FetchResult {
                    text,
                    synced: is_synced,
                    source: "LRCLIB",
                },
            ));
        }
    }

    best.map(|(_, result)| result)
}

pub(crate) fn encode(value: &str) -> String {
    let mut output = String::new();
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(*byte as char);
            }
            b' ' => output.push('+'),
            _ => output.push_str(&format!("%{byte:02X}")),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_cjk_and_spaces() {
        assert_eq!(encode("晴天 周杰伦"), "%E6%99%B4%E5%A4%A9+%E5%91%A8%E6%9D%B0%E4%BC%A6");
    }

    #[test]
    fn prefers_synced_lyrics_close_to_duration() {
        let body = r#"[
            {"trackName":"A","instrumental":false,"duration":200.0,"syncedLyrics":null,"plainLyrics":"plain a"},
            {"trackName":"A","instrumental":false,"duration":201.0,"syncedLyrics":"[00:01.00]line","plainLyrics":"plain a"}
        ]"#;
        let result = select_from_json(body, 200.0).expect("candidate");
        assert!(result.synced);
        assert!(result.text.contains("[00:01.00]"));
    }

    #[test]
    fn parses_candidate_list_for_user_selection() {
        let body = r#"[
            {"id":1,"trackName":"晴天","artistName":"周杰伦","albumName":"叶惠美","duration":269.0,"syncedLyrics":"[00:01.00]line","plainLyrics":"line"},
            {"id":2,"trackName":"晴天","artistName":"翻唱","albumName":"","duration":270.0,"syncedLyrics":null,"plainLyrics":"plain"}
        ]"#;
        let candidates = candidates_from_json(body);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].title, "晴天");
        assert!(candidates[0].synced);
        assert!(!candidates[1].synced);
    }

    #[test]
    fn skips_instrumentals_and_empty_results() {
        let body = r#"[
            {"instrumental":true,"syncedLyrics":"[00:01.00]x"},
            {"instrumental":false,"syncedLyrics":null,"plainLyrics":null}
        ]"#;
        assert!(select_from_json(body, 100.0).is_none());
        assert!(select_from_json("[]", 100.0).is_none());
    }
}
