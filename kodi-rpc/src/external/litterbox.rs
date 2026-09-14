use std::{
    fs::{self, File, OpenOptions},
    io::{Error, ErrorKind, Write},
    path::Path,
};

use chrono::prelude::*;
use log::debug;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{Client, KodiResult};
use reqwest::blocking::multipart::{Form, Part};

#[derive(Deserialize, Serialize, Clone)]
struct ImageUrl {
    id: String,
    url: String,
    timestamp: String,
}

impl ImageUrl {
    fn new<T: Into<String>, Y: Into<String>, U: Into<String>>(id: T, url: Y, timestamp: U) -> Self {
        Self {
            id: id.into(),
            url: url.into(),
            timestamp: timestamp.into(),
        }
    }
}

pub fn get_image(client: &Client) -> KodiResult<Url> {
    // Cache key includes the resolved art source: if the artwork behind an
    // item changes (e.g. episode still -> season poster), the stale upload
    // of the old art must not be served.
    let art_src = client
        .artwork_download_url()
        .map(|u| u.to_string())
        .unwrap_or_default();
    let key = format!("{}::{}", client.session.as_ref().unwrap().item_id, art_src);

    let mut image_urls = read_file(client)?;

    if let Some(idx) = image_urls.iter().position(|image_url| key == image_url.id) {
        let image_url = image_urls[idx].clone();

        // Sanity + expiry: litterbox links die server-side after 72h, and a
        // corrupt cache entry must evict (not panic the loop or serve garbage).
        let usable = Url::parse(&image_url.url).is_ok()
            && image_url
                .timestamp
                .parse::<i64>()
                .ok()
                .and_then(|t| DateTime::from_timestamp(t, 0))
                .map(|date| (Utc::now() - date).num_hours() < 72)
                .unwrap_or(false);

        if !usable {
            debug!("Cached litterbox image invalid or expired; evicting");

            image_urls.swap_remove(idx);

            let mut file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&client.litterbox_options.urls_location)?;

            file.write_all(serde_json::to_string(&image_urls)?.as_bytes())?;

            let _ = file.flush();
            // Try again
            self::get_image(client)
        } else {
            debug!("Found image url: \"{}\"", image_url.url);
            Ok(Url::parse(&image_url.url)?)
        }
    } else {
        debug!("No cached litterbox image found. Uploading new image");

        let current_time: DateTime<Utc> = Utc::now();

        let litterbox_url = upload(client)?;

        debug!("Litterbox response: {}", litterbox_url);

        let image_url = ImageUrl::new(
            &key,
            litterbox_url.as_str(),
            current_time.timestamp().to_string(),
        );

        image_urls.push(image_url);

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&client.litterbox_options.urls_location)?;

        file.write_all(serde_json::to_string(&image_urls)?.as_bytes())?;

        let _ = file.flush();

        Ok(litterbox_url)
    }
}

fn read_file(client: &Client) -> KodiResult<Vec<ImageUrl>> {
    if let Ok(contents_raw) = fs::read_to_string(&client.litterbox_options.urls_location) {
        if let Ok(contents) = serde_json::from_str::<Vec<ImageUrl>>(&contents_raw) {
            return Ok(contents);
        }
    }

    let path = Path::new(&client.litterbox_options.urls_location)
        .parent()
        .ok_or(Error::new(
            ErrorKind::Other,
            "Can't find parent folder of urls.json",
        ))?;

    fs::create_dir_all(path)?;

    let mut file = File::create(client.litterbox_options.urls_location.clone())?;

    let new: Vec<ImageUrl> = vec![];

    file.write_all(serde_json::to_string(&new)?.as_bytes())?;

    let _ = file.flush();

    Ok(new)
}

fn upload(client: &Client) -> KodiResult<Url> {
    // Download through the owning instance (works for localhost + image://);
    // the uploaded litterbox URL is what Discord actually sees.
    let image_bytes = client.download_artwork()?;

    debug!("Uploading image to litterbox");

    let litterbox_client = reqwest::blocking::Client::builder().build()?;
    let filename = Utc::now().to_string();

    let file_bytes = if client.process_images {
        use crate::external::image_utils::make_square_with_blur;
        make_square_with_blur(&image_bytes, &client.image_processing_options)?
    } else {
        image_bytes.to_vec()
    };

    let litterbox_form = Form::new()
        .text("reqtype", "fileupload")
        .text("time", "72h")
        .part(
            "fileToUpload",
            Part::bytes(file_bytes).file_name(filename + ".jpg"),
        );

    let res: String = litterbox_client
        .post("https://litterbox.catbox.moe/resources/internals/api.php")
        .multipart(litterbox_form)
        .send()?
        .text()?;

    debug!("Response from Litterbox: \"{}\"", res.clone());

    Ok(Url::parse(&res)?)
}
