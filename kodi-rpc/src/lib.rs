use crunchyroll::CrunchySeriesEntry;
use discord_rich_presence::activity::{
    ActivityType, Button as ActButton, StatusDisplayType as DiscordIpcStatusDisplayType,
};
use discord_rich_presence::{
    activity::{Activity, Assets, Timestamps},
    DiscordIpc, DiscordIpcClient,
};
pub use error::KodiError;
pub use external::image_utils::ImageProcessingOptions;
use kodi::{
    get_item_properties, pick_art_poster, pick_season_poster, ActivePlayer, JsonRpcRequest,
    JsonRpcResponse, MovieDetailsResult, NowPlayingItem, PlayState, PlayTime, PlayerGetItemResult,
    PlayerGetPropertiesResult, SeasonsResult, TVShowDetailsResult,
};
pub use kodi::{Button, MediaType};
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use url::Url;

pub(crate) mod crunchyroll;
mod error;
mod external;
mod kodi;
#[cfg(test)]
mod tests;

pub(crate) type KodiResult<T> = Result<T, Box<dyn std::error::Error>>;

pub const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

/// Full version string: `CARGO_PKG_VERSION` plus the commit hash for
/// snapshot (untagged) builds, e.g. `0.1.0+a1b2c3d`. Release builds
/// (KODI_RPC_RELEASE=1, i.e. tags) report the clean version.
pub fn version_string() -> String {
    match option_env!("KODI_RPC_GIT_SHA") {
        Some(sha) if !sha.is_empty() => {
            format!("{}+{}", VERSION.unwrap_or("0.0.0"), sha)
        }
        _ => VERSION.unwrap_or("0.0.0").to_string(),
    }
}

/// Discord activity field limits (discord.com/developers/docs): text fields
/// cap at 128 chars (min 3, zero-width-padded below that), max 2 buttons
/// with 32-char labels.
const MAX_TEXT_LEN: usize = 128;
const MAX_BUTTON_LABEL_LEN: usize = 32;
const MAX_BUTTON_URL_LEN: usize = 512;

/// One Kodi server: URL + HTTP auth + a display name for logs.
#[derive(Debug, Clone, Default)]
pub struct InstanceCfg {
    pub url: String,
    pub username: String,
    pub password: String,
    pub self_signed_cert: bool,
    pub name: String,
}

struct KodiInstance {
    name: String,
    base: Url,
    http: reqwest::blocking::Client,
    direct_art: bool,
}

/// One poll result from a single instance.
struct FetchedSession {
    npi: NowPlayingItem,
    play_state: PlayState,
    item_id: String,
    tvshowid: Option<i32>,
    movieid: Option<i64>,
}

/// Cached library artwork + info links per show/season/movie.
#[derive(Clone, Default)]
struct CachedArt {
    poster: Option<String>,
    info_urls: Vec<kodi::ExternalUrl>,
    trailer_url: Option<String>,
    tmdb_id: Option<String>,
    tvdb_id: Option<String>,
    imdb_id: Option<String>,
}

/// Crunchyroll series cache (see [`crunchyroll::CrunchySeriesEntry`]).
/// Client used to interact with Kodi and Discord
pub struct Client {
    discord_ipc_client: DiscordIpcClient,
    instances: Vec<KodiInstance>,
    session: Option<Session>,
    /// Which instance owns the presence (first-plays-wins).
    session_owner: Option<usize>,
    music_display_options: DisplayOptions,
    movies_display_options: DisplayOptions,
    episodes_display_options: DisplayOptions,
    unknown_display_options: DisplayOptions,
    blacklist: Blacklist,
    show_images: bool,
    imgur_options: ImgurOptions,
    litterbox_options: LitterboxOptions,
    process_images: bool,
    image_processing_options: external::image_utils::ImageProcessingOptions,
    large_image_text: String,
    /// Poster lookups per show/season/movie; avoids an extra RPC per poll.
    poster_cache: HashMap<String, CachedArt>,
    /// Crunchyroll series lookups, same zero-per-poll goal.
    crunchy_cache: HashMap<String, CrunchySeriesEntry>,
}

use kodi::Session;

impl Client {
    /// Calls the `ClientBuilder::new()` function
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Connects to the discord socket
    pub fn connect(&mut self) -> KodiResult<()> {
        self.discord_ipc_client.connect()?;
        Ok(())
    }

    /// Reconnects to the discord socket
    pub fn reconnect(&mut self) -> KodiResult<()> {
        self.discord_ipc_client.reconnect()?;
        Ok(())
    }

    /// Clears current activity on discord if anything is being displayed
    pub fn clear_activity(&mut self) -> KodiResult<()> {
        self.discord_ipc_client.clear_activity()?;
        Ok(())
    }

    /// Whether the owned session is paused. False when idle.
    pub fn is_paused(&self) -> bool {
        self.session
            .as_ref()
            .map(|s| s.play_state.is_paused)
            .unwrap_or(false)
    }

    /// Display names of all watched instances, in config order.
    pub fn instance_names(&self) -> Vec<String> {
        self.instances.iter().map(|i| i.name.clone()).collect()
    }

    /// Display name of the instance owning the current session, if any.
    pub fn session_source_name(&self) -> Option<String> {
        self.session
            .as_ref()
            .and_then(|s| self.instances.get(s.source))
            .map(|i| i.name.clone())
    }

    /// Polls Kodi and pushes the presence to Discord. Plugin
    /// (`type: "unknown"`) streams display like any other content.
    pub fn set_activity(&mut self) -> KodiResult<String> {
        self.get_session()?;

        if let Some(session) = &self.session {
            if session.now_playing_item.media_type == MediaType::None {
                return Err(Box::new(KodiError::UnrecognizedMediaType));
            }

            if self.check_blacklist()? {
                return Err(Box::new(KodiError::ContentBlacklist));
            }

            let mut activity = Activity::new();

            let mut image_url = Url::from_str("https://i.imgur.com/koCh16G.png")?;

            if session.now_playing_item.media_type == MediaType::LiveTv {
                image_url = Url::from_str("https://i.imgur.com/XxdHOqm.png")?;
            } else if self.imgur_options.enabled && self.show_images {
                if let Ok(imgur_url) = external::imgur::get_image(self) {
                    image_url = imgur_url;
                } else {
                    debug!("imgur::get_image() didnt return an image, using default..")
                }
            } else if self.litterbox_options.enabled && self.show_images {
                if let Ok(litterbox_url) = external::litterbox::get_image(self) {
                    image_url = litterbox_url;
                } else {
                    debug!("litterbox::get_image() didn't return an image, using default..")
                }
            } else if self.show_images {
                if let Ok(iu) = self.get_image() {
                    image_url = iu;
                } else {
                    debug!("self.get_image() didnt return an image, using default..")
                }
            }

            let mut assets = Assets::new().large_image(image_url.as_str());

            if !self.large_image_text.is_empty() {
                assets = assets.large_text(&self.large_image_text);
            }

            let mut timestamps = Timestamps::new();

            // Per-type behavior: paused handling and buttons come from the
            // playing section's own options.
            let show_paused = self
                .display_options(session.now_playing_item.media_type)
                .show_paused;
            match session.get_time()? {
                PlayTime::Some(start, end) => timestamps = timestamps.start(start).end(end),
                PlayTime::None => (),
                PlayTime::Paused if show_paused => {
                    assets = assets
                        .small_image("https://i.imgur.com/wlHSvYy.png")
                        .small_text("Paused");
                }
                PlayTime::Paused => return Ok(String::new()),
            }

            let buttons: Vec<Button>;
            let button_names: Vec<String>;

            if let Some(b) = self.get_buttons() {
                buttons = b;
                button_names = buttons
                    .iter()
                    .map(|b| b.name.chars().take(MAX_BUTTON_LABEL_LEN).collect())
                    .collect();
                activity = activity.buttons(
                    buttons
                        .iter()
                        .zip(button_names.iter())
                        .map(|(b, name)| ActButton::new(name, &b.url))
                        .collect(),
                );
            }

            let mut state = self.get_state();

            if state.len() > MAX_TEXT_LEN {
                state = state.chars().take(MAX_TEXT_LEN).collect();
            } else if state.len() < 3 {
                state += "‎‎‎";
            }

            let mut details = self.get_details();

            if details.len() > MAX_TEXT_LEN {
                details = details.chars().take(MAX_TEXT_LEN).collect();
            } else if details.len() < 3 {
                details += "‎‎‎";
            }

            let mut image_text = self.get_image_text();

            if image_text.is_empty() {
                image_text = format!("Kodi-RPC v{}", VERSION.unwrap_or("UNKNOWN"));
            }

            if image_text.len() > MAX_TEXT_LEN {
                image_text = image_text.chars().take(MAX_TEXT_LEN).collect();
            } else if image_text.len() < 3 {
                image_text += "‎‎‎";
            }

            assets = assets.large_text(image_text.as_str());

            match session.now_playing_item.media_type {
                MediaType::Book => (),
                MediaType::Music | MediaType::AudioBook => {
                    activity = activity.activity_type(ActivityType::Listening)
                }
                _ => activity = activity.activity_type(ActivityType::Watching),
            }

            let status_display_type = self.get_status_display_type();

            activity = activity
                .timestamps(timestamps)
                .assets(assets)
                .details(&details)
                .state(&state)
                .status_display_type(status_display_type.into());

            self.discord_ipc_client.set_activity(activity)?;

            return Ok(format!("{} | {}", details, state));
        }
        Ok(String::new())
    }

