//! Patch for `plugin.video.crunchyroll`: `Player.GetItem` returns stub
//! metadata during playback (`type: "unknown"`, no season/episode/titles),
//! so the `"{series} - S{SS}E{EE} - {episode}"` label is parsed back into
//! episode fields and artwork is resolved via `Files.GetDirectory`.
//!
//! Cache contract: one entry per instance + series, hits cost 0 RPCs,
//! misses cost at most 2 directory RPCs, entries expire after [`CACHE_TTL`].

use serde::Deserialize;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Addon id this module patches.
pub const ADDON_ID: &str = "plugin.video.crunchyroll";

/// Series entry lifetime. Long enough to never refetch mid-binge, short
/// enough that rotated posters and transient login failures recover.
pub const CACHE_TTL: Duration = Duration::from_secs(3 * 60 * 60);

/// Properties requested for every `Files.GetDirectory` call.
pub const DIRECTORY_PROPERTIES: &[&str] = &[
    "title",
    "showtitle",
    "season",
    "episode",
    "plot",
    "art",
    "thumbnail",
    "fanart",
    "file",
];

/// True when an addon id belongs to the patched Crunchyroll addon.
pub fn is_crunchyroll_addon(addon_id: Option<&str>) -> bool {
    addon_id.is_some_and(|a| a == ADDON_ID)
}

/// `S01E07` -> `(1, 7)`. Case-insensitive, no padding required.
pub fn parse_season_episode_token(token: &str) -> Option<(i32, i32)> {
    let t = token.trim();
    if t.len() < 4 {
        return None;
    }
    let mut chars = t.chars();
    let first = chars.next()?;
    if first != 'S' && first != 's' {
        return None;
    }
    let rest: String = chars.collect();
    let e_pos = rest.find(['E', 'e'])?;
    let (s_part, e_part_with) = rest.split_at(e_pos);
    let e_part = &e_part_with[1..];
    if s_part.is_empty() || e_part.is_empty() {
        return None;
    }
    if !s_part.bytes().all(|b| b.is_ascii_digit()) || !e_part.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let season: i32 = s_part.parse().ok()?;
    let episode: i32 = e_part.parse().ok()?;
    if season < 0 || episode < 0 {
        return None;
    }
    Some((season, episode))
}

/// Parse `"{series} - S{SS}E{EE} - {episode}"` into
/// `(series, season, episode, episode_title)`. Last `SxxEyy` token wins so
/// titles containing `" - "` still split correctly.
pub fn parse_label(label: &str) -> Option<(String, i32, i32, String)> {
    let label = label.trim();
    if label.is_empty() {
        return None;
    }
    let parts: Vec<&str> = label.split(" - ").collect();
    if parts.len() < 3 {
        return None;
    }
    let mut token_idx: Option<usize> = None;
    let mut season = 0;
    let mut episode = 0;
    for (i, part) in parts.iter().enumerate() {
        if let Some((s, e)) = parse_season_episode_token(part) {
            token_idx = Some(i);
            season = s;
            episode = e;
        }
    }
    let idx = token_idx?;
    if idx == 0 || idx + 1 >= parts.len() {
        return None;
    }
    let show = parts[..idx].join(" - ").trim().to_string();
    let ep_title = parts[idx + 1..].join(" - ").trim().to_string();
    if show.is_empty() || ep_title.is_empty() {
        return None;
    }
    Some((show, season, episode, ep_title))
}

/// Ids parsed from a playback path. `series` is `None` for movie plays
/// (`.../video/{episode}/{stream}`).
pub struct PlaybackIds {
    pub series: Option<String>,
    pub episode: String,
}

/// Parse `.../video/{series}/{episode}/{stream}` or
/// `.../video/{episode}/{stream}`. Anything else returns `None`.
pub fn playback_ids(file: &str) -> Option<PlaybackIds> {
    let rest = file.strip_prefix("plugin://plugin.video.crunchyroll/")?;
    let rest = rest.trim_start_matches('/');
    let mut parts = rest
        .split('/')
        .map(|s| s.split(['?', '#']).next().unwrap_or(s));
    if parts.next()? != "video" {
        return None;
    }
    let a = parts.next()?.trim();
    let b = parts.next()?.trim();
    if a.is_empty() || b.is_empty() {
        return None;
    }
    match parts.next().map(str::trim) {
        Some(stream) if !stream.is_empty() => Some(PlaybackIds {
            series: Some(a.to_string()),
            episode: b.to_string(),
        }),
        _ => Some(PlaybackIds {
            series: None,
            episode: a.to_string(),
        }),
    }
}

/// `.../video/{series}/{episode}/{stream}` -> `Some(series)`.
/// Movie paths and others return `None`.
pub fn series_id_from_file(file: &str) -> Option<String> {
    playback_ids(file)?.series
}

