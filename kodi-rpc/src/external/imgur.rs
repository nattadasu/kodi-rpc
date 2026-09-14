use std::{
    fs::{self, File, OpenOptions},
    io::{Error, ErrorKind, Write},
    path::Path,
};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::{Client, KodiResult};

#[derive(Deserialize, Serialize)]
struct ImageUrl {
    id: String,
    url: String,
}

impl ImageUrl {
    fn new<T: Into<String>, Y: Into<String>>(id: T, url: Y) -> Self {
        Self {
            id: id.into(),
            url: url.into(),
        }
    }
}

#[derive(Deserialize)]
struct ImgurResponse {
    data: Data,
}

#[derive(Deserialize)]
struct Data {
    link: String,
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

    if let Some(pos) = image_urls.iter().position(|image_url| key == image_url.id) {
        // Sanity: a corrupt entry evicts and re-uploads instead of erroring
        // every poll (imgur links themselves don't expire).
        match Url::parse(&image_urls[pos].url) {
            Ok(url) => Ok(url),
            Err(_) => {
                image_urls.swap_remove(pos);
                let mut file = OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(&client.imgur_options.urls_location)?;
                file.write_all(serde_json::to_string(&image_urls)?.as_bytes())?;
                let _ = file.flush();
                self::get_image(client)
            }
        }
    } else {
        let imgur_url = upload(client)?;

        let image_url = ImageUrl::new(&key, imgur_url.as_str());

        image_urls.push(image_url);

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&client.imgur_options.urls_location)?;

        file.write_all(serde_json::to_string(&image_urls)?.as_bytes())?;

        let _ = file.flush();

        Ok(imgur_url)
    }
}

fn read_file(client: &Client) -> KodiResult<Vec<ImageUrl>> {
    if let Ok(contents_raw) = fs::read_to_string(&client.imgur_options.urls_location) {
        if let Ok(contents) = serde_json::from_str::<Vec<ImageUrl>>(&contents_raw) {
            return Ok(contents);
        }
    }

    let path = Path::new(&client.imgur_options.urls_location)
        .parent()
        .ok_or(Error::new(
            ErrorKind::Other,
            "Can't find parent folder of urls.json",
        ))?;

    fs::create_dir_all(path)?;

    let mut file = File::create(client.imgur_options.urls_location.clone())?;

    let new: Vec<ImageUrl> = vec![];

    file.write_all(serde_json::to_string(&new)?.as_bytes())?;

    let _ = file.flush();

    Ok(new)
}

fn upload(client: &Client) -> KodiResult<Url> {
    // Download through the owning instance (works for localhost + image://);
    // the uploaded imgur URL is what Discord actually sees.
    let image_bytes = client.download_artwork()?;

    let imgur_client = reqwest::blocking::Client::builder().build()?;

    let body = if client.process_images {
        use crate::external::image_utils::make_square_with_blur;
        make_square_with_blur(&image_bytes, &client.image_processing_options)?
    } else {
        image_bytes.to_vec()
    };

    let res: ImgurResponse = imgur_client
        .post("https://api.imgur.com/3/image")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Client-ID {}", client.imgur_options.client_id),
        )
        .body(body)
        .send()?
        .json()?;

    Ok(Url::parse(res.data.link.as_str())?)
}