    fn jsonrpc<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        inst: &KodiInstance,
        method: &'static str,
        params: T,
    ) -> KodiResult<R> {
        let endpoint = inst.base.join("jsonrpc")?;
        let req = JsonRpcRequest::new(method, params, "kodi-rpc");
        debug!("Kodi JSON-RPC -> {} {:?} [{}]", method, endpoint, inst.name);
        let resp: JsonRpcResponse<R> = inst.http.post(endpoint).json(&req).send()?.json()?;
        resp.result
            .ok_or_else(|| "kodi json-rpc returned no result".into())
    }

    /// First-plays-wins arbitration across instances: the owner keeps the
    /// presence while it plays (even if others start too); once it stops,
    /// the first playing instance in config order takes over.
    fn elect_owner(current: Option<usize>, playing: &[bool]) -> Option<usize> {
        if let Some(o) = current {
            if playing.get(o).copied().unwrap_or(false) {
                return Some(o);
            }
        }
        playing.iter().position(|p| *p)
    }

    fn get_session(&mut self) -> KodiResult<()> {
        let mut fetched = Vec::with_capacity(self.instances.len());
        for idx in 0..self.instances.len() {
            fetched.push(self.fetch_instance(idx));
        }

        let playing: Vec<bool> = fetched.iter().map(|f| f.is_some()).collect();
        match Self::elect_owner(self.session_owner, &playing) {
            Some(idx) => {
                if self.session_owner != Some(idx) {
                    debug!(
                        "Presence ownership -> instance {} ({})",
                        idx, self.instances[idx].name
                    );
                }
                self.session_owner = Some(idx);
                let fs = fetched[idx].take().expect("elected instance is playing");
                let mut npi = fs.npi;
                self.populate_poster(
                    idx,
                    &mut npi,
                    fs.tvshowid,
                    fs.movieid,
                    self.episodes_display_options.poster_source,
                );
                self.session = Some(Session {
                    now_playing_item: npi,
                    play_state: fs.play_state,
                    item_id: fs.item_id,
                    source: idx,
                });
            }
            None => {
                if self.session_owner.is_some() {
                    debug!("All instances idle; clearing presence ownership");
                }
                self.session_owner = None;
                self.session = None;
            }
        }
        Ok(())
    }

    /// Poll one instance. Dead/unreachable instances yield None so the rest
    /// keep working.
    fn fetch_instance(&self, idx: usize) -> Option<FetchedSession> {
        let inst = &self.instances[idx];

        #[derive(Serialize)]
        struct EmptyParams {}

        let players: Vec<ActivePlayer> =
            match self.jsonrpc(inst, "Player.GetActivePlayers", EmptyParams {}) {
                Ok(p) => p,
                Err(e) => {
                    debug!("[{}] Player.GetActivePlayers failed: {}", inst.name, e);
                    return None;
                }
            };

        debug!("[{}] Found {} active players", inst.name, players.len());

        if players.is_empty() {
            return None;
        }

        // Prefer video over audio over picture (Kodi can run music + slideshow
        // at the same time).
        let mut sorted = players.clone();
        sorted.sort_by_key(|p| match p.player_type.as_str() {
            "video" => 0,
            "audio" => 1,
            _ => 2,
        });

        for active in sorted {
            #[derive(Serialize)]
            struct ItemParams {
                playerid: i32,
                properties: Vec<&'static str>,
            }
            #[derive(Serialize)]
            struct PropsParams {
                playerid: i32,
                properties: Vec<&'static str>,
            }

            let item_res: Result<PlayerGetItemResult, _> = self.jsonrpc(
                inst,
                "Player.GetItem",
                ItemParams {
                    playerid: active.playerid,
                    properties: get_item_properties(),
                },
            );
            let props_res: Result<PlayerGetPropertiesResult, _> = self.jsonrpc(
                inst,
                "Player.GetProperties",
                PropsParams {
                    playerid: active.playerid,
                    properties: vec!["speed", "time", "totaltime", "percentage"],
                },
            );

            let (item_res, props_res) = match (item_res, props_res) {
                (Ok(i), Ok(p)) => (i, p),
                (Err(e), _) | (_, Err(e)) => {
                    debug!(
                        "[{}] Skipping player {} ({}): {}",
                        inst.name, active.playerid, active.player_type, e
                    );
                    continue;
                }
            };

            debug!(
                "[{}] Player {} ({}): type={} label={} file={}",
                inst.name,
                active.playerid,
                active.player_type,
                item_res.item.item_type,
                item_res.item.label,
                item_res.item.file.as_deref().unwrap_or("")
            );

            if let Some(npi) =
                NowPlayingItem::from_kodi(&item_res.item, &props_res, &active.player_type)
            {
                // Image-cache key: the file path works for library and plugin items.
                let item_id = npi.id.clone();
                return Some(FetchedSession {
                    npi,
                    play_state: PlayState::from_props(&props_res),
                    item_id,
                    tvshowid: item_res.item.tvshowid.filter(|v| *v >= 0),
                    movieid: item_res.item.id.filter(|v| *v >= 0),
                });
            } else {
                debug!(
                    "[{}] Player {} has nothing displayable, trying next player",
                    inst.name, active.playerid
                );
            }
        }

        None
    }

    /// Fill `poster` from the library (series/season/movie). Never fails the
    /// session: no IDs / RPC errors just keep the still/fanart fallbacks.
    fn populate_poster(
        &mut self,
        inst_idx: usize,
        npi: &mut NowPlayingItem,
        tvshowid: Option<i32>,
        movieid: Option<i64>,
        source: PosterSource,
    ) {
        match npi.media_type {
            MediaType::Episode => {
                // Crunchyroll has no library tvshowid; use its directory listing.
                if tvshowid.is_none() && crunchyroll::is_crunchyroll_addon(npi.addon_id.as_deref())
                {
                    self.populate_crunchyroll(npi, inst_idx, source);
                    return;
                }
                if source == PosterSource::Episode {
                    return;
                }
                let (Some(tvshowid), Some(season)) = (tvshowid, npi.parent_index_number) else {
                    return;
                };
                let cache_key = format!("{}:tv:{}:s{}:{:?}", inst_idx, tvshowid, season, source);
                if let Some(cached) = self.poster_cache.get(&cache_key) {
                    Self::apply_art(npi, cached);
                    return;
                }
                let mut art = CachedArt::default();
                if source == PosterSource::Series {
                    if let Some(details) = self.fetch_tvshow_details(inst_idx, tvshowid) {
                        art.poster = pick_art_poster(&details.art);
                        art.info_urls = kodi::info_urls("tv", &details);
                        art.tmdb_id = kodi::tmdb_id(&details);
                        art.tvdb_id = kodi::tvdb_id(&details);
                        art.imdb_id = kodi::imdb_id(&details);
                    }
                } else if let Some(details) = self.fetch_tvshow_details(inst_idx, tvshowid) {
                    art.poster = self
                        .fetch_season_poster(inst_idx, tvshowid, season)
                        .or_else(|| pick_art_poster(&details.art));
                    art.info_urls = kodi::info_urls("tv", &details);
                    art.tmdb_id = kodi::tmdb_id(&details);
                    art.tvdb_id = kodi::tvdb_id(&details);
                    art.imdb_id = kodi::imdb_id(&details);
                } else {
                    art.poster = self.fetch_season_poster(inst_idx, tvshowid, season);
                }
                debug!(
                    "Art for tvshow {} season {} ({:?}): poster={}, info={:?}",
                    tvshowid,
                    season,
                    source,
                    art.poster.as_deref().unwrap_or("<none>"),
                    art.info_urls
                        .iter()
                        .map(|e| e.url.as_str())
                        .collect::<Vec<_>>(),
                );
                self.poster_cache.insert(cache_key, art.clone());
                Self::apply_art(npi, &art);
            }
            MediaType::Movie => {
                let Some(movieid) = movieid else {
                    return;
                };
                let cache_key = format!("{}:movie:{}", inst_idx, movieid);
                if let Some(cached) = self.poster_cache.get(&cache_key) {
                    Self::apply_art(npi, cached);
                    return;
                }
                let mut art = CachedArt::default();
                if let Some(details) = self.fetch_movie_details(inst_idx, movieid) {
                    art.poster = pick_art_poster(&details.art);
                    art.info_urls = kodi::info_urls("movie", &details);
                    art.tmdb_id = kodi::tmdb_id(&details);
                    art.tvdb_id = kodi::tvdb_id(&details);
                    art.imdb_id = kodi::imdb_id(&details);
                    art.trailer_url = details
                        .trailer
                        .clone()
                        .filter(|t| t.starts_with("http://") || t.starts_with("https://"))
                        .filter(|t| !kodi::is_loopback_url(t));
                }
                debug!(
                    "Art for movie {}: poster={}, info={:?}, trailer={}",
                    movieid,
                    art.poster.as_deref().unwrap_or("<none>"),
                    art.info_urls
                        .iter()
                        .map(|e| e.url.as_str())
                        .collect::<Vec<_>>(),
                    art.trailer_url.as_deref().unwrap_or("<none>"),
                );
                self.poster_cache.insert(cache_key, art.clone());
                Self::apply_art(npi, &art);
            }
            _ => {}
        }
    }

    /// Crunchyroll `poster` (+ missing episode `plot`) from directory
    /// listings. Hits cost 0 RPCs, misses at most 2; entries live for
    /// [`crunchyroll::CACHE_TTL`], failures negative-cached alike.
    fn populate_crunchyroll(
        &mut self,
        npi: &mut NowPlayingItem,
        inst_idx: usize,
        source: PosterSource,
    ) {
        let Some(series_id) = crunchyroll::series_id_from_file(&npi.file) else {
            return;
        };
        let cache_key = format!("{}:crunchy:{}", inst_idx, series_id);
        if let Some(entry) = self.crunchy_cache.get(&cache_key) {
            if entry.is_fresh() {
                Self::apply_crunchy_entry(npi, entry, source);
                return;
            }
            debug!(
                "Crunchyroll cache expired for series {} (ttl {:?}), refetching",
                series_id,
                crunchyroll::CACHE_TTL
            );
        }
        let mut entry = CrunchySeriesEntry::default();
        if let Some(seasons) =
            self.fetch_crunchy_directory(inst_idx, &crunchyroll::series_directory(&series_id))
        {
            for s in &seasons {
                if entry.poster.is_none() {
                    if let Some(poster) = pick_art_poster(&s.art) {
                        entry.poster = Some(poster);
                    }
                }
                if entry.fanart.is_none() {
                    if let Some(f) = s.fanart.clone().filter(|f| !f.is_empty()) {
                        entry.fanart = Some(f);
                    }
                }
                if let Some(num) = s.season.filter(|v| *v >= 0) {
                    if let Some(season_id) = s.file.rsplit('/').next().filter(|v| !v.is_empty()) {
                        entry.seasons.entry(num).or_insert(season_id.to_string());
                    }
                }
            }
            if let Some(season_id) = npi
                .parent_index_number
                .and_then(|n| entry.season_id(n).map(str::to_string))
            {
                if let Some(episodes) = self.fetch_crunchy_directory(
                    inst_idx,
                    &crunchyroll::season_directory(&series_id, &season_id),
                ) {
                    for e in &episodes {
                        if entry.poster.is_none() {
                            if let Some(poster) = pick_art_poster(&e.art) {
                                entry.poster = Some(poster);
                            }
                        }
                        if let Some(plot) = e.plot.clone().filter(|p| !p.trim().is_empty()) {
                            if !e.file.is_empty() {
                                entry.episode_plots.entry(e.file.clone()).or_insert(plot);
                            }
                        }
                    }
                }
            }
        }
        debug!(
            "Crunchyroll art for series {}: poster={}, seasons={}, plots={}",
            series_id,
            entry.poster.as_deref().unwrap_or("<none>"),
            entry.seasons.len(),
            entry.episode_plots.len(),
        );
        // Cache even empty misses so failures don't cost RPCs every poll.
        self.crunchy_cache.insert(cache_key.clone(), entry.clone());
        if let Some(entry) = self.crunchy_cache.get(&cache_key) {
            Self::apply_crunchy_entry(npi, entry, source);
        }
    }

    fn apply_crunchy_entry(
        npi: &mut NowPlayingItem,
        entry: &CrunchySeriesEntry,
        source: PosterSource,
    ) {
        // Episode-still mode keeps Player.GetItem art.
        if source != PosterSource::Episode && npi.poster.is_none() {
            if let Some(poster) = entry.poster.clone() {
                npi.poster = Some(poster);
            }
        }
        if npi.plot.is_none() {
            if let Some(plot) = entry.plot_for(&npi.file).map(str::to_string) {
                npi.plot = Some(plot);
            }
        }
    }

    fn fetch_crunchy_directory(
        &self,
        inst_idx: usize,
        directory: &str,
    ) -> Option<Vec<crunchyroll::DirectoryFile>> {
        #[derive(Serialize)]
        struct Params<'a> {
            directory: &'a str,
            properties: Vec<&'static str>,
        }
        let res: crunchyroll::FilesDirectoryResult = self
            .jsonrpc(
                &self.instances[inst_idx],
                "Files.GetDirectory",
                Params {
                    directory,
                    properties: crunchyroll::DIRECTORY_PROPERTIES.to_vec(),
                },
            )
            .ok()?;
        if crunchyroll::is_login_failure(&res.files) {
            return None;
        }
        Some(res.files)
    }

    fn apply_art(npi: &mut NowPlayingItem, art: &CachedArt) {
        npi.poster = art.poster.clone();
        npi.tmdb_id = art.tmdb_id.clone();
        npi.tvdb_id = art.tvdb_id.clone();
        npi.imdb_id = art.imdb_id.clone();
        npi.trailer_url = art.trailer_url.clone();
        let mut urls = Vec::new();
        if let Some(trailer) = art.trailer_url.clone() {
            urls.push(kodi::ExternalUrl {
                name: "Trailer".to_string(),
                url: trailer,
            });
        }
        for info in &art.info_urls {
            urls.push(kodi::ExternalUrl {
                name: info.name.clone(),
                url: info.url.clone(),
            });
        }
        npi.external_urls = if urls.is_empty() { None } else { Some(urls) };
    }

    fn fetch_season_poster(&self, inst_idx: usize, tvshowid: i32, season: i32) -> Option<String> {
        #[derive(Serialize)]
        struct Params {
            tvshowid: i32,
            properties: Vec<&'static str>,
        }
        let res: SeasonsResult = self
            .jsonrpc(
                &self.instances[inst_idx],
                "VideoLibrary.GetSeasons",
                Params {
                    tvshowid,
                    properties: vec!["season", "art"],
                },
            )
            .ok()?;
        pick_season_poster(&res.seasons, season)
    }

    fn fetch_tvshow_details(&self, inst_idx: usize, tvshowid: i32) -> Option<kodi::ArtHolder> {
        #[derive(Serialize)]
        struct Params {
            tvshowid: i32,
            properties: Vec<&'static str>,
        }
        let res: TVShowDetailsResult = self
            .jsonrpc(
                &self.instances[inst_idx],
                "VideoLibrary.GetTVShowDetails",
                Params {
                    tvshowid,
                    properties: vec!["art", "uniqueid"],
                },
            )
            .ok()?;
        Some(res.tvshowdetails)
    }

    fn fetch_movie_details(&self, inst_idx: usize, movieid: i64) -> Option<kodi::ArtHolder> {
        #[derive(Serialize)]
        struct Params {
            movieid: i64,
            properties: Vec<&'static str>,
        }
        let res: MovieDetailsResult = self
            .jsonrpc(
                &self.instances[inst_idx],
                "VideoLibrary.GetMovieDetails",
                Params {
                    movieid,
                    properties: vec!["art", "uniqueid", "trailer"],
                },
            )
            .ok()?;
        Some(res.moviedetails)
    }

    /// Drop buttons Discord would reject: a URL over 512 chars breaks the
    /// whole presence, like an unfetchable image does.
    fn usable_buttons(buttons: Vec<Button>) -> Vec<Button> {
        buttons
            .into_iter()
            .filter(|b| b.url.chars().count() <= MAX_BUTTON_URL_LEN)
            .collect()
    }

    fn display_options(&self, media_type: MediaType) -> &DisplayOptions {
        match media_type {
            MediaType::Music => &self.music_display_options,
            MediaType::Movie => &self.movies_display_options,
            MediaType::Episode => &self.episodes_display_options,
            _ => &self.unknown_display_options,
        }
    }

    fn button_template_values(&self) -> Vec<(String, String)> {
        let session = self.session.as_ref().unwrap();
        let item = &session.now_playing_item;
        let mut values: Vec<(String, String)> = Vec::new();
        match item.media_type {
            MediaType::Music => {
                values.push(("{track}".to_string(), item.name.clone()));
                values.push((
                    "{album}".to_string(),
                    item.album.clone().unwrap_or_default(),
                ));
                values.push(("{artists}".to_string(), session.format_artists()));
            }
            MediaType::Movie => {
                values.push(("{title}".to_string(), item.name.clone()));
                values.push((
                    "{original-title}".to_string(),
                    item.original_title.clone().unwrap_or_default(),
                ));
                values.push((
                    "{critic-score}".to_string(),
                    item.critic_rating
                        .map(|s| format!("🍅 {}/100", s))
                        .unwrap_or_default(),
                ));
                values.push((
                    "{community-score}".to_string(),
                    item.community_rating
                        .map(|s| format!("⭐ {:.1}/10", s))
                        .unwrap_or_default(),
                ));
            }
            MediaType::Episode => {
                values.push((
                    "{show-title}".to_string(),
                    item.series_name.clone().unwrap_or_default(),
                ));
                values.push(("{title}".to_string(), item.name.clone()));
                values.push((
                    "{original-title}".to_string(),
                    item.original_title.clone().unwrap_or_default(),
                ));
                let episode = item.index_number.unwrap_or(0);
                let season = item.parent_index_number.unwrap_or(0);
                values.push(("{episode}".to_string(), episode.to_string()));
                values.push(("{episode-padded}".to_string(), format!("{:02}", episode)));
                values.push(("{season}".to_string(), season.to_string()));
                values.push(("{season-padded}".to_string(), format!("{:02}", season)));
                values.push((
                    "{studio}".to_string(),
                    item.series_studio.clone().unwrap_or_default(),
                ));
            }
            _ => {
                values.push(("{title}".to_string(), item.name.clone()));
                values.push(("{label}".to_string(), item.label.clone()));
                values.push((
                    "{addon}".to_string(),
                    item.addon_id
                        .as_ref()
                        .map(|a| a.rsplit('.').next().unwrap_or(a).to_string())
                        .unwrap_or_default(),
                ));
                values.push((
                    "{addon-full}".to_string(),
                    item.addon_id.clone().unwrap_or_default(),
                ));
                values.push(("{file-host}".to_string(), item.file_host_display()));
                values.push((
                    "{studio}".to_string(),
                    item.series_studio.clone().unwrap_or_default(),
                ));
                values.push(("{plot}".to_string(), item.plot.clone().unwrap_or_default()));
            }
        }
        values.push((
            "{genres}".to_string(),
            item.genres
                .as_ref()
                .unwrap_or(&vec!["".to_string()])
                .join(", "),
        ));
        values.push((
            "{year}".to_string(),
            item.production_year
                .map(|y| y.to_string())
                .unwrap_or_default(),
        ));
        values.push(("{tmdb}".to_string(), item.tmdb_id.clone().unwrap_or_default()));
        values.push(("{tvdb}".to_string(), item.tvdb_id.clone().unwrap_or_default()));
        values.push(("{imdb}".to_string(), item.imdb_id.clone().unwrap_or_default()));
        values.push((
            "{trailer}".to_string(),
            item.trailer_url.clone().unwrap_or_default(),
        ));
        values.push((
            "{version}".to_string(),
            VERSION.unwrap_or("UNKNOWN").to_string(),
        ));
        values.push((
            "{sep}".to_string(),
            self.display_options(item.media_type).separator.clone(),
        ));
        values
    }

    fn is_usable_button_url(url: &str) -> bool {
        if url.chars().count() > MAX_BUTTON_URL_LEN {
            return false;
        }
        let parsed = match Url::parse(url) {
            Ok(u) => u,
            Err(_) => return false,
        };
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return false;
        }
        !kodi::is_loopback_url(url)
    }

    fn render_custom_button(&self, button: &Button) -> Option<Button> {
        let mut values = self.button_template_values();
        values.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        let mut name = button.name.clone();
        let mut url = button.url.clone();
        let mut missing = false;
        for (key, raw) in &values {
            if name.contains(key) {
                name = name.replace(key, raw);
            }
            if url.contains(key) {
                if raw.is_empty() {
                    missing = true;
                }
                if key == "{trailer}" {
                    url = url.replace(key, raw);
                } else {
                    url = url.replace(key, &urlencoding::encode(raw));
                }
            }
        }
        if missing {
            return None;
        }
        if name.trim().is_empty() {
            return None;
        }
        if !Self::is_usable_button_url(&url) {
            return None;
        }
        Some(Button::new(name, url))
    }

    fn get_buttons(&self) -> Option<Vec<Button>> {
        let session = self.session.as_ref()?;
        let conf_buttons = &self
            .display_options(session.now_playing_item.media_type)
            .buttons;

        let mut activity_buttons: Vec<Button> = Vec::new();

        if let (Some(ext_urls), Some(buttons)) = (
            &session.now_playing_item.external_urls,
            conf_buttons.as_ref(),
        ) {
            let ext_urls: Vec<&kodi::ExternalUrl> = ext_urls
                .iter()
                .filter(|eu| !kodi::is_loopback_url(&eu.url))
                .collect();
            let mut i = 0;
            for button in buttons {
                if activity_buttons.len() == 2 {
                    break;
                }

                if button.is_dynamic() {
                    if ext_urls.len() > i {
                        activity_buttons.push(Button::new(
                            ext_urls[i].name.clone(),
                            ext_urls[i].url.clone(),
                        ));
                        i += 1;
                    }
                } else if let Some(rendered) = self.render_custom_button(button) {
                    activity_buttons.push(rendered)
                }
            }
            return Some(Self::usable_buttons(activity_buttons));
        } else if let Some(buttons) = conf_buttons.as_ref() {
            for button in buttons {
                if activity_buttons.len() == 2 {
                    break;
                }

                if !button.is_dynamic() {
                    if let Some(rendered) = self.render_custom_button(button) {
                        activity_buttons.push(rendered)
                    }
                }
            }
            return Some(Self::usable_buttons(activity_buttons));
        } else if let Some(ext_urls) = &session.now_playing_item.external_urls {
            let ext_urls: Vec<&kodi::ExternalUrl> = ext_urls
                .iter()
                .filter(|eu| !kodi::is_loopback_url(&eu.url))
                .collect();
            for ext_url in ext_urls {
                if activity_buttons.len() == 2 {
                    break;
                }

                activity_buttons.push(Button::new(ext_url.name.clone(), ext_url.url.clone()))
            }
            return Some(Self::usable_buttons(activity_buttons));
        }
        None
    }

    /// HTTP client of the instance that owns the current session (its
    /// credentials are the only ones that may download its artwork).
    fn owner_http(&self) -> KodiResult<&reqwest::blocking::Client> {
        let idx = self.session.as_ref().map(|s| s.source).unwrap_or(0);
        self.instances
            .get(idx)
            .map(|i| &i.http)
            .ok_or_else(|| "no kodi instance".into())
    }

    /// Download the current artwork bytes through the owning instance.
    fn download_artwork(&self) -> KodiResult<Vec<u8>> {
        Ok(self
            .owner_http()?
            .get(self.artwork_download_url()?)
            .send()?
            .bytes()?
            .to_vec())
    }

    /// Artwork URL Discord can fetch: plugin/CDN hotlinks always, Kodi
    /// `image://` only when the owning server is public without auth
    /// (localhost/LAN URLs would break the whole presence render, so those
    /// fall back to the default icon — use imgur/litterbox uploads instead).
    pub fn get_image(&self) -> KodiResult<Url> {
        let session = self.session.as_ref().unwrap();
        let item = &session.now_playing_item;
        let inst = &self.instances[session.source];

        for candidate in [
            item.poster.as_ref(),
            item.thumbnail.as_ref(),
            item.fanart.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(u) = Self::resolve_candidate(candidate, &inst.base, inst.direct_art, true) {
                return Ok(u);
            }
        }

        Err(Box::new(KodiError::NoImage))
    }

    /// Resolve one art candidate. `direct` allows translating Kodi `image://`
    /// to local `/image/` URLs (upload path only); `https_only` additionally
    /// refuses plain-http remote art (display path: Discord's proxy can't be
    /// trusted with it, and a bad `large_image` kills the whole presence).
    fn resolve_candidate(t: &str, base: &Url, direct: bool, https_only: bool) -> Option<Url> {
        let t = t.trim();
        if t.is_empty() {
            return None;
        }
        if let Some(inner) = kodi::unwrap_remote_image(t) {
            if !https_only || inner.starts_with("https://") {
                if let Ok(u) = Url::parse(&inner) {
                    return Some(u);
                }
            }
            return None;
        }
        if t.starts_with("https://") || (!https_only && t.starts_with("http://")) {
            if let Ok(u) = Url::parse(t) {
                return Some(u);
            }
            return None;
        }
        if t.starts_with("http://") {
            return None;
        }
        if t.starts_with("image://") && direct {
            let encoded = urlencoding::encode(t);
            if let Ok(u) = base.join(&format!("image/{}", encoded)) {
                return Some(u);
            }
        }
        None
    }

    /// Raw artwork URL for byte downloads (imgur/litterbox). Always resolves
    /// `image://` via our authed client; never hand to Discord directly.
    fn artwork_download_url(&self) -> KodiResult<Url> {
        let session = self.session.as_ref().unwrap();
        let item = &session.now_playing_item;
        let inst = &self.instances[session.source];

        for candidate in [
            item.poster.as_ref(),
            item.thumbnail.as_ref(),
            item.fanart.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(u) = Self::resolve_candidate(candidate, &inst.base, true, false) {
                return Ok(u);
            }
        }

        Err(Box::new(KodiError::NoImage))
    }

    fn sanitize_display_format(input: &str) -> String {
        // Remove unnecessary spaces
        let mut result = input.split_whitespace().collect::<Vec<&str>>().join(" ");

        // Remove duplicated separators
        while result.contains("{sep}{sep}") || result.contains("{sep} {sep}") {
            result = result.replace("{sep}{sep}", "{sep}");
            result = result.replace("{sep} {sep}", "{sep}");
        }

        // Remove unnecessary separators
        while result.starts_with("{sep}") {
            result = result
                .drain(5..)
                .collect::<String>()
                .trim_start()
                .to_string();
        }

        while result.ends_with("{sep}") {
            result = result
                .drain(..result.len() - 5)
                .collect::<String>()
                .trim_end()
                .to_string();
        }

        result
    }

    fn parse_music_display(&self, input: &str) -> String {
        let mut result = input.trim().to_string();
        let session = self.session.as_ref().unwrap();

        let separator = &self.music_display_options.separator;
        let track = session.now_playing_item.name.as_ref();
        let artists = session.format_artists();
        let genres = session
            .now_playing_item
            .genres
            .as_ref()
            .unwrap_or(&vec!["".to_string()])
            .join(", ");
        let year = session
            .now_playing_item
            .production_year
            .map(|y| y.to_string())
            .unwrap_or_default();
        let album = session
            .now_playing_item
            .album
            .as_ref()
            .unwrap_or(&"".to_string())
            .clone();

        result = result
            .replace("{track}", track)
            .replace("{album}", &album)
            .replace("{artists}", &artists)
            .replace("{genres}", &genres)
            .replace("{year}", &year)
            .replace("{version}", VERSION.unwrap_or("UNKNOWN"));

        Self::sanitize_display_format(&result).replace("{sep}", separator)
    }

    fn parse_movies_display(&self, input: &str) -> String {
        let mut result = input.trim().to_string();
        let session = self.session.as_ref().unwrap();

        let separator = &self.movies_display_options.separator;
        let title = session.now_playing_item.name.as_ref();
        let original_title = session
            .now_playing_item
            .original_title
            .as_ref()
            .unwrap_or(&"".to_string())
            .clone();
        let genres = &session
            .now_playing_item
            .genres
            .as_ref()
            .unwrap_or(&vec!["".to_string()])
            .join(", ");
        let year = session
            .now_playing_item
            .production_year
            .map(|y| y.to_string())
            .unwrap_or_default();
        let critic_score = &session
            .now_playing_item
            .critic_rating
            .map(|s| format!("🍅 {}/100", s))
            .unwrap_or_default();
        let community_score = &session
            .now_playing_item
            .community_rating
            .map(|s| format!("⭐ {:.1}/10", s))
            .unwrap_or_default();

        result = result
            .replace("{title}", title)
            .replace("{original-title}", &original_title)
            .replace("{genres}", &genres)
            .replace("{year}", &year)
            .replace("{critic-score}", critic_score)
            .replace("{community-score}", community_score)
            .replace("{version}", VERSION.unwrap_or("UNKNOWN"));

        Self::sanitize_display_format(&result).replace("{sep}", separator)
    }

    fn parse_episodes_display(&self, input: &str) -> String {
        let mut result = input.trim().to_string();
        let session = self.session.as_ref().unwrap();

        let separator = &self.episodes_display_options.separator;
        let show_title = session
            .now_playing_item
            .series_name
            .as_ref()
            .unwrap_or(&"".to_string())
            .clone();
        let episode_title = session.now_playing_item.name.as_ref();
        let original_title = session
            .now_playing_item
            .original_title
            .as_ref()
            .unwrap_or(&"".to_string())
            .clone();
        let season = session.now_playing_item.parent_index_number.unwrap_or(0);
        let year = session
            .now_playing_item
            .production_year
            .map(|y| y.to_string())
            .unwrap_or_default();
        let genres = session
            .now_playing_item
            .genres
            .as_ref()
            .unwrap_or(&vec!["".to_string()])
            .join(", ");
        let studio = session
            .now_playing_item
            .series_studio
            .as_ref()
            .unwrap_or(&"".to_string())
            .clone();

        let episode = session.now_playing_item.index_number.unwrap_or(0);
        result = result
            .replace("{show-title}", &show_title)
            .replace("{title}", episode_title)
            .replace("{original-title}", &original_title)
            .replace("{episode}", &episode.to_string())
            .replace("{episode-padded}", &format!("{:02}", episode))
            .replace("{season}", &season.to_string())
            .replace("{season-padded}", &format!("{:02}", season))
            .replace("{year}", &year)
            .replace("{genres}", &genres)
            .replace("{studio}", &studio)
            .replace("{version}", VERSION.unwrap_or("UNKNOWN"));

        Self::sanitize_display_format(&result).replace("{sep}", separator)
    }

    fn parse_unknown_display(&self, input: &str) -> String {
        let mut result = input.trim().to_string();
        let session = self.session.as_ref().unwrap();
        let item = &session.now_playing_item;

        let separator = &self.unknown_display_options.separator;
        let addon = item
            .addon_id
            .as_ref()
            .map(|a| a.rsplit('.').next().unwrap_or(a).to_string())
            .unwrap_or_default();
        let addon_full = item.addon_id.clone().unwrap_or_default();
        let file_host = item.file_host_display();
        let genres = item
            .genres
            .as_ref()
            .unwrap_or(&vec!["".to_string()])
            .join(", ");
        let year = item
            .production_year
            .map(|y| y.to_string())
            .unwrap_or_default();
        let studio = item.series_studio.clone().unwrap_or_default();
        let plot = item.plot.clone().unwrap_or_default();

        result = result
            .replace("{title}", &item.name)
            .replace("{label}", &item.label)
            .replace("{addon}", &addon)
            .replace("{addon-full}", &addon_full)
            .replace("{file-host}", &file_host)
            .replace("{genres}", &genres)
            .replace("{year}", &year)
            .replace("{studio}", &studio)
            .replace("{plot}", &plot)
            .replace("{version}", VERSION.unwrap_or("UNKNOWN"));

        Self::sanitize_display_format(&result).replace("{sep}", separator)
    }

    fn get_details(&self) -> String {
        let session = self.session.as_ref().unwrap();

        match session.now_playing_item.media_type {
            MediaType::Music => {
                let display_details_format = &self
                    .music_display_options
                    .display
                    .details_text
                    .as_ref()
                    .unwrap();
                self.parse_music_display(
                    display_details_format
                        .replace("{__default}", "{track}")
                        .as_str(),
                )
            }
            MediaType::Movie => {
                let display_details_format = &self
                    .movies_display_options
                    .display
                    .details_text
                    .as_ref()
                    .unwrap();
                self.parse_movies_display(
                    display_details_format
                        .replace("{__default}", "{title}")
                        .as_str(),
                )
            }
            MediaType::Episode => {
                let display_details_format = &self
                    .episodes_display_options
                    .display
                    .details_text
                    .as_ref()
                    .unwrap();
                self.parse_episodes_display(
                    display_details_format
                        .replace("{__default}", "{show-title}")
                        .as_str(),
                )
            }
            MediaType::AudioBook => session
                .now_playing_item
                .album
                .as_ref()
                .map(|a| a.to_string())
                .unwrap_or_else(|| session.now_playing_item.name.to_string()),
            MediaType::Unknown => {
                let display_details_format = &self
                    .unknown_display_options
                    .display
                    .details_text
                    .as_ref()
                    .unwrap();
                self.parse_unknown_display(
                    display_details_format
                        .replace("{__default}", "{title}")
                        .as_str(),
                )
            }
            _ => session.now_playing_item.name.to_string(),
        }
    }

    fn get_state(&self) -> String {
        let session = self.session.as_ref().unwrap();

        match session.now_playing_item.media_type {
            MediaType::Episode => {
                let display_state_format = &self
                    .episodes_display_options
                    .display
                    .state_text
                    .as_ref()
                    .unwrap();
                self.parse_episodes_display(
                    display_state_format.replace("{__default}", "").as_str(),
                )
            }
            MediaType::LiveTv => {
                // Prefer the channel name when Kodi provides one.
                if !session.now_playing_item.name.is_empty() {
                    format!("Live TV - {}", session.now_playing_item.name)
                } else {
                    "Live TV".to_string()
                }
            }
            MediaType::Music => {
                let display_state_format = &self
                    .music_display_options
                    .display
                    .state_text
                    .as_ref()
                    .unwrap();
                self.parse_music_display(
                    display_state_format
                        .replace("{__default}", "By {artists} {sep} ")
                        .as_str(),
                )
            }
            MediaType::Book => {
                let mut state = String::new();

                if let Some(position_ticks) = session.play_state.position_ticks {
                    let ticks_to_pages = 10000;

                    let page = position_ticks / ticks_to_pages;

                    state += &format!("Reading page {}", page);
                }

                state
            }
            MediaType::AudioBook => {
                let mut state = String::new();

                let artists = session.format_artists();

                let genres = session
                    .now_playing_item
                    .genres
                    .as_ref()
                    .unwrap_or(&vec!["".to_string()])
                    .join(", ");

                if !artists.is_empty() {
                    state += &format!("By {}", artists)
                }

                if !state.is_empty() && !genres.is_empty() {
                    state += " - "
                }

                state += &genres;

                state
            }
            MediaType::Movie => {
                let display_state_format = &self
                    .movies_display_options
                    .display
                    .state_text
                    .as_ref()
                    .unwrap();
                self.parse_movies_display(display_state_format.replace("{__default}", "").as_str())
            }
            MediaType::Unknown => {
                let display_state_format = &self
                    .unknown_display_options
                    .display
                    .state_text
                    .as_ref()
                    .unwrap();
                let fallback = if session.now_playing_item.is_plugin {
                    "via {addon} {sep} {file-host}".to_string()
                } else {
                    "".to_string()
                };
                self.parse_unknown_display(
                    display_state_format
                        .replace("{__default}", &fallback)
                        .as_str(),
                )
            }
            _ => session
                .now_playing_item
                .genres
                .as_ref()
                .unwrap_or(&vec!["".to_string()])
                .join(", "),
        }
    }

    fn get_status_display_type(&self) -> StatusType {
        let session = self.session.as_ref().unwrap();
        match session.now_playing_item.media_type {
            MediaType::Episode => self.episodes_display_options.status_display_type.clone(),
            MediaType::Movie => self.movies_display_options.status_display_type.clone(),
            MediaType::Music => self.music_display_options.status_display_type.clone(),
            MediaType::Unknown => self.unknown_display_options.status_display_type.clone(),
            _ => Default::default(),
        }
    }

    fn get_image_text(&self) -> String {
        let session = self.session.as_ref().unwrap();

        match session.now_playing_item.media_type {
            MediaType::Music => {
                let display_image_format = &self
                    .music_display_options
                    .display
                    .image_text
                    .as_ref()
                    .unwrap();
                self.parse_music_display(display_image_format)
            }
            MediaType::Movie => {
                let display_image_format = &self
                    .movies_display_options
                    .display
                    .image_text
                    .as_ref()
                    .unwrap();
                self.parse_movies_display(display_image_format)
            }
            MediaType::Episode => {
                let display_image_format = &self
                    .episodes_display_options
                    .display
                    .image_text
                    .as_ref()
                    .unwrap();
                self.parse_episodes_display(display_image_format)
            }
            MediaType::Unknown => {
                let display_image_format = &self
                    .unknown_display_options
                    .display
                    .image_text
                    .as_ref()
                    .unwrap();
                self.parse_unknown_display(display_image_format)
            }
            _ => "".to_string(),
        }
    }

    fn check_blacklist(&self) -> KodiResult<bool> {
        let session = self.session.as_ref().unwrap();

        if self
            .blacklist
            .media_types
            .iter()
            .any(|m| m == &session.now_playing_item.media_type)
        {
            return Ok(true);
        }

        if self.blacklist.check_item(&session.now_playing_item) {
            return Ok(true);
        }

        Ok(false)
    }
}