/// Public watch page for an episode. `GET /watch/{id}` redirects to the
/// full slugged episode URL.
pub fn watch_url(episode_id: &str) -> String {
    format!("https://www.crunchyroll.com/watch/{episode_id}")
}

/// Public series page.
pub fn series_url(series_id: &str) -> String {
    format!("https://www.crunchyroll.com/series/{series_id}")
}

/// Series path (season dirs + series poster art).
pub fn series_directory(series_id: &str) -> String {
    format!("plugin://plugin.video.crunchyroll/series/{series_id}")
}

/// Season path (episode plots + art).
pub fn season_directory(series_id: &str, season_id: &str) -> String {
    format!("plugin://plugin.video.crunchyroll/series/{series_id}/{season_id}")
}

/// One `Files.GetDirectory` entry.
#[derive(Deserialize, Debug, Clone, Default)]
#[allow(dead_code)] // API model: fields exist to parse directory listings
pub struct DirectoryFile {
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub filetype: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub showtitle: Option<String>,
    #[serde(default)]
    pub season: Option<i32>,
    #[serde(default)]
    pub episode: Option<i32>,
    #[serde(default)]
    pub plot: Option<String>,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub fanart: Option<String>,
    #[serde(default)]
    pub art: HashMap<String, String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct FilesDirectoryResult {
    #[serde(default)]
    pub files: Vec<DirectoryFile>,
}

/// Empty listings and "Login failed" placeholders count as misses (callers
/// negative-cache them until [`CACHE_TTL`]).
pub fn is_login_failure(files: &[DirectoryFile]) -> bool {
    if files.is_empty() {
        return true;
    }
    files.iter().any(|f| f.label.contains("Login failed"))
}

/// Cached series: poster/fanart, season-id map, per-episode plots.
/// Keyed by `"{instance_idx}:crunchy:{series_id}"`.
#[derive(Clone, Debug)]
pub struct CrunchySeriesEntry {
    pub poster: Option<String>,
    pub fanart: Option<String>,
    pub seasons: HashMap<i32, String>,
    pub episode_plots: HashMap<String, String>,
    fetched_at: Instant,
}

impl Default for CrunchySeriesEntry {
    fn default() -> Self {
        Self {
            poster: None,
            fanart: None,
            seasons: HashMap::new(),
            episode_plots: HashMap::new(),
            fetched_at: Instant::now(),
        }
    }
}

impl CrunchySeriesEntry {
    pub fn is_fresh(&self) -> bool {
        self.fetched_at.elapsed() < CACHE_TTL
    }

    pub fn plot_for(&self, file: &str) -> Option<&str> {
        self.episode_plots.get(file).map(String::as_str)
    }

