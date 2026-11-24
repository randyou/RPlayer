//! Online album-art matching through MusicBrainz + Cover Art Archive.
//!
//! MusicBrainz asks clients to identify themselves and to stay around one
//! request per second per IP. Requests are serialized behind a process-wide
//! throttle; downloaded art is cached by album key.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::model::Track;

const MUSICBRAINZ_USER_AGENT: &str = "RPlayer/0.5 (local desktop music player)";
const MUSICBRAINZ_SEARCH: &str = "https://musicbrainz.org/ws/2/release-group/";
const COVER_ART_ARCHIVE: &str = "https://coverartarchive.org/release-group";

static MUSICBRAINZ_THROTTLE: Mutex<Option<Instant>> = Mutex::new(None);

#[derive(Debug)]
pub struct FetchResult {
    pub bytes: Vec<u8>,
    pub source: &'static str,
    pub matched_title: String,
}

#[derive(Debug)]
pub enum FetchError {
    NotFound,
    Network(String),
    TooLarge,
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(formatter, "没有找到匹配的封面"),
            Self::Network(message) => write!(formatter, "封面网络请求失败：{message}"),
            Self::TooLarge => write!(formatter, "封面图片过大"),
        }
    }
}

pub fn fetch(track: &Track) -> Result<FetchResult, FetchError> {
    let album = track.album.trim();
    if album.is_empty() || album == "未知专辑" {
        return Err(FetchError::NotFound);
    }
    let artist = track.artist.trim();

    let mut query = format!("releasegroup:\"{album}\"");
    if !artist.is_empty() && artist != "未知艺术家" {
        query.push_str(&format!(" AND artist:\"{artist}\""));
    }
    let url = format!(
        "{MUSICBRAINZ_SEARCH}?query={}&fmt=json&limit=8",
        crate::lyrics_fetch::encode(&query)
    );

    let body = musicbrainz_get(&url)?;
    let candidate = select_release_group(&body, track).ok_or(FetchError::NotFound)?;

    let mut last_error = None;
    for size in ["front-1200", "front-500", "front"] {
        let art_url = format!("{COVER_ART_ARCHIVE}/{}/{}", candidate.id, size);
        match http_get(&art_url, MUSICBRAINZ_USER_AGENT, 8) {
            Ok(bytes) => {
                if bytes.len() > 8 * 1024 * 1024 {
                    return Err(FetchError::TooLarge);
                }
                if bytes.len() < 512 {
                    last_error = Some("封面文件为空".to_string());
                    continue;
                }
                return Ok(FetchResult {
                    bytes,
                    source: "Cover Art Archive",
                    matched_title: candidate.title,
                });
            }
            Err(error) => last_error = Some(error),
        }
    }

    Err(FetchError::Network(
        last_error.unwrap_or_else(|| "封面下载失败".to_string()),
    ))
}

struct ReleaseGroup {
    id: String,
    title: String,
}

/// Picks the best MusicBrainz release group for the local album. Split out so
/// matching can be tested without network access.
fn select_release_group(body: &str, track: &Track) -> Option<ReleaseGroup> {
    let json: Value = serde_json::from_str(body).ok()?;
    let groups = json.get("release-groups")?.as_array()?;
    let wanted_album = normalize(&track.album);
    let wanted_artist = normalize(&track.artist);
    let mut best: Option<(i64, ReleaseGroup)> = None;

    for group in groups {
        let id = group.get("id")?.as_str()?.to_string();
        let title = group
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let mut score = group.get("score").and_then(Value::as_i64).unwrap_or(0);
        if normalize(&title) == wanted_album {
            score += 25;
        }
        if group
            .get("primary-type")
            .and_then(Value::as_str)
            .is_some_and(|kind| kind.eq_ignore_ascii_case("album"))
        {
            score += 10;
        }
        let artist_matches = group
            .get("artist-credit")
            .and_then(Value::as_array)
            .map(|credits| {
                credits.iter().any(|credit| {
                    credit
                        .get("name")
                        .and_then(Value::as_str)
                        .map(|name| normalize(name) == wanted_artist)
                        .unwrap_or(false)
                        || credit
                            .get("artist")
                            .and_then(|artist| artist.get("name"))
                            .and_then(Value::as_str)
                            .map(|name| normalize(name) == wanted_artist)
                            .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        if artist_matches {
            score += 20;
        }
        if score < 65 {
            continue;
        }
        if best.as_ref().is_none_or(|(best_score, _)| score > *best_score) {
            best = Some((
                score,
                ReleaseGroup { id, title },
            ));
        }
    }

    best.map(|(_, group)| group)
}

fn musicbrainz_get(url: &str) -> Result<String, FetchError> {
    // Hold the lock while waiting so concurrent lookups queue up instead of
    // bursting past MusicBrainz's rate limit.
    let mut last = MUSICBRAINZ_THROTTLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(previous) = *last {
        let elapsed = previous.elapsed();
        if elapsed < Duration::from_millis(1100) {
            std::thread::sleep(Duration::from_millis(1100) - elapsed);
        }
    }
    let result = http_get(url, MUSICBRAINZ_USER_AGENT, 12);
    if result.is_err() {
        // One short retry helps with transient 503 responses.
        std::thread::sleep(Duration::from_millis(1200));
        let retry = http_get(url, MUSICBRAINZ_USER_AGENT, 12);
        *last = Some(Instant::now());
        return retry
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .map_err(FetchError::Network);
    }
    *last = Some(Instant::now());
    result
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .map_err(FetchError::Network)
}

fn http_get(url: &str, user_agent: &str, timeout_secs: u64) -> Result<Vec<u8>, String> {
    let mut command = std::process::Command::new("curl");
    command.args([
        "--silent",
        "--show-error",
        "--fail",
        "--location",
        "--compressed",
        "--max-time",
        &timeout_secs.to_string(),
        "--user-agent",
        user_agent,
        url,
    ]);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
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
    Ok(output.stdout)
}

fn normalize(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace(['（', '）', '【', '】', '《', '》'], "")
        .replace(['(', ')', '[', ']'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn track() -> Track {
        Track {
            id: 1,
            path: PathBuf::from("song.flac"),
            title: "晴天".into(),
            artist: "周杰伦".into(),
            album: "叶惠美".into(),
            genre: String::new(),
            year: Some(2003),
            track_number: None,
            duration: 269.0,
            added_at: 0,
            modified: 0,
            cue_source: None,
            cue_start: None,
            cue_end: None,
        }
    }

    #[test]
    fn picks_release_group_with_matching_artist_and_album() {
        let body = r#"{
            "release-groups": [
                {"id":"wrong","title":"叶惠美","score":60,"primary-type":"Album","artist-credit":[{"name":"Other"}]},
                {"id":"right","title":"叶惠美","score":80,"primary-type":"Album","artist-credit":[{"name":"周杰伦"}]}
            ]
        }"#;
        let selected = select_release_group(body, &track()).expect("candidate");
        assert_eq!(selected.id, "right");
    }

    #[test]
    fn rejects_low_confidence_results() {
        let body = r#"{"release-groups":[{"id":"x","title":"Something","score":10,"primary-type":"Single"}]}"#;
        assert!(select_release_group(body, &track()).is_none());
    }
}