pub struct EpisodeDisplayOptions {
    pub divider: bool,
    pub prefix: bool,
    pub simple: bool,
}

struct DisplayOptions {
    separator: String,
    display: DisplayFormat,
    status_display_type: StatusType,
    poster_source: PosterSource,
    show_paused: bool,
    buttons: Option<Vec<Button>>,
}

/// Which artwork episodes use for the presence image.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PosterSource {
    /// Season poster, falling back to the series poster (default).
    #[default]
    Season,
    /// Series poster only (no season lookup).
    Series,
    /// Episode still only (no poster lookups at all).
    Episode,
}

impl From<String> for PosterSource {
    fn from(v: String) -> Self {
        match v.to_lowercase().as_str() {
            "series" => Self::Series,
            "episode" | "still" => Self::Episode,
            _ => Self::Season,
        }
    }
}

/// Represents the formatting details for `Display`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct DisplayFormat {
    /// First line of the activity.
    pub details_text: Option<String>,
    /// Second line of the activity.
    pub state_text: Option<String>,
    /// Third line / large image text of the activity.
    pub image_text: Option<String>,
}

/// Converts legacy `Vec<String>` to `DisplayFormat`
impl From<Vec<String>> for DisplayFormat {
    fn from(items: Vec<String>) -> Self {
        let details_text = "{__default}".to_string();
        let image_text = "Kodi-RPC v{version}".to_string();
        let mut state_text = "{__default}".to_string();

        let items_joined = items
            .iter()
            .map(|i| format!("{{{}}}", i.trim()))
            .collect::<Vec<String>>()
            .join(" {sep} ");

        if !items_joined.is_empty() {
            state_text += &items_joined;
        }

        DisplayFormat {
            details_text: Some(details_text),
            state_text: Some(state_text),
            image_text: Some(image_text),
        }
    }
}

