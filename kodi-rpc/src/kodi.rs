use serde::{de::Visitor, Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// JSON-RPC envelopes
// ---------------------------------------------------------------------------

#[derive(Serialize, Debug)]
pub struct JsonRpcRequest<T: Serialize> {
    pub jsonrpc: &'static str,
    pub method: &'static str,
    pub params: T,
    pub id: &'static str,
}

impl<T: Serialize> JsonRpcRequest<T> {
    pub fn new(method: &'static str, params: T, id: &'static str) -> Self {
        Self {
            jsonrpc: "2.0",
            method,
            params,
            id,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct JsonRpcResponse<T> {
    pub result: Option<T>,
    #[allow(dead_code)]
    pub error: Option<JsonRpcError>,
}

#[derive(Deserialize, Debug)]
pub struct JsonRpcError {
    #[allow(dead_code)]
    pub code: i32,
    #[allow(dead_code)]
    pub message: String,
}

// ---------------------------------------------------------------------------
// Player.GetActivePlayers
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug, Clone)]
pub struct ActivePlayer {
    pub playerid: i32,
    #[serde(rename = "type")]
    pub player_type: String,
}

// ---------------------------------------------------------------------------
// Player.GetItem
// ---------------------------------------------------------------------------

/// Properties for every `Player.GetItem` call. Kodi ignores ones that don't
/// apply; plugins usually fill in just `label` + `file` (`type: "unknown"`).
pub fn get_item_properties() -> Vec<&'static str> {
    vec![
        "title",
        "showtitle",
        "season",
        "episode",
        "artist",
        "album",
        "genre",
        "year",
        "rating",
        "plot",
        "file",
        "thumbnail",
        "fanart",
        "studio",
        "director",
        "tagline",
        "mpaa",
        "duration",
        "originaltitle",
        "albumartist",
        "track",
        "channeltype",
        "channelnumber",
        "tvshowid",
    ]
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlayerGetItemResult {
    #[serde(default)]
    pub item: KodiItem,
}

fn default_type() -> String {
    "unknown".to_string()
}

fn default_label() -> String {
    String::new()
}

/// Accepts `String | Vec<String> | null`.
fn string_or_vec<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct SOV;

    impl<'de> Visitor<'de> for SOV {
        type Value = Option<Vec<String>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string, an array of strings, or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_some<D2>(self, deserializer: D2) -> Result<Self::Value, D2::Error>
        where
            D2: Deserializer<'de>,
        {
            deserializer.deserialize_any(SOVInner)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if v.is_empty() {
                Ok(None)
            } else {
                Ok(Some(vec![v.to_owned()]))
            }
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if v.is_empty() {
                Ok(None)
            } else {
                Ok(Some(vec![v]))
            }
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut out = Vec::new();
            while let Some(el) = seq.next_element::<String>()? {
                if !el.is_empty() {
                    out.push(el);
                }
            }
            if out.is_empty() {
                Ok(None)
            } else {
                Ok(Some(out))
            }
        }
    }

    struct SOVInner;

    impl<'de> Visitor<'de> for SOVInner {
        type Value = Option<Vec<String>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string, an array of strings, or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if v.is_empty() {
                Ok(None)
            } else {
                Ok(Some(vec![v.to_owned()]))
            }
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if v.is_empty() {
                Ok(None)
            } else {
                Ok(Some(vec![v]))
            }
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut out = Vec::new();
            while let Some(el) = seq.next_element::<String>()? {
                if !el.is_empty() {
                    out.push(el);
                }
            }
            if out.is_empty() {
                Ok(None)
            } else {
                Ok(Some(out))
            }
        }
    }

    deserializer.deserialize_option(SOV)
}

#[derive(Deserialize, Debug, Clone, Default)]
#[allow(dead_code)] // API model: fields exist to parse Kodi's responses
pub struct KodiItem {
    #[serde(default = "default_label")]
    pub label: String,
    #[serde(rename = "type", default = "default_type")]
    pub item_type: String,
    /// Library ID of the item itself. Absent for plugins.
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub tvshowid: Option<i32>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub showtitle: Option<String>,
    #[serde(default)]
    pub season: Option<i32>,
    #[serde(default)]
    pub episode: Option<i32>,
    #[serde(default, deserialize_with = "string_or_vec")]
    pub artist: Option<Vec<String>>,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default, alias = "genre", deserialize_with = "string_or_vec")]
    pub genres: Option<Vec<String>>,
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub rating: Option<f64>,
    #[serde(default)]
    pub plot: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub fanart: Option<String>,
    #[serde(default, deserialize_with = "string_or_vec")]
    pub studio: Option<Vec<String>>,
    #[serde(default, deserialize_with = "string_or_vec")]
    pub director: Option<Vec<String>>,
    #[serde(default)]
    pub duration: Option<i64>,
    #[serde(default)]
    pub originaltitle: Option<String>,
    #[serde(default)]
    pub tagline: Option<String>,
    #[serde(default)]
    pub channeltype: Option<String>,
}

// ---------------------------------------------------------------------------
// Player.GetProperties
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug, Clone, Default)]
#[allow(dead_code)] // API model: fields exist to parse Kodi's responses
pub struct KodiTime {
    #[serde(default)]
    pub hours: i64,
    #[serde(default)]
    pub minutes: i64,
    #[serde(default)]
    pub seconds: i64,
    #[serde(default)]
    pub milliseconds: i64,
}

impl KodiTime {
    pub fn total_seconds(&self) -> i64 {
        self.hours * 3600 + self.minutes * 60 + self.seconds
    }
}

