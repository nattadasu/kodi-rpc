use kodi_rpc::{Button, DisplayFormat, MediaType, PosterSource, StatusType};
use log::debug;
use serde::{Deserialize, Serialize};
use std::env;

/// Main struct containing every other struct in the file.
///
/// The config file is parsed into this struct.
pub struct Config {
    /// Kodi configuration.
    pub kodi: Kodi,
    /// Discord configuration.
    pub discord: Discord,
    /// Imgur configuration.
    pub imgur: Imgur,
    /// Images configuration.
    pub images: Images,
}

/// This struct contains every "required" part of the config.
pub struct Kodi {
    /// Watched servers. Single dict-style configs resolve to exactly one entry.
    pub instances: Vec<KodiInstance>,
    /// Contains configuration for Music display.
    pub music: DisplayOptions,
    /// Contains configuration for Movie display.
    pub movies: DisplayOptions,
    /// Contains configuration for Episode display.
    pub episodes: DisplayOptions,
    /// Contains configuration for Unknown / 3rd-party plugin display.
    pub unknown: DisplayOptions,
    /// Blacklist configuration.
    pub blacklist: Blacklist,
    /// Simple episode name
    pub show_simple: bool,
    /// Add "0" before season/episode number if lower than 10.
    pub append_prefix: bool,
    /// Add a divider between numbers
    pub add_divider: bool,
}

/// One watched Kodi server.
pub struct KodiInstance {
    pub name: String,
    pub url: String,
    pub username: String,
    pub password: String,
    pub self_signed_cert: bool,
}

/// Contains configuration for Music/Movie display.
pub struct DisplayOptions {
    /// Display is where you tell the program what should be displayed.
    pub display: Option<DisplayFormat>,
    /// Separator is what should be between the artist(s) and the `display` options.
    pub separator: Option<String>,
    /// Whether the to display the name, state, or details in the status title.
    pub status_display_type: Option<StatusType>,
    /// Episode artwork: season poster, series poster, or episode still.
    pub poster_source: PosterSource,
    /// Show this section while paused. Resolved: section value wins,
    /// otherwise the discord default.
    pub show_paused: bool,
    /// Buttons for this section. Resolved: section value wins (even an
    /// empty list, which hides all buttons), otherwise the discord default.
    pub buttons: Option<Vec<Button>>,
}

/// Discord configuration.
///
/// `show_paused` / `buttons` live here only as defaults: each display
/// section resolves its own value from them at load time.
pub struct Discord {
    /// Set a custom Application ID to be used.
    pub application_id: Option<String>,
}