/// Reuses `DisplayFormat::from(Vec<String>)`
impl From<String> for DisplayFormat {
    fn from(item: String) -> Self {
        let data: Vec<String> = item.split(',').map(|d| d.to_string()).collect();
        DisplayFormat::from(data)
    }
}

/// Converts `EpisodeDisplayOptions` to `DisplayFormat`
impl From<EpisodeDisplayOptions> for DisplayFormat {
    fn from(value: EpisodeDisplayOptions) -> Self {
        let details_text = "{show-title}".to_string();
        let state_text = {
            let (season_tag, episode_tag) = if value.prefix {
                (
                    "S{season-padded}".to_string(),
                    "E{episode-padded}".to_string(),
                )
            } else {
                ("S{season}".to_string(), "E{episode}".to_string())
            };

            let divider = if value.divider { " - " } else { "" };

            if value.simple {
                format!("{}{}{}", season_tag, divider, episode_tag)
            } else {
                format!("{}{}{} {}", season_tag, divider, episode_tag, "{title}")
            }
        };
        let image_text = "Kodi-RPC v{version}".to_string();

        DisplayFormat {
            details_text: Some(details_text),
            state_text: Some(state_text),
            image_text: Some(image_text),
        }
    }
}

#[derive(Clone, Debug)]
pub enum StatusType {
    Name,
    State,
    Details,
}
impl Default for StatusType {
    fn default() -> Self {
        Self::Name
    }
}

