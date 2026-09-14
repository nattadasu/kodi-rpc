use clap::Parser;
use colored::Colorize;
use config::{get_config_path, get_urls_path, Config};
use kodi_rpc::{
    version_string, Client, DisplayFormat, EpisodeDisplayOptions, InstanceCfg, VERSION,
};
use log::{debug, error, info};
use retry::retry_with_index;
use simple_logger::SimpleLogger;
use std::{thread::sleep, time::Duration};
use time::macros::format_description;
mod config;
#[cfg(feature = "updates")]
mod updates;

/*
    TODO: Comments
*/

#[derive(Parser)]
#[command(author = "kodi-rpc contributors")]
#[command(version)]
#[command(about = "Rich presence for Kodi", long_about = None)]
struct Args {
    #[arg(short = 'c', long = "config", help = "Path to the config file")]
    config: Option<String>,
    #[arg(
        short = 'i',
        long = "image-urls-file",
        help = "Path to image urls file for imgur"
    )]
    image_urls: Option<String>,
    #[arg(
        short = 't',
        long = "wait-time",
        help = "Time to wait between loops in seconds",
        default_value_t = 7
    )]
    wait_time: usize,
    #[arg(
        short = 'v',
        long = "log-level",
        help = "Sets the log level to one of: trace, debug, info, warn, error, off",
        default_value_t = String::from("info")
    )]
    log_level: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", args.log_level);
    }

    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .env()
        .with_timestamp_format(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        ))
        .init()
        .unwrap();

    info!("Initializing Kodi-RPC v{}", version_string());

    #[cfg(feature = "updates")]
    updates::checker();

    let conf_path = &args
        .config
        .unwrap_or(get_config_path().expect("default config path couldn't be determined"));

    let conf = match Config::builder().load(conf_path) {
        Ok(file) => file.build(),
        Err(error) => {
            error!(
                "Config file could not be loaded at path: {}",
                conf_path.red()
            );
            error!("{}", error);
            error!(
                "Please create a proper config file, see {}",
                "example.json".green()
            );
            std::process::exit(1)
        }
    };

    debug!("Creating kodi-rpc client builder");
    let mut builder = Client::builder();

    let instances: Vec<InstanceCfg> = conf
        .kodi
        .instances
        .into_iter()
        .map(|i| InstanceCfg {
            url: i.url,
            username: i.username,
            password: i.password,
            self_signed_cert: i.self_signed_cert,
            name: i.name,
        })
        .collect();
    debug!("Watching {} Kodi instance(s)", instances.len());
    builder.instances(instances);

    builder
        .episode_simple(conf.kodi.show_simple)
        .episode_divider(conf.kodi.add_divider)
        .episode_prefix(conf.kodi.append_prefix)
        .show_paused(conf.discord.show_paused)
        .show_images(conf.images.enable_images)
        .use_imgur(conf.images.imgur_images)
        .use_litterbox(conf.images.litterbox_images)
        .process_images(conf.images.process_images)
        .image_size(conf.images.size)
        .image_background(conf.images.bg)
        .image_background_blur(conf.images.bg_blur)
        .image_corner_radius(conf.images.corner_radius)
        .large_image_text(format!("Kodi-RPC v{}", version_string()))
        .imgur_urls_file_location(args.image_urls.clone().unwrap_or(get_urls_path()?))
        .litterbox_urls_file_location(args.image_urls.unwrap_or(get_urls_path()?));

    if let Some(display) = conf.kodi.music.display {
        debug!("Found config.kodi.music.display");
        builder.music_display(display);
    }

    if let Some(separator) = conf.kodi.music.separator {
        debug!("Found config.kodi.music.separator");
        builder.music_separator(separator);
    }

    if let Some(status_display_type) = conf.kodi.music.status_display_type {
        debug!("Found config.kodi.music.status_display_type");
        builder.music_status_display_type(status_display_type);
    }

    if let Some(display) = conf.kodi.movies.display {
        debug!("Found config.kodi.movies.display");
        builder.movies_display(display);
    }

    if let Some(separator) = conf.kodi.movies.separator {
        debug!("Found config.kodi.movies.separator");
        builder.movies_separator(separator);
    }

    if let Some(status_display_type) = conf.kodi.movies.status_display_type {
        debug!("Found config.kodi.movies.status_display_type");
        builder.movies_status_display_type(status_display_type);
    }

    if let Some(display) = conf.kodi.episodes.display {
        debug!("Found config.kodi.episodes.display");
        builder.episodes_display(display);
    } else {
        debug!("Couldn't find config.kodi.episodes.display, using legacy episode values");
        builder.episodes_display(DisplayFormat::from(EpisodeDisplayOptions {
            divider: conf.kodi.add_divider,
            prefix: conf.kodi.append_prefix,
            simple: conf.kodi.show_simple,
        }));
    }

    if let Some(separator) = conf.kodi.episodes.separator {
        debug!("Found config.kodi.episodes.separator");
        builder.episodes_separator(separator);
    }

    if let Some(status_display_type) = conf.kodi.episodes.status_display_type {
        debug!("Found config.kodi.episodes.status_display_type");
        builder.episodes_status_display_type(status_display_type);
    }

    builder.episodes_poster_source(conf.kodi.episodes.poster_source);

    if let Some(display) = conf.kodi.unknown.display {
        debug!("Found config.kodi.unknown.display");
        builder.unknown_display(display);
    }

    if let Some(separator) = conf.kodi.unknown.separator {
        debug!("Found config.kodi.unknown.separator");
        builder.unknown_separator(separator);
    }

    if let Some(status_display_type) = conf.kodi.unknown.status_display_type {
        debug!("Found config.kodi.unknown.status_display_type");
        builder.unknown_status_display_type(status_display_type);
    }

    if let Some(media_types) = conf.kodi.blacklist.media_types {
        debug!("Found config.kodi.blacklist.media_types");
        debug!("Blacklisted MediaTypes: {:?}", media_types);
        builder.blacklist_media_types(media_types);
    }

    if let Some(libraries) = conf.kodi.blacklist.libraries {
        debug!("Found config.kodi.blacklist.libraries");
        debug!("Blacklisted libraries: {:?}", libraries);
        builder.blacklist_libraries(libraries);
    }

    if let Some(application_id) = conf.discord.application_id {
        debug!("Found config.discord.application_id");
        builder.client_id(application_id);
    }

    if let Some(buttons) = conf.discord.buttons {
        debug!("Found config.discord.buttons");
        builder.buttons(buttons);
    }

    if let Some(client_id) = conf.imgur.client_id {
        debug!("Found config.imgur.client_id");
        builder.imgur_client_id(client_id);
    }

    debug!("Building client");
    let mut client = builder.build()?;

    info!("Connecting to Discord");
    retry_with_index(
        retry::delay::Exponential::from_millis(1000),
        |current_try| {
            info!("Attempt {}: Trying to connect", current_try);
            match client.connect() {
                Ok(_) => retry::OperationResult::Ok(()),
                Err(err) => {
                    error!("{}", err);
                    retry::OperationResult::Retry(())
                }
            }
        },
    )
    .unwrap();
    info!("Connected!");

    let mut currently_playing = String::new();

    loop {
        sleep(Duration::from_secs(args.wait_time as u64));

        match client.set_activity() {
            Ok(activity) => {
                if activity.is_empty() && !currently_playing.is_empty() {
                    let _ = client.clear_activity();
                    info!("Cleared activity");
                    currently_playing = activity;
                } else if activity != currently_playing {
                    currently_playing = activity;

                    info!("{}", currently_playing);
                }
            }
            Err(err) => {
                // TODO: There has to be a better way to check this..
                if err.to_string() == "content is blacklisted"
                    || err.to_string() == "nothing is playing"
                {
                    debug!("{}", err);
                    if !currently_playing.is_empty() {
                        let _ = client.clear_activity();
                        info!("Cleared activity");
                        currently_playing = String::new();
                    }
                    continue;
                }

                error!("{}", err);
                debug!("{:?}", err);
                retry_with_index(
                    retry::delay::Exponential::from_millis(1000),
                    |current_try| {
                        info!("Attempt {}: Trying to reconnect", current_try);
                        match client.reconnect() {
                            Ok(_) => retry::OperationResult::Ok(()),
                            Err(err) => {
                                error!("{}", err);
                                retry::OperationResult::Retry(())
                            }
                        }
                    },
                )
                .unwrap();
                info!("Reconnected!");
            }
        }
    }
}