/// Images configuration
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Images {
    /// Enables images, not everyone wants them so its a toggle.
    pub enable_images: bool,
    /// Enables imgur images.
    pub imgur_images: bool,
    /// Enables litterbox images.
    pub litterbox_images: bool,
    /// Processes images by making them square and adding a blur.
    pub process_images: bool,
    /// The size of the output square image canvas (e.g., 512 for 512x512px).
    pub size: Option<u32>,
    /// Whether to create a blurred background. Default: true.
    pub bg: bool,
    /// The blur radius for the background image as a percentage of canvas size.
    /// Default: 3.
    pub bg_blur: f32,
    /// Corner radius as a percentage of the image size.
    /// Only applied when background is disabled. Default: 4.
    pub corner_radius: Option<f32>,
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub struct ConfigBuilder {
    pub kodi: KodiBuilder,
    pub discord: Option<DiscordBuilder>,
    pub imgur: Option<Imgur>,
    pub images: Option<ImagesBuilder>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct KodiBuilder {
    #[serde(default)]
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    /// Alternate base URLs for this server. When non-empty, each entry
    /// becomes its own polled instance (same credentials) and `url` is
    /// ignored.
    pub urls: Option<Vec<String>>,
    /// Extra servers. When non-empty, the top-level url/credentials are ignored.
    pub instances: Option<Vec<KodiInstanceBuilder>>,
    pub music: Option<DisplayOptionsBuilder>,
    pub movies: Option<DisplayOptionsBuilder>,
    pub episodes: Option<DisplayOptionsBuilder>,
    pub unknown: Option<DisplayOptionsBuilder>,
    pub blacklist: Option<Blacklist>,
    pub self_signed_cert: Option<bool>,
    pub show_simple: Option<bool>,
    pub append_prefix: Option<bool>,
    pub add_divider: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct KodiInstanceBuilder {
    #[serde(default)]
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub self_signed_cert: Option<bool>,
    pub name: Option<String>,
    /// Alternate base URLs for this server. When non-empty, each entry
    /// becomes its own polled instance (same credentials) and `url` is
    /// ignored.
    pub urls: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DisplayOptionsBuilder {
    pub display: Option<Display>,
    pub separator: Option<String>,
    pub status_display_type: Option<String>,
    pub poster_source: Option<String>,
    pub show_paused: Option<bool>,
    pub buttons: Option<Vec<Button>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum Display {
    /// If the Display is a `Vec<String>`.
    Vec(Vec<String>),
    /// If the Display is a comma separated `String`.
    String(String),
    /// If the Display is a `DisplayFormat` struct.
    CustomFormat(DisplayFormat),
}

/// Blacklist MediaTypes and libraries (path substrings for Kodi).
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Blacklist {
    /// `Vec<String>` of MediaTypes to blacklist
    pub media_types: Option<Vec<MediaType>>,
    /// `Vec<String>` of path substrings to blacklist
    /// (e.g. `"plugin.video.foo"`, `"Kids"`, `"smb://nas/private"`).
    pub libraries: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DiscordBuilder {
    pub application_id: Option<String>,
    pub buttons: Option<Vec<Button>>,
    pub show_paused: Option<bool>,
}

/// Imgur configuration
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Imgur {
    /// Contains the client ID used to upload images to imgur.
    pub client_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ImagesBuilder {
    pub enable_images: Option<bool>,
    pub imgur_images: Option<bool>,
    pub litterbox_images: Option<bool>,
    pub process_images: Option<bool>,
    /// The size of the output square image canvas (e.g., 512 for 512x512px).
    pub size: Option<u32>,
    /// Whether to create a blurred background. Default: true.
    pub bg: Option<bool>,
    /// The blur radius for the background image as a percentage of canvas size.
    /// Default: 3.
    pub bg_blur: Option<f32>,
    /// Corner radius as a percentage of the image size.
    /// Only applied when background is disabled. Default: 4.
    pub corner_radius: Option<f32>,
}

/// Find urls.json in filesystem, used to store images that were already previously uploaded to imgur.
///
/// Default urls.json path depends on OS
/// Windows: `%appdata%\kodi-rpc\urls.json`
/// Linux/macOS: `~/.config/kodi-rpc/urls.json`
pub fn get_urls_path() -> Result<String, Box<dyn std::error::Error>> {
    if cfg!(not(windows)) {
        debug!("Platform is not Windows");
        let xdg_config_home = match env::var("XDG_CONFIG_HOME") {
            Ok(xdg_config_home) => xdg_config_home,
            Err(_) => env::var("HOME")? + "/.config",
        };

        Ok(xdg_config_home + ("/kodi-rpc/urls.json"))
    } else {
        debug!("Platform is Windows");
        let app_data = env::var("APPDATA")?;
        Ok(app_data + r"\kodi-rpc\urls.json")
    }
}

/// Find default config path (main.json) in filesystem.
///
/// Default config path depends on OS
/// Windows: `%appdata%\kodi-rpc\main.json`
/// Linux/macOS: `~/.config/kodi-rpc/main.json`
pub fn get_config_path() -> Result<String, Box<dyn std::error::Error>> {
    debug!("Getting config path");
    if cfg!(not(windows)) {
        debug!("Platform is not Windows");
        let xdg_config_home = match env::var("XDG_CONFIG_HOME") {
            Ok(xdg_config_home) => xdg_config_home,
            Err(_) => env::var("HOME")? + "/.config",
        };

        Ok(xdg_config_home + "/kodi-rpc/main.json")
    } else {
        debug!("Platform is Windows");
        let app_data = env::var("APPDATA")?;
        Ok(app_data + r"\kodi-rpc\main.json")
    }
}

impl ConfigBuilder {
    fn new() -> Self {
        Self {
            kodi: KodiBuilder {
                url: "".to_string(),
                username: None,
                password: None,
                urls: None,
                instances: None,
                music: None,
                movies: None,
                episodes: None,
                unknown: None,
                blacklist: None,
                self_signed_cert: None,
                show_simple: Some(false),
                append_prefix: Some(false),
                add_divider: Some(false),
            },
            discord: None,
            imgur: None,
            images: None,
        }
    }

    /// Loads the config from the given path.
    ///
    /// Accepts both the new `{"kodi": ...}` shape and the legacy
    /// `{"jellyfin": ...}` shape from jellyfin-rpc (mapped automatically).
    pub fn load(self, path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        debug!("Config path is: {}", path);

        let data = std::fs::read_to_string(path)?;
        // Backwards compat: jellyfin-rpc configs use `jellyfin.url/api_key/username`.
        // Accept them by rewriting the top-level key before parsing.
        let mut value: serde_json::Value = serde_json::from_str(&data)?;
        if value.get("kodi").is_none() {
            if let Some(jf) = value.get("jellyfin").cloned() {
                debug!("Found legacy jellyfin config block, mapping to kodi");
                if let Some(obj) = value.as_object_mut() {
                    obj.insert("kodi".to_string(), jf);
                }
            }
        }
        let config: ConfigBuilder = serde_json::from_value(value)?;

        debug!("Config loaded successfully");

        Ok(config)
    }

    fn parse_display_options(
        input: Option<DisplayOptionsBuilder>,
    ) -> (
        Option<DisplayFormat>,
        Option<String>,
        Option<StatusType>,
        PosterSource,
        Option<bool>,
        Option<Vec<Button>>,
    ) {
        if let Some(opts) = input {
            let display = opts.display.map(|disp| match disp {
                Display::Vec(display) => DisplayFormat::from(display),
                Display::String(display) => DisplayFormat::from(display),
                Display::CustomFormat(display) => display,
            });
            let separator = opts.separator;
            let status = opts
                .status_display_type
                .and_then(|x| StatusType::try_from(x).ok());
            let poster_source = opts
                .poster_source
                .map(PosterSource::from)
                .unwrap_or_default();
            (
                display,
                separator,
                status,
                poster_source,
                opts.show_paused,
                opts.buttons,
            )
        } else {
            (None, None, None, PosterSource::default(), None, None)
        }
    }

    pub fn build(self) -> Config {
        let (
            music_display,
            music_separator,
            music_status_display_type,
            music_poster,
            music_paused,
            music_buttons,
        ) = Self::parse_display_options(self.kodi.music);
        let (
            movie_display,
            movie_separator,
            movie_status_display_type,
            movies_poster,
            movies_paused,
            movies_buttons,
        ) = Self::parse_display_options(self.kodi.movies);
        let (
            episode_display,
            episode_separator,
            episode_status_display_type,
            episodes_poster,
            episodes_paused,
            episodes_buttons,
        ) = Self::parse_display_options(self.kodi.episodes);
        let (
            unknown_display,
            unknown_separator,
            unknown_status_display_type,
            unknown_poster,
            unknown_paused,
            unknown_buttons,
        ) = Self::parse_display_options(self.kodi.unknown);

        let media_types;
        let libraries;

        if let Some(blacklist) = self.kodi.blacklist {
            media_types = blacklist.media_types;
            libraries = blacklist.libraries;
        } else {
            media_types = None;
            libraries = None;
        }

        let application_id;
        let buttons;
        let show_paused;

        if let Some(discord) = self.discord {
            application_id = discord.application_id;
            buttons = discord.buttons;
            show_paused = discord.show_paused.unwrap_or(true)
        } else {
            application_id = None;
            buttons = None;
            show_paused = true;
        }

        let client_id;

        if let Some(imgur) = self.imgur {
            client_id = imgur.client_id;
        } else {
            client_id = None
        }

        let enable_images;
        let imgur_images;
        let litterbox_images;
        let process_images;
        let image_size;
        let image_bg;
        let image_bg_blur;
        let image_corner_radius;

        if let Some(images) = self.images {
            enable_images = images.enable_images.unwrap_or(false);
            imgur_images = images.imgur_images.unwrap_or(false);
            litterbox_images = images.litterbox_images.unwrap_or(false);
            process_images = images.process_images.unwrap_or(true);
            image_size = images.size;
            image_bg = images.bg.unwrap_or(true);
            image_bg_blur = images.bg_blur.unwrap_or(3.0);
            image_corner_radius = images.corner_radius.or(Some(4.0));
        } else {
            enable_images = false;
            imgur_images = false;
            litterbox_images = false;
            process_images = true;
            image_size = None;
            image_bg = true;
            image_bg_blur = 3.0;
            image_corner_radius = Some(4.0);
        }

        // One entry becomes one polled instance per URL in its `urls`
        // list (trumping `url`), or a single instance from `url`.
        fn expand(
            url: &str,
            urls: Option<&Vec<String>>,
            username: Option<String>,
            password: Option<String>,
            self_signed_cert: Option<bool>,
            name: Option<String>,
        ) -> Vec<KodiInstance> {
            let targets: Vec<String> = match urls {
                Some(list) if list.iter().any(|u| !u.trim().is_empty()) => list
                    .iter()
                    .filter(|u| !u.trim().is_empty())
                    .cloned()
                    .collect(),
                _ => vec![url.to_string()],
            };
            targets
                .into_iter()
                .filter(|u| !u.trim().is_empty())
                .map(|u| {
                    let display = name.clone().unwrap_or_else(|| u.clone());
                    KodiInstance {
                        name: display,
                        url: u,
                        username: username.clone().unwrap_or_default(),
                        password: password.clone().unwrap_or_default(),
                        self_signed_cert: self_signed_cert.unwrap_or(false),
                    }
                })
                .collect()
        }

        let instances: Vec<KodiInstance> = match &self.kodi.instances {
            Some(list) if list.iter().any(|i| !i.url.is_empty() || has_urls(&i.urls)) => list
                .iter()
                .flat_map(|i| {
                    expand(
                        &i.url,
                        i.urls.as_ref(),
                        i.username.clone(),
                        i.password.clone(),
                        i.self_signed_cert,
                        i.name.clone(),
                    )
                })
                .collect(),
            _ => expand(
                &self.kodi.url,
                self.kodi.urls.as_ref(),
                self.kodi.username.clone(),
                self.kodi.password.clone(),
                self.kodi.self_signed_cert,
                None,
            ),
        };

        fn has_urls(urls: &Option<Vec<String>>) -> bool {
            urls.as_ref()
                .is_some_and(|list| list.iter().any(|u| !u.trim().is_empty()))
        }

        Config {
            kodi: Kodi {
                instances,
                music: DisplayOptions {
                    display: music_display,
                    separator: music_separator,
                    status_display_type: music_status_display_type,
                    poster_source: music_poster,
                    show_paused: music_paused.unwrap_or(show_paused),
                    buttons: music_buttons.or(buttons.clone()),
                },
                movies: DisplayOptions {
                    display: movie_display,
                    separator: movie_separator,
                    status_display_type: movie_status_display_type,
                    poster_source: movies_poster,
                    show_paused: movies_paused.unwrap_or(show_paused),
                    buttons: movies_buttons.or(buttons.clone()),
                },
                episodes: DisplayOptions {
                    display: episode_display,
                    separator: episode_separator,
                    status_display_type: episode_status_display_type,
                    poster_source: episodes_poster,
                    show_paused: episodes_paused.unwrap_or(show_paused),
                    buttons: episodes_buttons.or(buttons.clone()),
                },
                unknown: DisplayOptions {
                    display: unknown_display,
                    separator: unknown_separator,
                    status_display_type: unknown_status_display_type,
                    poster_source: unknown_poster,
                    show_paused: unknown_paused.unwrap_or(show_paused),
                    buttons: unknown_buttons.or(buttons.clone()),
                },
                blacklist: Blacklist {
                    media_types,
                    libraries,
                },
                show_simple: self.kodi.show_simple.unwrap_or(false),
                append_prefix: self.kodi.append_prefix.unwrap_or(false),
                add_divider: self.kodi.add_divider.unwrap_or(false),
            },
            discord: Discord { application_id },
            imgur: Imgur { client_id },
            images: Images {
                enable_images,
                imgur_images,
                litterbox_images,
                process_images,
                size: image_size,
                bg: image_bg,
                bg_blur: image_bg_blur,
                corner_radius: image_corner_radius,
            },
        }
    }
}