impl From<DiscordIpcStatusDisplayType> for StatusType {
    fn from(x: DiscordIpcStatusDisplayType) -> Self {
        use DiscordIpcStatusDisplayType as T;
        match x {
            T::Name => Self::Name,
            T::State => Self::State,
            T::Details => Self::Details,
        }
    }
}

impl Into<DiscordIpcStatusDisplayType> for StatusType {
    fn into(self) -> DiscordIpcStatusDisplayType {
        use DiscordIpcStatusDisplayType as T;
        match self {
            Self::Name => T::Name,
            Self::State => T::State,
            Self::Details => T::Details,
        }
    }
}

#[derive(Debug)]
pub struct StatusTypeFromStringError;

impl TryFrom<String> for StatusType {
    type Error = StatusTypeFromStringError;
    fn try_from(x: String) -> Result<Self, Self::Error> {
        match x.as_ref() {
            "name" => Ok(Self::Name),
            "state" => Ok(Self::State),
            "details" => Ok(Self::Details),
            _ => Err(StatusTypeFromStringError),
        }
    }
}

struct Blacklist {
    media_types: Vec<MediaType>,
    libraries_names: Vec<String>,
}

impl Blacklist {
    /// Check whether a [NowPlayingItem] is blacklisted.
    ///
    /// Unlike Jellyfin there is no VirtualFolders API to resolve: `libraries`
    /// entries are matched as case-insensitive substrings against the Kodi
    /// `file` path (works for `plugin://`, `smb://`, local paths, ...).
    fn check_item(&self, playing_item: &NowPlayingItem) -> bool {
        debug!("Checking if an item is blacklisted: {}", playing_item.name);
        self.check_path(playing_item.path.as_ref().unwrap_or(&String::new()))
    }