    pub fn season_id(&self, season: i32) -> Option<&str> {
        self.seasons.get(&season).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn label_parsing() {
        assert_eq!(
            parse_label(
                "The Duke's Son Claims He Won't Love Me, Yet Showers Me with Adoration - S01E07 - An Unexpected Gift"
            ),
            Some((
                "The Duke's Son Claims He Won't Love Me, Yet Showers Me with Adoration".to_string(),
                1,
                7,
                "An Unexpected Gift".to_string()
            ))
        );
        // Show titles containing " - " re-join correctly.
        assert_eq!(
            parse_label("Re:Zero - Starting Life in Another World - S03E12 - The End"),
            Some((
                "Re:Zero - Starting Life in Another World".to_string(),
                3,
                12,
                "The End".to_string()
            ))
        );
        // Episode titles containing " - " survive too.
        assert_eq!(
            parse_label("Show - S1E2 - Part 1 - Finale"),
            Some(("Show".to_string(), 1, 2, "Part 1 - Finale".to_string()))
        );
        // Lowercase + unpadded still parses.
        assert_eq!(
            parse_label("Show - s1e2 - Title"),
            Some(("Show".to_string(), 1, 2, "Title".to_string()))
        );
        // Non-matching labels stay None.
        assert_eq!(parse_label("Just a movie title"), None);
        assert_eq!(parse_label("Show - S01 - Title"), None);
        assert_eq!(parse_label("Show - S01E07"), None);
        assert_eq!(parse_label(""), None);
        assert_eq!(parse_season_episode_token("S01E07"), Some((1, 7)));
        assert_eq!(parse_season_episode_token("s1e2"), Some((1, 2)));
        assert_eq!(parse_season_episode_token("S01"), None);
        assert_eq!(parse_season_episode_token("E07"), None);
    }

    #[test]
    fn series_id_parsing() {
        assert_eq!(
            series_id_from_file(
                "plugin://plugin.video.crunchyroll/video/GT00378118/GE00378555JAJP/GE00378555JAJPV"
            )
            .as_deref(),
            Some("GT00378118")
        );
        // Movie paths have no series segment.
        assert_eq!(
            series_id_from_file(
                "plugin://plugin.video.crunchyroll/video/GE00378555JAJP/GE00378555JAJPV"
            ),
            None
        );
        // Non-video / foreign addon paths.
        assert_eq!(
            series_id_from_file("plugin://plugin.video.crunchyroll/series/GT00378118"),
            None
        );
        assert_eq!(
            series_id_from_file("plugin://plugin.video.youtube/play/?video_id=abc"),
            None
        );
        assert_eq!(series_id_from_file("smb://server/share/movie.mkv"), None);
        assert_eq!(series_id_from_file(""), None);
    }

    #[test]
    fn playback_ids_shared_parse() {
        let ids = playback_ids(
            "plugin://plugin.video.crunchyroll/video/GT00378118/GE00378555JAJP/GE00378555JAJPV",
        )
        .expect("parses");
        assert_eq!(ids.series.as_deref(), Some("GT00378118"));
        assert_eq!(ids.episode, "GE00378555JAJP");
        let movie =
            playback_ids("plugin://plugin.video.crunchyroll/video/GE00378555JAJP/GE00378555JAJPV")
                .expect("parses");
        assert_eq!(movie.series, None);
        assert_eq!(movie.episode, "GE00378555JAJP");
        assert!(playback_ids("plugin://plugin.video.crunchyroll/series/GT00378118").is_none());
    }

    #[test]
    fn episode_watch_and_series_urls() {
        let ids = playback_ids(
            "plugin://plugin.video.crunchyroll/video/GT00378118/GE00378555JAJP/GE00378555JAJPV",
        )
        .expect("parses");
        assert_eq!(
            watch_url(&ids.episode),
            "https://www.crunchyroll.com/watch/GE00378555JAJP"
        );
        assert_eq!(
            series_url("GT00378118"),
            "https://www.crunchyroll.com/series/GT00378118"
        );
    }

    #[test]
    fn directory_listing_parses() {
        let raw = r#"{"files": [{
            "art": {"poster": "image://https%3a%2f%2fx%2fposter.png/"},
            "episode": 0, "season": 1,
            "file": "plugin://plugin.video.crunchyroll/series/GT00378118/GS00378134JAJP",
            "filetype": "directory", "label": "Season 1"
        }]}"#;
        let res: FilesDirectoryResult = serde_json::from_str(raw).expect("parses");
        assert_eq!(res.files.len(), 1);
        assert_eq!(res.files[0].season, Some(1));
        assert_eq!(
            res.files[0].art.get("poster").map(String::as_str),
            Some("image://https%3a%2f%2fx%2fposter.png/")
        );
    }

    #[test]
    fn login_failure_detection() {
        let failed = vec![DirectoryFile {
            label: "Login failed".to_string(),
            ..Default::default()
        }];
        assert!(is_login_failure(&failed));
        assert!(is_login_failure(&[]));
        let ok = vec![DirectoryFile {
            label: "Season 1".to_string(),
            plot: Some("plot".to_string()),
            ..Default::default()
        }];
        assert!(!is_login_failure(&ok));
    }

    #[test]
    fn cache_entry_ttl() {
        let fresh = CrunchySeriesEntry::default();
        assert!(fresh.is_fresh());
        let mut stale = CrunchySeriesEntry::default();
        // Simulate an entry fetched over TTL ago.
        stale.fetched_at = Instant::now() - (CACHE_TTL + Duration::from_secs(1));
        assert!(!stale.is_fresh());
    }

    #[test]
    fn cache_entry_lookups() {
        let mut entry = CrunchySeriesEntry::default();
        entry.seasons.insert(1, "GS00378134JAJP".to_string());
        entry.episode_plots.insert(
            "plugin://plugin.video.crunchyroll/video/GT00378118/GE00378555JAJP/GE00378555JAJPV"
                .to_string(),
            "A plot".to_string(),
        );
        assert_eq!(entry.season_id(1), Some("GS00378134JAJP"));
        assert_eq!(entry.season_id(2), None);
        assert_eq!(
            entry.plot_for(
                "plugin://plugin.video.crunchyroll/video/GT00378118/GE00378555JAJP/GE00378555JAJPV"
            ),
            Some("A plot")
        );
        assert_eq!(entry.plot_for("plugin://other"), None);
    }

    #[test]
    fn directory_paths() {
        assert_eq!(
            series_directory("GT00378118"),
            "plugin://plugin.video.crunchyroll/series/GT00378118"
        );
        assert_eq!(
            season_directory("GT00378118", "GS00378134JAJP"),
            "plugin://plugin.video.crunchyroll/series/GT00378118/GS00378134JAJP"
        );
    }
}
