//! Minimal M3U/M3U8 playlist support.
//!
//! Paths are resolved relative to the playlist file, which is what VLC,
//! foobar2000 and most portable players expect.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Reads an M3U or M3U8 file and returns its media paths in order.
pub fn parse_file(path: &Path) -> Result<Vec<PathBuf>> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    Ok(parse(&text, path.parent().unwrap_or_else(|| Path::new("."))))
}

pub fn parse(text: &str, base: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    for raw in text.lines() {
        let line = raw.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let path = Path::new(line);
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            base.join(path)
        };
        result.push(resolved);
    }
    result
}

/// Writes paths as UTF-8 M3U8.
pub fn write_file(path: &Path, paths: &[PathBuf], relative_to: Option<&Path>) -> Result<()> {
    let mut text = String::from("#EXTM3U\n");
    for media in paths {
        let shown = relative_to
            .and_then(|base| media.strip_prefix(base).ok())
            .unwrap_or(media);
        text.push_str(&shown.to_string_lossy());
        text.push('\n');
    }
    fs::write(path, text).with_context(|| format!("write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comments_blank_lines_and_relative_paths() {
        let paths = parse(
            "#EXTM3U\n#EXTINF:123,Artist - Title\nmusic/a.mp3\n\n../b.flac\n",
            Path::new("C:/lists"),
        );
        assert_eq!(
            paths,
            vec![
                PathBuf::from("C:/lists/music/a.mp3"),
                PathBuf::from("C:/lists/../b.flac")
            ]
        );
    }

    #[test]
    fn writes_bom_free_utf8_header() {
        let dir = std::env::temp_dir().join(format!("rplayer-m3u-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("test.m3u8");
        write_file(&file, &[PathBuf::from("C:/music/中文.mp3")], None).unwrap();
        let text = fs::read_to_string(&file).unwrap();
        assert!(text.starts_with("#EXTM3U\n"));
        assert!(text.contains("中文.mp3"));
        let _ = fs::remove_dir_all(&dir);
    }
}