    /// Check whether a path is in a blacklisted library
    fn check_path(&self, item_path: &str) -> bool {
        let hay = item_path.to_lowercase();
        let hit = self.libraries_names.iter().any(|lib| {
            let needle = lib.to_lowercase();
            !needle.is_empty() && hay.contains(&needle)
        });
        if hit {
            debug!("Blacklisted path hit: {}", item_path);
        }
        hit
    }
}

struct ImgurOptions {
    enabled: bool,
    client_id: String,
    urls_location: String,
}

struct LitterboxOptions {
    enabled: bool,
    urls_location: String,
}

/// Used to build a new Client
#[derive(Default)]
pub struct ClientBuilder {
    url: String,
    username: String,
    password: String,
    client_id: String,
    self_signed: bool,
    extra_instances: Vec<InstanceCfg>,
    episode_divider: bool,
    episode_prefix: bool,
    episode_simple: bool,
    music_separator: String,
    music_display: DisplayFormat,
    music_status_display_type: StatusType,
    music_show_paused: bool,
    music_buttons: Option<Vec<Button>>,
    movies_separator: String,
    movies_display: DisplayFormat,
    movies_status_display_type: StatusType,
    movies_show_paused: bool,
    movies_buttons: Option<Vec<Button>>,
    episodes_separator: String,
    episodes_display: DisplayFormat,
    episodes_status_display_type: StatusType,
    episodes_poster_source: PosterSource,
    episodes_show_paused: bool,
    episodes_buttons: Option<Vec<Button>>,
    unknown_separator: String,
    unknown_display: DisplayFormat,
    unknown_status_display_type: StatusType,
    unknown_show_paused: bool,
    unknown_buttons: Option<Vec<Button>>,
    blacklist_media_types: Vec<MediaType>,
    blacklist_libraries: Vec<String>,
    show_images: bool,
    use_imgur: bool,
    imgur_client_id: String,
    imgur_urls_file_location: String,
    use_litterbox: bool,
    litterbox_urls_file_location: String,
    large_image_text: String,
    process_images: bool,
    image_size: Option<u32>,
    image_background: bool,
    image_background_blur: f32,
    image_corner_radius: Option<f32>,
}