fn de_speed<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    struct SpeedV;
    impl<'de> Visitor<'de> for SpeedV {
        type Value = f64;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a number")
        }
        fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<f64, E> {
            Ok(v as f64)
        }
        fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<f64, E> {
            Ok(v as f64)
        }
        fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<f64, E> {
            Ok(v)
        }
    }
    deserializer.deserialize_any(SpeedV)
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)] // API model: fields exist to parse Kodi's responses
pub struct PlayerGetPropertiesResult {
    #[serde(default, deserialize_with = "de_speed")]
    pub speed: f64,
    #[serde(default)]
    pub time: KodiTime,
    #[serde(default)]
    pub totaltime: KodiTime,
    #[serde(default)]
    pub percentage: Option<f64>,
}

// ---------------------------------------------------------------------------
// Library artwork: Player.GetItem only has the episode still, so posters
// come from GetSeasons (season -> series fallback) / GetMovieDetails.
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug, Clone)]
pub struct SeasonsResult {
    #[serde(default)]
    pub seasons: Vec<SeasonInfo>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SeasonInfo {
    #[serde(default)]
    pub season: i32,
    #[serde(default)]
    pub art: HashMap<String, String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TVShowDetailsResult {
    pub tvshowdetails: ArtHolder,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MovieDetailsResult {
    pub moviedetails: ArtHolder,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct ArtHolder {
    #[serde(default)]
    pub art: HashMap<String, String>,
    /// Provider IDs. Values can be numbers.
    #[serde(default)]
    pub uniqueid: HashMap<String, serde_json::Value>,
    /// Trailer URL when scraped. Often a `plugin://` URL.
    #[serde(default)]
    pub trailer: Option<String>,
}

fn id_string(value: Option<&serde_json::Value>) -> Option<String> {
    let v = value?;
    if let Some(s) = v.as_str() {
        let s = s.trim();
        if s.is_empty() {
            None
        } else {
            Some(s.to_string())
        }
    } else if v.is_number() {
        Some(v.to_string())
    } else {
        None
    }
}

pub fn tmdb_id(details: &ArtHolder) -> Option<String> {
    id_string(details.uniqueid.get("tmdb"))
}

pub fn tvdb_id(details: &ArtHolder) -> Option<String> {
    id_string(details.uniqueid.get("tvdb"))
}

pub fn imdb_id(details: &ArtHolder) -> Option<String> {
    let imdb = id_string(details.uniqueid.get("imdb"))?;
    if imdb.starts_with("tt") {
        Some(imdb)
    } else {
        None
    }
}
pub fn tvdb_url(kind: &str, details: &ArtHolder) -> Option<String> {
    let tvdb = tvdb_id(details)?;
    let segment = match kind {
        "tv" => "series",
        "movie" => "movie",
        _ => return None,
    };
    Some(format!("https://thetvdb.com/dereferrer/{segment}/{tvdb}"))
}

pub fn wetrakr_url(kind: &str, details: &ArtHolder) -> Option<String> {
    let tmdb = tmdb_id(details)?;
    let segment = match kind {
        "tv" => "shows",
        "movie" => "movies",
        _ => return None,
    };
    Some(format!("https://wetrakr.com/tmdb/{segment}/{tmdb}"))
}

pub fn info_urls(kind: &str, details: &ArtHolder) -> Vec<ExternalUrl> {
    let mut urls = Vec::new();
    if let Some(tmdb) = tmdb_id(details) {
        urls.push(ExternalUrl {
            name: "The Movie DB".to_string(),
            url: format!("https://www.themoviedb.org/{kind}/{tmdb}"),
        });
    }
    if let Some(url) = tvdb_url(kind, details) {
        urls.push(ExternalUrl {
            name: "The TV DB".to_string(),
            url,
        });
    }
    if let Some(url) = wetrakr_url(kind, details) {
        urls.push(ExternalUrl {
            name: "WeTrakr".to_string(),
            url,
        });
    }
    if let Some(imdb) = imdb_id(details) {
        urls.push(ExternalUrl {
            name: "IMDb".to_string(),
            url: format!("https://www.imdb.com/title/{imdb}/"),
        });
    }
    urls
}

/// Pick the `poster` entry out of an `art` dict. Blank entries don't count.
pub fn pick_art_poster(art: &HashMap<String, String>) -> Option<String> {
    art.get("poster")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Find the poster for `season` in a GetSeasons response.
pub fn pick_season_poster(seasons: &[SeasonInfo], season: i32) -> Option<String> {
    seasons
        .iter()
        .find(|s| s.season == season)
        .and_then(|s| pick_art_poster(&s.art))
}

// Normalized session

#[derive(Debug)]
pub struct Session {
    pub now_playing_item: NowPlayingItem,
    pub play_state: PlayState,
    pub item_id: String,
    /// Owning Kodi instance index.
    pub source: usize,
}

impl Session {
    /// Formats artists with comma separation and a final "and" before the last name.
    pub fn format_artists(&self) -> String {
        let default = vec!["".to_string()];
        let artists_vec = self.now_playing_item.artists.as_ref().unwrap_or(&default);
        let mut artists = String::new();

        for i in 0..artists_vec.len() {
            if i == 0 {
                artists += &artists_vec[i];
                continue;
            }

            if i == artists_vec.len() - 1 {
                artists += &format!(" and {}", artists_vec[i]);
                continue;
            }

            artists += &format!(", {}", artists_vec[i]);
        }

        artists
    }

    pub fn get_time(&self) -> Result<PlayTime, SystemTimeError> {
        if self.now_playing_item.media_type == MediaType::LiveTv {
            return Ok(PlayTime::None);
        }

        if self.play_state.is_paused {
            return Ok(PlayTime::Paused);
        }

        match (
            self.play_state.position_ticks,
            self.now_playing_item.run_time_ticks,
        ) {
            (Some(pos), Some(total)) if total > 0 && pos >= 0 => {
                let ticks_to_seconds = 10000000;
                let position = pos / ticks_to_seconds;
                let runtime = total / ticks_to_seconds;
                // Unknown-length streams (common for plugins): show without
                // timestamps instead of pretending to be paused.
                if runtime <= 0 {
                    return Ok(PlayTime::None);
                }
                let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
                Ok(PlayTime::Some(now - position, now + (runtime - position)))
            }
            _ => Ok(PlayTime::None),
        }
    }
}

#[derive(PartialEq)]
pub enum PlayTime {
    Some(i64, i64),
    Paused,
    None,
}

/// Contains information about buttons displayed in Discord
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Button {
    /// What the name should be showed as in Discord.
    pub name: String,
    /// What clicking it should point to in Discord.
    pub url: String,
}

impl Default for Button {
    fn default() -> Self {
        Self {
            name: String::from("dynamic"),
            url: String::from("dynamic"),
        }
    }
}

impl Button {
    pub fn new(name: String, url: String) -> Self {
        Self { name, url }
    }

    pub(crate) fn is_dynamic(&self) -> bool {
        self.name == "dynamic" && self.url == "dynamic"
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ExternalUrl {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Normalized model: all fields kept for display/config use
pub struct NowPlayingItem {
    pub name: String,
    pub media_type: MediaType,
    pub id: String,
    pub run_time_ticks: Option<i64>,
    pub production_year: Option<i64>,
    pub genres: Option<Vec<String>>,
    pub external_urls: Option<Vec<ExternalUrl>>,
    pub critic_rating: Option<i64>,
    pub community_rating: Option<f64>,
    pub original_title: Option<String>,
    pub path: Option<String>,
    pub parent_index_number: Option<i32>,
    pub index_number: Option<i32>,
    pub index_number_end: Option<i32>,
    pub series_name: Option<String>,
    pub series_id: Option<String>,
    pub series_studio: Option<String>,
    pub artists: Option<Vec<String>>,
    pub extra_type: Option<String>,
    pub album_id: Option<String>,
    pub album: Option<String>,
    pub label: String,
    pub file: String,
    pub addon_id: Option<String>,
    pub is_plugin: bool,
    pub thumbnail: Option<String>,
    pub fanart: Option<String>,
    pub poster: Option<String>,
    pub plot: Option<String>,
    pub player_kind: String,
    pub tmdb_id: Option<String>,
    pub tvdb_id: Option<String>,
    pub imdb_id: Option<String>,
    pub trailer_url: Option<String>,
}

impl NowPlayingItem {
    pub fn from_kodi(
        item: &KodiItem,
        props: &PlayerGetPropertiesResult,
        player_kind: &str,
    ) -> Option<Self> {
        let file = item.file.clone().unwrap_or_default();
        let is_plugin = file.starts_with("plugin://");

        let mut media_type = MediaType::from_kodi(&item.item_type, &file);

        // Pictures carry no watch status; anything else with a label or
        // file still counts as something playing.
        if media_type == MediaType::None {
            let title_empty = item
                .title
                .as_ref()
                .map(|t| t.trim().is_empty())
                .unwrap_or(true);
            if item.label.trim().is_empty() && file.trim().is_empty() && title_empty {
                return None;
            }
        }

        // Title resolution: title -> label -> file name.
        let raw_title = item
            .title
            .clone()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| item.label.clone());
        let mut name = strip_kodi_tags(&raw_title);
        if name.trim().is_empty() {
            name = file_name_fallback(&file).unwrap_or_else(|| file.clone());
        }
        if name.trim().is_empty() {
            return None;
        }

        let addon_id = extract_addon_id(&file);

        let mut season = item.season.filter(|s| *s >= 0);
        let mut episode = item.episode.filter(|e| *e >= 0);

        let mut showtitle = item
            .showtitle
            .clone()
            .filter(|s| !s.trim().is_empty())
            .map(|s| strip_kodi_tags(&s));

        // Crunchyroll fallback: fill any missing episode fields from the
        // `"{series} - S{SS}E{EE} - {episode}"` label. Fully rich items skip
        // parsing; partial stubs get only their gaps filled.
        if crate::crunchyroll::is_crunchyroll_addon(addon_id.as_deref())
            && (media_type == MediaType::Unknown
                || season.is_none()
                || episode.is_none()
                || showtitle.is_none())
        {
            let stripped_label = strip_kodi_tags(&item.label);
            let from_name = crate::crunchyroll::parse_label(&name);
            if let Some((show, s_num, e_num, ep_title)) = from_name
                .clone()
                .or_else(|| crate::crunchyroll::parse_label(&stripped_label))
            {
                if media_type == MediaType::Unknown {
                    media_type = MediaType::Episode;
                }
                if season.is_none() {
                    season = Some(s_num);
                }
                if episode.is_none() {
                    episode = Some(e_num);
                }
                if showtitle.is_none() {
                    showtitle = Some(show);
                }
                if from_name.is_some() {
                    name = ep_title;
                }
            }
        }

        let studio_joined = item
            .studio
            .clone()
            .filter(|v| !v.is_empty())
            .map(|v| v.join(", "));

        // Kodi reports seconds; presence math uses Jellyfin ticks (100ns).
        let runtime_ticks = {
            let secs = item
                .duration
                .unwrap_or_else(|| props.totaltime.total_seconds());
            if secs > 0 {
                Some(secs * 10_000_000)
            } else if props.totaltime.total_seconds() > 0 {
                Some(props.totaltime.total_seconds() * 10_000_000)
            } else {
                None
            }
        };

        // Dynamic buttons come from series/movie library info. Crunchyroll
        // items also get Series/Watch links from the playback path, stub or not.
        let external_urls = if crate::crunchyroll::is_crunchyroll_addon(addon_id.as_deref()) {
            crate::crunchyroll::playback_ids(&file).map(|ids| {
                let mut urls = vec![ExternalUrl {
                    name: "Watch Episode".to_string(),
                    url: crate::crunchyroll::watch_url(&ids.episode),
                }];
                if let Some(series) = ids.series {
                    urls.push(ExternalUrl {
                        name: "View Series Info".to_string(),
                        url: crate::crunchyroll::series_url(&series),
                    });
                }
                urls
            })
        } else {
            None
        };

        let id = if file.is_empty() {
            name.clone()
        } else {
            file.clone()
        };

        Some(Self {
            name: name.clone(),
            media_type,
            id,
            run_time_ticks: runtime_ticks,
            production_year: item.year.map(|y| y as i64),
            genres: item.genres.clone(),
            external_urls,
            critic_rating: None,
            community_rating: item.rating,
            original_title: item.originaltitle.clone().filter(|s| !s.is_empty()),
            path: if file.is_empty() {
                None
            } else {
                Some(file.clone())
            },
            parent_index_number: season,
            index_number: episode,
            index_number_end: None,
            series_name: showtitle,
            series_id: None,
            series_studio: studio_joined,
            artists: item.artist.clone(),
            extra_type: None,
            album_id: None,
            album: item.album.clone().filter(|a| !a.is_empty()),
            label: strip_kodi_tags(&item.label),
            file: file.clone(),
            addon_id,
            is_plugin,
            thumbnail: item.thumbnail.clone().filter(|t| !t.is_empty()),
            fanart: item.fanart.clone().filter(|f| !f.is_empty()),
            poster: None,
            plot: item.plot.clone().filter(|p| !p.is_empty()),
            player_kind: player_kind.to_string(),
            tmdb_id: None,
            tvdb_id: None,
            imdb_id: None,
            trailer_url: None,
        })
    }

    /// Host/addon for `{file-host}` templates.
    pub fn file_host_display(&self) -> String {
        if let Some(addon) = &self.addon_id {
            let short = addon.rsplit('.').next().unwrap_or(addon);
            return short.to_string();
        }
        if self.file.starts_with("http://") || self.file.starts_with("https://") {
            if let Ok(u) = url::Url::parse(&self.file) {
                if is_loopback_url(&self.file) {
                    // Local playback proxies (e.g. SlyGuy's 127.0.0.1 proxy)
                    // wrap the real stream in a `url` param — show its host.
                    if let Some(inner) = u
                        .query_pairs()
                        .find(|(k, _)| k == "url")
                        .map(|(_, v)| v.into_owned())
                    {
                        if let Ok(inner_u) = url::Url::parse(&inner) {
                            if let Some(h) = inner_u.host_str() {
                                return h.to_string();
                            }
                        }
                    }
                } else if let Some(h) = u.host_str() {
                    return h.to_string();
                }
            }
        }
        // Local files: show the file name.
        file_name_fallback(&self.file).unwrap_or_default()
    }
}

/// Content type. `Unknown` (e.g. `type: "unknown"` plugin streams) still
/// displays, unlike `None`.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MediaType {
    Movie,
    Episode,
    /// Live TV / PVR channel.
    LiveTv,
    Music,
    /// Kept for config compat; unused by Kodi.
    Book,
    /// Kept for config compat; unused by Kodi.
    AudioBook,
    /// Plugin / unclassified content. Still displayed.
    Unknown,
    /// Nothing usable (pictures, empty players).
    None,
}

impl MediaType {
    pub fn from_kodi(item_type: &str, file: &str) -> Self {
        let t = item_type.to_lowercase();
        match t.as_str() {
            "movie" => Self::Movie,
            "episode" => Self::Episode,
            "song" | "album" | "artist" | "musicvideo" => Self::Music,
            "channel" | "tvchannel" | "recording" => Self::LiveTv,
            // Pictures have no watch-status value.
            "picture" | "photo" | "image" => Self::None,
            "book" => Self::Book,
            "audiobook" => Self::AudioBook,
            // PVR streams sometimes surface as unknown with a pvr:// file.
            _ => {
                if file.starts_with("pvr://") {
                    return Self::LiveTv;
                }
                if (t == "unknown" || t.is_empty() || t == "file") && file.is_empty() {
                    return Self::None;
                }
                // Unrecognized/plugin types display instead of hiding.
                Self::Unknown
            }
        }
    }
}

impl Serialize for MediaType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            MediaType::Movie => serializer.serialize_unit_variant("MediaType", 0, "Movie"),
            MediaType::Episode => serializer.serialize_unit_variant("MediaType", 1, "Episode"),
            MediaType::LiveTv => serializer.serialize_unit_variant("MediaType", 2, "LiveTv"),
            MediaType::Music => serializer.serialize_unit_variant("MediaType", 3, "Music"),
            MediaType::Book => serializer.serialize_unit_variant("MediaType", 4, "Book"),
            MediaType::AudioBook => serializer.serialize_unit_variant("MediaType", 5, "AudioBook"),
            MediaType::Unknown => serializer.serialize_unit_variant("MediaType", 6, "Unknown"),
            MediaType::None => serializer.serialize_unit_variant("MediaType", 7, "None"),
        }
    }
}

impl<'de> Deserialize<'de> for MediaType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(MediaTypeVisitor)
    }
}

struct MediaTypeVisitor;

impl<'de> Visitor<'de> for MediaTypeVisitor {
    type Value = MediaType;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(MediaType::from(v.to_lowercase()))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(MediaType::from(v.to_lowercase()))
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(MediaType::from(v.to_lowercase()))
    }
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = match self {
            MediaType::Episode => "Episode",
            MediaType::LiveTv => "LiveTv",
            MediaType::Movie => "Movie",
            MediaType::Music => "Music",
            MediaType::Book => "Book",
            MediaType::AudioBook => "AudioBook",
            MediaType::Unknown => "Unknown",
            MediaType::None => "None",
        };
        write!(f, "{}", res)
    }
}