impl ClientBuilder {
    /// Returns a ClientBuilder with some default options set
    pub fn new() -> Self {
        Self {
            client_id: "1549062924545556480".to_string(),
            music_separator: "-".to_string(),
            music_display: DisplayFormat::from(vec!["genres".to_string()]),
            movies_separator: "-".to_string(),
            movies_display: DisplayFormat::from(vec!["genres".to_string()]),
            episodes_separator: "-".to_string(),
            episodes_display: DisplayFormat::from(EpisodeDisplayOptions {
                divider: true,
                prefix: true,
                simple: false,
            }),
            unknown_separator: "-".to_string(),
            unknown_display: DisplayFormat {
                details_text: Some("{title}".to_string()),
                state_text: Some("via {addon} {sep} {file-host}".to_string()),
                image_text: Some("Kodi-RPC v{version}".to_string()),
            },
            music_show_paused: true,
            movies_show_paused: true,
            episodes_show_paused: true,
            unknown_show_paused: true,
            process_images: true,
            image_background: true,
            image_background_blur: 3.0,
            image_corner_radius: Some(4.0),
            ..Default::default()
        }
    }

    /// Kodi base URL (e.g. `https://kodi.example.com` or `http://localhost:8080`).
    pub fn url<T: Into<String>>(&mut self, url: T) -> &mut Self {
        self.url = url.into();
        self
    }

    /// HTTP username for Kodi (`Settings → Services → Control`).
    /// Empty = no auth.
    pub fn username<T: Into<String>>(&mut self, username: T) -> &mut Self {
        self.username = username.into();
        self
    }

    /// HTTP password for Kodi. Empty = no auth.
    pub fn password<T: Into<String>>(&mut self, password: T) -> &mut Self {
        self.password = password.into();
        self
    }

    /// Discord Application ID that the client will use when connecting to Discord.
    pub fn client_id<T: Into<String>>(&mut self, client_id: T) -> &mut Self {
        self.client_id = client_id.into();
        self
    }

    /// Controls the use of certificate validation in reqwest.
    pub fn self_signed(&mut self, self_signed: bool) -> &mut Self {
        self.self_signed = self_signed;
        self
    }

    /// Extra Kodi servers to watch. The primary server (from `url`) plus
    /// these are polled every cycle; whichever plays first owns the presence
    /// until it stops.
    pub fn instances(&mut self, instances: Vec<InstanceCfg>) -> &mut Self {
        self.extra_instances.extend(instances);
        self
    }

    /// buttons to be displayed on the activity.
    pub fn episode_divider(&mut self, val: bool) -> &mut Self {
        self.episode_divider = val;
        self
    }

    pub fn episode_prefix(&mut self, val: bool) -> &mut Self {
        self.episode_prefix = val;
        self
    }

    pub fn episode_simple(&mut self, val: bool) -> &mut Self {
        self.episode_simple = val;
        self
    }

    pub fn music_separator<T: Into<String>>(&mut self, separator: T) -> &mut Self {
        self.music_separator = separator.into();
        self
    }

    pub fn music_display(&mut self, display: DisplayFormat) -> &mut Self {
        self.music_display = display;
        self
    }

    pub fn music_status_display_type(&mut self, status_type: StatusType) -> &mut Self {
        self.music_status_display_type = status_type;
        self
    }

    /// Show music activity when paused.
    pub fn music_show_paused(&mut self, val: bool) -> &mut Self {
        self.music_show_paused = val;
        self
    }

    /// Buttons for music activity. Empty vec = no buttons.
    pub fn music_buttons(&mut self, buttons: Vec<Button>) -> &mut Self {
        self.music_buttons = Some(buttons);
        self
    }

    pub fn movies_separator<T: Into<String>>(&mut self, separator: T) -> &mut Self {
        self.movies_separator = separator.into();
        self
    }

    pub fn movies_display(&mut self, display: DisplayFormat) -> &mut Self {
        self.movies_display = display;
        self
    }

    pub fn movies_status_display_type(&mut self, status_type: StatusType) -> &mut Self {
        self.movies_status_display_type = status_type;
        self
    }

    /// Show movie activity when paused.
    pub fn movies_show_paused(&mut self, val: bool) -> &mut Self {
        self.movies_show_paused = val;
        self
    }

    /// Buttons for movie activity. Empty vec = no buttons.
    pub fn movies_buttons(&mut self, buttons: Vec<Button>) -> &mut Self {
        self.movies_buttons = Some(buttons);
        self
    }

    pub fn episodes_separator<T: Into<String>>(&mut self, separator: T) -> &mut Self {
        self.episodes_separator = separator.into();
        self
    }

    pub fn episodes_display(&mut self, display: DisplayFormat) -> &mut Self {
        self.episodes_display = display;
        self
    }

    pub fn episodes_status_display_type(&mut self, status_type: StatusType) -> &mut Self {
        self.episodes_status_display_type = status_type;
        self
    }

    /// Episode artwork: season poster (default), series poster, or still.
    pub fn episodes_poster_source(&mut self, source: PosterSource) -> &mut Self {
        self.episodes_poster_source = source;
        self
    }

    /// Show episode activity when paused.
    pub fn episodes_show_paused(&mut self, val: bool) -> &mut Self {
        self.episodes_show_paused = val;
        self
    }

    /// Buttons for episode activity. Empty vec = no buttons.
    pub fn episodes_buttons(&mut self, buttons: Vec<Button>) -> &mut Self {
        self.episodes_buttons = Some(buttons);
        self
    }

    pub fn unknown_separator<T: Into<String>>(&mut self, separator: T) -> &mut Self {
        self.unknown_separator = separator.into();
        self
    }

    pub fn unknown_display(&mut self, display: DisplayFormat) -> &mut Self {
        self.unknown_display = display;
        self
    }

    pub fn unknown_status_display_type(&mut self, status_type: StatusType) -> &mut Self {
        self.unknown_status_display_type = status_type;
        self
    }

    /// Show plugin/unknown activity when paused.
    pub fn unknown_show_paused(&mut self, val: bool) -> &mut Self {
        self.unknown_show_paused = val;
        self
    }

    /// Buttons for plugin/unknown activity. Empty vec = no buttons.
    pub fn unknown_buttons(&mut self, buttons: Vec<Button>) -> &mut Self {
        self.unknown_buttons = Some(buttons);
        self
    }

    /// Blacklist certain `MediaType`s so they don't display.
    pub fn blacklist_media_types(&mut self, media_types: Vec<MediaType>) -> &mut Self {
        self.blacklist_media_types = media_types;
        self
    }

    /// Blacklist path substrings (matched against the Kodi `file` path,
    /// e.g. `"plugin.video.foo"`, `"Kids"`, `"smb://nas/adult"`).
    pub fn blacklist_libraries(&mut self, libraries: Vec<String>) -> &mut Self {
        self.blacklist_libraries = libraries;
        self
    }

    /// Show activity when paused.
    /// Show images from Kodi on the activity.
    pub fn show_images(&mut self, val: bool) -> &mut Self {
        self.show_images = val;
        self
    }

    /// Use imgur for images
    pub fn use_imgur(&mut self, val: bool) -> &mut Self {
        self.use_imgur = val;
        self
    }

    /// Imgur client id
    pub fn imgur_client_id<T: Into<String>>(&mut self, client_id: T) -> &mut Self {
        self.imgur_client_id = client_id.into();
        self
    }

    pub fn imgur_urls_file_location<T: Into<String>>(&mut self, location: T) -> &mut Self {
        self.imgur_urls_file_location = location.into();
        self
    }

    /// Use litterbox.catbox.moe for images
    pub fn use_litterbox(&mut self, val: bool) -> &mut Self {
        self.use_litterbox = val;
        self
    }

    pub fn litterbox_urls_file_location<T: Into<String>>(&mut self, location: T) -> &mut Self {
        self.litterbox_urls_file_location = location.into();
        self
    }

    /// Process images before uploading to imgur or litterbox
    pub fn process_images(&mut self, val: bool) -> &mut Self {
        self.process_images = val;
        self
    }

    pub fn image_size(&mut self, size: Option<u32>) -> &mut Self {
        self.image_size = size;
        self
    }

    pub fn image_background(&mut self, val: bool) -> &mut Self {
        self.image_background = val;
        self
    }

    pub fn image_background_blur(&mut self, blur: f32) -> &mut Self {
        self.image_background_blur = blur;
        self
    }

    pub fn image_corner_radius(&mut self, radius: Option<f32>) -> &mut Self {
        self.image_corner_radius = radius;
        self
    }

    /// Text to be displayed when hovering the large activity image in Discord
    pub fn large_image_text<T: Into<String>>(&mut self, text: T) -> &mut Self {
        self.large_image_text = text.into();
        self
    }

    /// Builds a client from the options specified in the builder.
    ///
    /// The primary server comes from [`ClientBuilder::url`] (+ credentials);
    /// [`ClientBuilder::instances`] appends more. At least one usable server
    /// is required.
    pub fn build(self) -> KodiResult<Client> {
        let mut cfgs: Vec<InstanceCfg> = Vec::new();
        if !self.url.is_empty() {
            cfgs.push(InstanceCfg {
                url: self.url.clone(),
                username: self.username.clone(),
                password: self.password.clone(),
                self_signed_cert: self.self_signed,
                name: String::new(),
            });
        }
        cfgs.extend(self.extra_instances.clone());

        if cfgs.is_empty() {
            return Err(Box::new(KodiError::MissingRequiredValues));
        }

        let mut instances = Vec::with_capacity(cfgs.len());
        for cfg in cfgs {
            if cfg.url.is_empty() {
                continue;
            }
            let mut base: Url = cfg.url.parse()?;
            // Trailing slash so `join("jsonrpc")` never drops the last path
            // segment of reverse-proxied hosts.
            if !base.path().ends_with('/') {
                let with_slash = format!("{}/", base.as_str().trim_end_matches('/'));
                base = with_slash.parse()?;
            }
            let name = if cfg.name.is_empty() {
                base.as_str().trim_end_matches('/').to_string()
            } else {
                cfg.name.clone()
            };
            instances.push(KodiInstance {
                direct_art: base_allows_direct_art(&base, &cfg.username),
                http: build_kodi_http_client(
                    &base,
                    &cfg.username,
                    &cfg.password,
                    cfg.self_signed_cert,
                )?,
                base,
                name,
            });
        }

        if instances.is_empty() {
            return Err(Box::new(KodiError::MissingRequiredValues));
        }

        Ok(Client {
            discord_ipc_client: DiscordIpcClient::new(&self.client_id),
            instances,
            session: None,
            session_owner: None,
            music_display_options: DisplayOptions {
                separator: self.music_separator,
                display: self.music_display,
                status_display_type: self.music_status_display_type,
                poster_source: PosterSource::default(),
                show_paused: self.music_show_paused,
                buttons: self.music_buttons,
            },
            movies_display_options: DisplayOptions {
                separator: self.movies_separator,
                display: self.movies_display,
                status_display_type: self.movies_status_display_type,
                poster_source: PosterSource::default(),
                show_paused: self.movies_show_paused,
                buttons: self.movies_buttons,
            },
            episodes_display_options: DisplayOptions {
                separator: self.episodes_separator,
                display: self.episodes_display,
                status_display_type: self.episodes_status_display_type,
                poster_source: self.episodes_poster_source,
                show_paused: self.episodes_show_paused,
                buttons: self.episodes_buttons,
            },
            unknown_display_options: DisplayOptions {
                separator: self.unknown_separator,
                display: self.unknown_display,
                status_display_type: self.unknown_status_display_type,
                poster_source: PosterSource::default(),
                show_paused: self.unknown_show_paused,
                buttons: self.unknown_buttons,
            },
            blacklist: Blacklist {
                media_types: self.blacklist_media_types,
                libraries_names: self.blacklist_libraries,
            },
            show_images: self.show_images,
            imgur_options: ImgurOptions {
                enabled: self.use_imgur,
                client_id: self.imgur_client_id,
                urls_location: self.imgur_urls_file_location,
            },
            litterbox_options: LitterboxOptions {
                enabled: self.use_litterbox,
                urls_location: self.litterbox_urls_file_location,
            },
            process_images: self.process_images,
            image_processing_options: external::image_utils::ImageProcessingOptions {
                size: self.image_size,
                background: self.image_background,
                background_blur: self.image_background_blur,
                corner_radius: self.image_corner_radius,
            },
            large_image_text: self.large_image_text,
            poster_cache: HashMap::new(),
            crunchy_cache: HashMap::new(),
        })
    }
}

/// True only for a publicly reachable Kodi without HTTP auth.
fn base_allows_direct_art(base: &Url, username: &str) -> bool {
    if !username.is_empty() {
        return false;
    }
    let host = match base.host_str() {
        Some(h) => h.trim_end_matches('.').to_lowercase(),
        None => return false,
    };
    if host.is_empty()
        || host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host == "invalid"
        || host.ends_with(".invalid")
    {
        return false;
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        match ip {
            std::net::IpAddr::V4(v4) => {
                if v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_unspecified()
                {
                    return false;
                }
            }
            std::net::IpAddr::V6(v6) => {
                if v6.is_loopback() || v6.is_unspecified() {
                    return false;
                }
                // unique local fc00::/7
                if (v6.segments()[0] & 0xfe00) == 0xfc00 {
                    return false;
                }
            }
        }
    }
    true
}

fn build_kodi_http_client(
    _base: &Url,
    username: &str,
    password: &str,
    self_signed: bool,
) -> KodiResult<reqwest::blocking::Client> {
    use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
    use std::time::Duration;

    let mut headers = HeaderMap::new();
    if !username.is_empty() {
        // Minimal base64 to avoid adding a dependency (Kodi uses HTTP Basic).
        let encoded = base64_encode(&format!("{}:{}", username, password));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Basic {}", encoded))?,
        );
    }

    Ok(reqwest::blocking::Client::builder()
        .default_headers(headers)
        .danger_accept_invalid_certs(self_signed)
        // Bound every RPC: one stalled instance must never freeze polling.
        .timeout(Duration::from_secs(10))
        .build()?)
}

// Small, dependency-free base64 encoder (standard alphabet, padded).
fn base64_encode(input: &str) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(((bytes.len() + 2) / 3) * 4);
    for chunk in bytes.chunks(3) {
        let mut buf = [0u8; 3];
        for (i, b) in chunk.iter().enumerate() {
            buf[i] = *b;
        }
        let n = ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32);
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}