impl Default for MediaType {
    fn default() -> Self {
        Self::None
    }
}

impl From<&'static str> for MediaType {
    fn from(value: &'static str) -> Self {
        MediaType::from(value.to_string())
    }
}

impl From<String> for MediaType {
    fn from(value: String) -> Self {
        match value.to_lowercase().as_str() {
            "episode" => Self::Episode,
            "movie" => Self::Movie,
            "music" | "audio" | "song" | "musicvideo" => Self::Music,
            "livetv" | "tvchannel" | "channel" => Self::LiveTv,
            "book" => Self::Book,
            "audiobook" => Self::AudioBook,
            "unknown" | "plugin" => Self::Unknown,
            _ => Self::None,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct PlayState {
    pub is_paused: bool,
    pub position_ticks: Option<i64>,
}

impl PlayState {
    pub fn from_props(props: &PlayerGetPropertiesResult) -> Self {
        let pos_secs = props.time.total_seconds();
        Self {
            // Kodi: speed 0 == paused. Anything else (1, 2, -1 ..) counts as active.
            is_paused: props.speed.abs() < f64::EPSILON,
            position_ticks: Some(pos_secs * 10_000_000),
        }
    }
}

// ---------------------------------------------------------------------------
// Kodi label helpers — this is what makes 3rd-party plugins work.
// ---------------------------------------------------------------------------

/// Strip Kodi `[...]` formatting tags from labels.
pub fn strip_kodi_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '[' => in_tag = true,
            ']' => {
                in_tag = false;
            }
            _ => {
                if !in_tag {
                    out.push(ch);
                }
            }
        }
    }
    // Collapse whitespace the same way the display layer expects.
    out.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

/// `plugin://plugin.video.youtube/play/?v=...` -> `Some("plugin.video.youtube")`
pub fn extract_addon_id(file: &str) -> Option<String> {
    let rest = file.strip_prefix("plugin://")?;
    let end = rest
        .find(|c| c == '/' || c == '?' || c == '#')
        .unwrap_or(rest.len());
    let host = &rest[..end];
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

/// True for loopback URLs (unopenable by anyone else, including Discord).
pub fn is_loopback_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    [
        "http://localhost",
        "https://localhost",
        "http://127.0.0.1",
        "https://127.0.0.1",
        "http://[::1]",
        "https://[::1]",
    ]
    .iter()
    .any(|p| lower.starts_with(p))
}

/// Unwrap `image://<url-encoded>/` to the inner URL for remote files.
/// Local schemes return None.
pub fn unwrap_remote_image(remote: &str) -> Option<String> {
    let inner = remote.strip_prefix("image://")?.trim_end_matches('/');
    if inner.is_empty() {
        return None;
    }
    let decoded = urlencoding::decode(inner).ok()?.into_owned();
    if decoded.starts_with("http://") || decoded.starts_with("https://") {
        Some(decoded)
    } else {
        None
    }
}

fn file_name_fallback(file: &str) -> Option<String> {
    if file.is_empty() {
        return None;
    }
    let trimmed = file.trim_end_matches('/');
    let last = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
    // Strip query strings some plugins append.
    let last = last.split(['?', '#']).next().unwrap_or(last);
    let decoded = urlencoding::decode(last).unwrap_or_else(|_| last.into());
    let s = decoded.trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod kodi_tests {
    use super::*;

    #[test]
    fn strips_color_tags() {
        assert_eq!(
            strip_kodi_tags("Nyheterna 22.00 [COLOR aqua]º[/COLOR]"),
            "Nyheterna 22.00 º"
        );
        assert_eq!(strip_kodi_tags("[B]Bold[/B] title"), "Bold title");
    }

    #[test]
    fn extracts_addon() {
        assert_eq!(
            extract_addon_id("plugin://plugin.video.youtube/play/?video_id=abc"),
            Some("plugin.video.youtube".to_string())
        );
        assert_eq!(extract_addon_id("smb://server/share/movie.mkv"), None);
    }

    #[test]
    fn unknown_maps_for_plugins() {
        assert_eq!(
            MediaType::from_kodi("unknown", "plugin://plugin.video.foo/play"),
            MediaType::Unknown
        );
        assert_eq!(
            MediaType::from_kodi("unknown", "pvr://channels/tv/1"),
            MediaType::LiveTv
        );
    }

    #[test]
    fn season_poster_picking() {
        use std::collections::HashMap;
        let art = |poster: &str| {
            let mut m = HashMap::new();
            m.insert("poster".to_string(), poster.to_string());
            m.insert("fanart".to_string(), "image://fanart/".to_string());
            m
        };
        let seasons = vec![
            SeasonInfo {
                season: 0,
                art: art("image://specials-poster/"),
            },
            SeasonInfo {
                season: 1,
                art: art("image://s1-poster/"),
            },
            SeasonInfo {
                season: 2,
                art: HashMap::new(),
            },
        ];
        assert_eq!(
            pick_season_poster(&seasons, 1).as_deref(),
            Some("image://s1-poster/")
        );
        assert_eq!(
            pick_season_poster(&seasons, 0).as_deref(),
            Some("image://specials-poster/")
        );
        // No poster scraped for this season -> None (caller falls back).
        assert_eq!(pick_season_poster(&seasons, 2), None);
        // Unknown season -> None.
        assert_eq!(pick_season_poster(&seasons, 9), None);
        // Blank poster values don't count.
        assert_eq!(pick_art_poster(&art("   ")), None);
        assert_eq!(pick_art_poster(&HashMap::new()), None);
    }

    #[test]
    fn unwrap_remote_image_cases() {
        assert_eq!(
            unwrap_remote_image("image://https%3a%2f%2fwww.crunchyroll.com%2fx%2fy.png/")
                .as_deref(),
            Some("https://www.crunchyroll.com/x/y.png")
        );
        assert_eq!(
            unwrap_remote_image("image://http%3a%2f%2flan%2fx.jpg/").as_deref(),
            Some("http://lan/x.jpg")
        );
        assert_eq!(unwrap_remote_image("image://video@foo/"), None);
        assert_eq!(unwrap_remote_image("image://musicdb://x/"), None);
        assert_eq!(unwrap_remote_image(""), None);
        assert_eq!(unwrap_remote_image("https://x/y.png"), None);
    }

    #[test]
    fn loopback_detection() {
        assert!(is_loopback_url("http://127.0.0.1:49543/proxy?url=x"));
        assert!(is_loopback_url("http://localhost:8080/image/a"));
        assert!(is_loopback_url("https://localhost/x"));
        assert!(!is_loopback_url("https://kodi.example.com/jsonrpc"));
        assert!(!is_loopback_url("https://image.tmdb.org/t/p/x.jpg"));
    }

    #[test]
    fn info_url_prefers_tmdb() {
        use serde_json::Value;
        use std::collections::HashMap;
        let holder = |tmdb: Option<Value>, imdb: Option<&str>| ArtHolder {
            art: HashMap::new(),
            uniqueid: [
                tmdb.map(|v| ("tmdb".to_string(), v)),
                imdb.map(|s| ("imdb".to_string(), Value::from(s))),
            ]
            .into_iter()
            .flatten()
            .collect(),
            trailer: None,
        };
        let first = |kind: &str, d: &ArtHolder| {
            info_urls(kind, d).into_iter().next().map(|e| e.url)
        };
        // Numeric tmdb tolerated; series vs movie paths differ.
        let d = holder(Some(Value::from(312849)), Some("tt41278600"));
        assert_eq!(
            first("tv", &d).as_deref(),
            Some("https://www.themoviedb.org/tv/312849")
        );
        assert_eq!(
            first("movie", &d).as_deref(),
            Some("https://www.themoviedb.org/movie/312849")
        );
        // No tmdb -> tt-prefixed imdb wins.
        let d = holder(None, Some("tt41278600"));
        assert_eq!(
            first("tv", &d).as_deref(),
            Some("https://www.imdb.com/title/tt41278600/")
        );
        // Garbage everywhere -> no link (never a broken button).
        let d = holder(None, Some(""));
        assert_eq!(first("tv", &d), None);
        let d = holder(None, None);
        assert_eq!(first("movie", &d), None);
    }

    #[test]
    fn info_urls_tmdb_tvdb_wetrakr_order() {
        use serde_json::Value;
        use std::collections::HashMap;
        let holder = ArtHolder {
            art: HashMap::new(),
            uniqueid: [
                ("tmdb".to_string(), Value::from(312849)),
                ("tvdb".to_string(), Value::from(400123)),
                ("imdb".to_string(), Value::from("tt41278600")),
            ]
            .into_iter()
            .collect(),
            trailer: None,
        };
        let urls = info_urls("tv", &holder);
        assert_eq!(urls.len(), 4);
        assert_eq!(urls[0].name, "The Movie DB");
        assert_eq!(urls[0].url, "https://www.themoviedb.org/tv/312849");
        assert_eq!(urls[1].name, "The TV DB");
        assert_eq!(
            urls[1].url,
            "https://thetvdb.com/dereferrer/series/400123"
        );
        assert_eq!(urls[2].name, "WeTrakr");
        assert_eq!(urls[2].url, "https://wetrakr.com/tmdb/shows/312849");
        assert_eq!(urls[3].name, "IMDb");

        let urls = info_urls("movie", &holder);
        assert_eq!(urls[1].url, "https://thetvdb.com/dereferrer/movie/400123");
        assert_eq!(urls[2].url, "https://wetrakr.com/tmdb/movies/312849");

        let holder = ArtHolder {
            art: HashMap::new(),
            uniqueid: [("tmdb".to_string(), Value::from(550))]
                .into_iter()
                .collect(),
            trailer: None,
        };
        let urls = info_urls("movie", &holder);
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0].url, "https://www.themoviedb.org/movie/550");
        assert_eq!(urls[1].url, "https://wetrakr.com/tmdb/movies/550");
    }

    #[test]
    fn plugin_repeat_play_still_displays() {
        // Second-play plugin data (xbmc/xbmc#16245): blank title/season,
        // `type: "unknown"`, but label + file survive and must display.
        let raw = r#"{
            "id": "VideoGetItem", "jsonrpc": "2.0",
            "result": {"item": {
                "dynpath": "plugin://plugin.video.retrospect/?action=playvideo",
                "episode": -1,
                "file": "plugin://plugin.video.retrospect/?action=playvideo",
                "label": "Nyheterna 22.00 [COLOR aqua]\u00ba[/COLOR]",
                "mediapath": "plugin://plugin.video.retrospect/?action=playvideo",
                "season": -1, "showtitle": "", "title": "",
                "type": "unknown"
            }}
        }"#;
        let resp: crate::kodi::JsonRpcResponse<crate::kodi::PlayerGetItemResult> =
            serde_json::from_str(raw).expect("item response must parse");
        let item = resp.result.expect("must have result").item;

        let props_raw = r#"{"speed": 1,
            "time": {"hours": 0, "minutes": 5, "seconds": 0, "milliseconds": 0},
            "totaltime": {"hours": 0, "minutes": 0, "seconds": 0, "milliseconds": 0}}"#;
        let props: PlayerGetPropertiesResult =
            serde_json::from_str(props_raw).expect("props must parse");

        let npi =
            NowPlayingItem::from_kodi(&item, &props, "video").expect("plugin item must display");
        assert_eq!(npi.media_type, MediaType::Unknown);
        assert!(npi.is_plugin);
        assert_eq!(npi.name, "Nyheterna 22.00 º");
        assert_eq!(npi.addon_id.as_deref(), Some("plugin.video.retrospect"));
        assert_eq!(npi.file_host_display(), "retrospect");
        // Unknown-length stream: no timestamps, but NOT paused.
        let session = Session {
            now_playing_item: npi,
            play_state: PlayState::from_props(&props),
            item_id: "x".to_string(),
            source: 0,
        };
        assert!(!session.play_state.is_paused);
        assert!(matches!(session.get_time().unwrap(), PlayTime::None));
    }

    #[test]
    fn crunchyroll_plugin_payload_promotes_to_episode() {
        // Live capture from plugin.video.crunchyroll (smirgol 3.8.0):
        // type unknown, season/episode -1, empty showtitle/title.
        let raw = r#"{
            "label": "The Duke's Son Claims He Won't Love Me, Yet Showers Me with Adoration - S01E07 - An Unexpected Gift",
            "type": "unknown",
            "season": -1,
            "episode": -1,
            "showtitle": "",
            "title": "",
            "tvshowid": -1,
            "file": "plugin://plugin.video.crunchyroll/video/GT00378118/GE00378555JAJP/GE00378555JAJPV",
            "thumbnail": "image://https%3a%2f%2fwww.crunchyroll.com%2fimgsrv%2fdisplay%2fthumbnail%2f1920x1080%2fcatalog%2fcrunchyroll%2f1397f82b5ef846af51683f99a0c3379d.png/",
            "fanart": "image://https%3a%2f%2fwww.crunchyroll.com%2fimgsrv%2fdisplay%2fthumbnail%2f1920x1080%2fcatalog%2fcrunchyroll%2f1397f82b5ef846af51683f99a0c3379d.png/"
        }"#;
        let item: KodiItem = serde_json::from_str(raw).expect("item parses");
        let props = PlayerGetPropertiesResult {
            speed: 1.0,
            time: KodiTime {
                minutes: 3,
                seconds: 54,
                ..Default::default()
            },
            totaltime: KodiTime {
                minutes: 22,
                seconds: 48,
                ..Default::default()
            },
            percentage: Some(17.14),
        };
        let npi = NowPlayingItem::from_kodi(&item, &props, "video").expect("displays");
        assert_eq!(npi.media_type, MediaType::Episode);
        assert!(npi.is_plugin);
        assert_eq!(npi.addon_id.as_deref(), Some("plugin.video.crunchyroll"));
        assert_eq!(
            npi.series_name.as_deref(),
            Some("The Duke's Son Claims He Won't Love Me, Yet Showers Me with Adoration")
        );
        assert_eq!(npi.parent_index_number, Some(1));
        assert_eq!(npi.index_number, Some(7));
        assert_eq!(npi.name, "An Unexpected Gift");
        let buttons = npi.external_urls.as_ref().expect("crunchy buttons");
        assert_eq!(buttons.len(), 2);
        assert_eq!(buttons[0].name, "Watch Episode");
        assert_eq!(
            buttons[0].url,
            "https://www.crunchyroll.com/watch/GE00378555JAJP"
        );
        assert_eq!(buttons[1].name, "View Series Info");
        assert_eq!(
            buttons[1].url,
            "https://www.crunchyroll.com/series/GT00378118"
        );
        // Timestamps still work (known 22:48 runtime via totaltime).
        let session = Session {
            now_playing_item: npi,
            play_state: PlayState::from_props(&props),
            item_id: "x".to_string(),
            source: 0,
        };
        assert!(matches!(session.get_time().unwrap(), PlayTime::Some(_, _)));
    }

    #[test]
    fn crunchyroll_non_episode_label_stays_unknown() {
        let raw = r#"{
            "label": "Just a movie title",
            "type": "unknown",
            "season": -1,
            "episode": -1,
            "showtitle": "",
            "title": "",
            "file": "plugin://plugin.video.crunchyroll/video/GT00378118/GE00378555JAJP/GE00378555JAJPV"
        }"#;
        let item: KodiItem = serde_json::from_str(raw).expect("item parses");
        let props = PlayerGetPropertiesResult {
            speed: 1.0,
            time: KodiTime::default(),
            totaltime: KodiTime::default(),
            percentage: None,
        };
        let npi = NowPlayingItem::from_kodi(&item, &props, "video").expect("displays");
        assert_eq!(npi.media_type, MediaType::Unknown);
    }

    #[test]
    fn slyguy_crunchyroll_payload() {
        // Captured live from a SlyGuy Crunchyroll stream: library-style
        // episode fields, but tvshowid -1 and a localhost proxy file.
        let raw = r#"{
            "label": "The Duke's Son Claims He Won't Love Me, Yet Showers Me with Adoration - S01E07 - An Unexpected Gift",
            "type": "episode",
            "season": 1,
            "episode": 7,
            "showtitle": "The Duke's Son Claims He Won't Love Me, Yet Showers Me with Adoration",
            "tvshowid": -1,
            "file": "http://127.0.0.1:49543/proxy?url=https%3A%2F%2Fwww.crunchyroll.com%2Fplayback%2Fv1%2Fmanifest",
            "thumbnail": "image://https%3a%2f%2fwww.crunchyroll.com%2fimgsrv%2fdisplay%2fthumbnail%2f1920x1080%2fcatalog%2fcrunchyroll%2f1397f82b5ef846af51683f99a0c3379d.png/",
            "title": "The Duke's Son Claims He Won't Love Me, Yet Showers Me with Adoration - S01E07 - An Unexpected Gift"
        }"#;
        let item: KodiItem = serde_json::from_str(raw).expect("item parses");
        let props = PlayerGetPropertiesResult {
            speed: 1.0,
            time: KodiTime {
                seconds: 28,
                ..Default::default()
            },
            totaltime: KodiTime {
                minutes: 22,
                seconds: 48,
                ..Default::default()
            },
            percentage: Some(2.0),
        };
        let npi = NowPlayingItem::from_kodi(&item, &props, "video").expect("displays");
        assert_eq!(npi.media_type, MediaType::Episode);
        assert_eq!(npi.parent_index_number, Some(1));
        assert_eq!(npi.index_number, Some(7));
        assert!(!npi.is_plugin);
        assert_eq!(npi.file_host_display(), "www.crunchyroll.com");
        assert!(is_loopback_url(&npi.file));
        let thumb = npi.thumbnail.as_deref().unwrap();
        assert_eq!(
            unwrap_remote_image(thumb).as_deref(),
            Some("https://www.crunchyroll.com/imgsrv/display/thumbnail/1920x1080/catalog/crunchyroll/1397f82b5ef846af51683f99a0c3379d.png")
        );
    }

    #[test]
    fn crunchyroll_partial_stub_backfills_gaps() {
        // Addon-browser style partial stub: season survived, the rest didn't.
        let raw = r#"{
            "label": "The Duke's Son Claims He Won't Love Me, Yet Showers Me with Adoration - S01E07 - An Unexpected Gift",
            "type": "unknown",
            "season": 1,
            "episode": -1,
            "showtitle": "",
            "title": "",
            "file": "plugin://plugin.video.crunchyroll/video/GT00378118/GE00378555JAJP/GE00378555JAJPV"
        }"#;
        let item: KodiItem = serde_json::from_str(raw).expect("item parses");
        let props = PlayerGetPropertiesResult {
            speed: 1.0,
            time: KodiTime::default(),
            totaltime: KodiTime::default(),
            percentage: None,
        };
        let npi = NowPlayingItem::from_kodi(&item, &props, "video").expect("displays");
        assert_eq!(npi.media_type, MediaType::Episode);
        assert_eq!(npi.parent_index_number, Some(1));
        assert_eq!(npi.index_number, Some(7));
        assert_eq!(
            npi.series_name.as_deref(),
            Some("The Duke's Son Claims He Won't Love Me, Yet Showers Me with Adoration")
        );
        assert_eq!(npi.name, "An Unexpected Gift");
    }

    #[test]
    fn crunchyroll_rich_metadata_left_alone() {
        let raw = r#"{
            "label": "The Duke's Son Claims He Won't Love Me, Yet Showers Me with Adoration - S01E07 - An Unexpected Gift",
            "type": "episode",
            "season": 1,
            "episode": 7,
            "showtitle": "The Duke's Son Claims He Won't Love Me, Yet Showers Me with Adoration",
            "title": "An Unexpected Gift",
            "file": "plugin://plugin.video.crunchyroll/video/GT00378118/GE00378555JAJP/GE00378555JAJPV"
        }"#;
        let item: KodiItem = serde_json::from_str(raw).expect("item parses");
        let props = PlayerGetPropertiesResult {
            speed: 1.0,
            time: KodiTime::default(),
            totaltime: KodiTime::default(),
            percentage: None,
        };
        let npi = NowPlayingItem::from_kodi(&item, &props, "video").expect("displays");
        assert_eq!(npi.media_type, MediaType::Episode);
        assert_eq!(npi.name, "An Unexpected Gift");
        assert_eq!(npi.parent_index_number, Some(1));
        assert_eq!(npi.index_number, Some(7));
        let buttons = npi.external_urls.as_ref().expect("crunchy buttons");
        assert_eq!(buttons.len(), 2);
        assert_eq!(
            buttons[0].url,
            "https://www.crunchyroll.com/watch/GE00378555JAJP"
        );
        assert_eq!(
            buttons[1].url,
            "https://www.crunchyroll.com/series/GT00378118"
        );
    }
}
