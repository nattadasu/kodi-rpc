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

const UPLOAD_URL: &str = "https://tmpfiles.org/api/v1/upload";
/// Highest retention tmpfiles.org allows (172800s = 48h, the API max).
const EXPIRE_SECS: &str = "172800";
/// Cache entries die with the server-side file.
const TTL_HOURS: i64 = 48;

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

#[derive(Deserialize)]
struct TmpfilesResponse {
    status: serde_json::Value,
    data: Option<TmpfilesData>,
}

#[derive(Deserialize)]
struct TmpfilesData {
    url: String,
}

pub fn get_image(client: &Client) -> KodiResult<Url> {
    // Cache key includes the resolved art source: if the artwork behind an
    // item changes (e.g. episode still -> season poster), the stale upload
    // of the old art must not be served. Prefixed so a urls.json shared
    // with litterbox can never collide.
    let art_src = client
        .artwork_download_url()
        .map(|u| u.to_string())
        .unwrap_or_default();
    let key = format!(
        "tmpfiles::{}::{}",
        client.session.as_ref().unwrap().item_id,
        art_src
    );

    let mut image_urls = read_file(client)?;

    if let Some(idx) = image_urls.iter().position(|image_url| key == image_url.id) {
        let image_url = image_urls[idx].clone();

        // Sanity + expiry: tmpfiles links die server-side after EXPIRE_SECS,
        // and a corrupt cache entry must evict (not panic the loop or serve
        // garbage).
        let usable = Url::parse(&image_url.url).is_ok()
            && image_url
                .timestamp
                .parse::<i64>()
                .ok()
                .and_then(|t| DateTime::from_timestamp(t, 0))
                .map(|date| (Utc::now() - date).num_hours() < TTL_HOURS)
                .unwrap_or(false);

        if !usable {
            debug!("Cached tmpfiles image invalid or expired; evicting");

            image_urls.swap_remove(idx);

            let mut file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&client.tmpfiles_options.urls_location)?;

            file.write_all(serde_json::to_string(&image_urls)?.as_bytes())?;

            let _ = file.flush();
            // Try again
            self::get_image(client)
        } else {
            debug!("Found image url: \"{}\"", image_url.url);
            Ok(Url::parse(&image_url.url)?)
        }
    } else {
        debug!("No cached tmpfiles image found. Uploading new image");

        let current_time: DateTime<Utc> = Utc::now();

        let tmpfiles_url = upload(client)?;

        debug!("Tmpfiles response: {}", tmpfiles_url);

        let image_url = ImageUrl::new(
            &key,
            tmpfiles_url.as_str(),
            current_time.timestamp().to_string(),
        );

        image_urls.push(image_url);

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&client.tmpfiles_options.urls_location)?;

        file.write_all(serde_json::to_string(&image_urls)?.as_bytes())?;

        let _ = file.flush();

        Ok(tmpfiles_url)
    }
}

fn read_file(client: &Client) -> KodiResult<Vec<ImageUrl>> {
    if let Ok(contents_raw) = fs::read_to_string(&client.tmpfiles_options.urls_location) {
        if let Ok(contents) = serde_json::from_str::<Vec<ImageUrl>>(&contents_raw) {
            return Ok(contents);
        }
    }

    let path = Path::new(&client.tmpfiles_options.urls_location)
        .parent()
        .ok_or(Error::new(
            ErrorKind::Other,
            "Can't find parent folder of urls.json",
        ))?;

    fs::create_dir_all(path)?;

    let mut file = File::create(client.tmpfiles_options.urls_location.clone())?;

    let new: Vec<ImageUrl> = vec![];

    file.write_all(serde_json::to_string(&new)?.as_bytes())?;

    let _ = file.flush();

    Ok(new)
}

/// Legacy `/dl` prefix derivation; fresh uploads scrape the tokenized link
/// from the preview page instead (see [`resolve_direct_url`]).
fn direct_download_url(page_url: &str) -> KodiResult<Url> {
    let url = Url::parse(page_url.trim())?;
    if url.path().starts_with("/dl/") {
        return Ok(url);
    }
    let mut derived = url.clone();
    derived.set_path(&format!("/dl{}", url.path()));
    Ok(derived)
}

/// Scrape the tokenized direct link from a tmpfiles preview page.
fn extract_direct_url_from_page(page_html: &str) -> Option<Url> {
    const PREFIX: &str = "https://tmpfiles.org/dl/";
    let mut search = page_html;
    let mut best: Option<String> = None;
    while let Some(start) = search.find(PREFIX) {
        let rest = &search[start..];
        let end = rest
            .find(|c| {
                c == '"'
                    || c == '\''
                    || c == '<'
                    || c == '>'
                    || c == ' '
                    || c == '\n'
                    || c == '\r'
                    || c == '\t'
            })
            .unwrap_or(rest.len());
        let mut candidate = rest[..end].to_string();
        candidate = candidate.replace("&amp;", "&");
        // Tokenized links have `/dl/{token}/{id}/{file}` (3 segments).
        let segments: Vec<&str> = candidate
            .trim_start_matches(PREFIX)
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        if segments.len() >= 3 && Url::parse(&candidate).is_ok() {
            // Prefer image links; otherwise keep the first tokenized hit.
            if candidate.ends_with(".png")
                || candidate.ends_with(".jpg")
                || candidate.ends_with(".jpeg")
                || candidate.ends_with(".webp")
                || candidate.ends_with(".gif")
            {
                return Url::parse(&candidate).ok();
            }
            if best.is_none() {
                best = Some(candidate);
            }
        }
        search = &search[start + PREFIX.len()..];
    }
    best.and_then(|u| Url::parse(&u).ok())
}

/// Fetch the preview page and scrape the tokenized direct-download URL.
fn resolve_direct_url(http: &reqwest::blocking::Client, page_url: &Url) -> KodiResult<Url> {
    let html = super::upload_body("tmpfiles", http.get(page_url.clone()).send()?)?;
    extract_direct_url_from_page(&html).ok_or_else(|| {
        format!(
            "tmpfiles preview page has no direct link: {}",
            super::snippet(&html)
        )
        .into()
    })
}

/// Validate an upload response; returns the preview-page URL.
fn parse_upload_response(text: &str) -> KodiResult<Url> {
    let resp: TmpfilesResponse = serde_json::from_str(text)
        .map_err(|_| format!("tmpfiles bad response: {}", super::snippet(text)))?;
    let ok = resp.status == serde_json::Value::String("success".to_string())
        || resp.status == serde_json::Value::Bool(true);
    if !ok {
        return Err(format!("tmpfiles rejected upload: {}", super::snippet(text)).into());
    }
    match resp.data {
        Some(d) if !d.url.trim().is_empty() => Ok(Url::parse(d.url.trim())?),
        _ => Err(format!("tmpfiles bad response: {}", super::snippet(text)).into()),
    }
}

fn upload(client: &Client) -> KodiResult<Url> {
    // Download through the owning instance (works for localhost + image://);
    // the uploaded tmpfiles URL is what Discord actually sees.
    let image_bytes = client.download_artwork()?;

    debug!("Uploading image to tmpfiles.org");

    let http = reqwest::blocking::Client::builder().build()?;

    let file_bytes = if client.process_images {
        use crate::external::image_utils::make_square_with_blur;
        make_square_with_blur(&image_bytes, &client.image_processing_options)?
    } else {
        image_bytes.to_vec()
    };
    let ext = if client.process_images { "png" } else { "jpg" };

    let form = Form::new().text("expire", EXPIRE_SECS).part(
        "file",
        Part::bytes(file_bytes.clone()).file_name(format!("{}.{}", Utc::now().timestamp(), ext)),
    );

    let text = super::upload_body("tmpfiles", http.post(UPLOAD_URL).multipart(form).send()?)?;

    debug!("Response from tmpfiles: \"{}\"", super::snippet(&text));

    let page_url = parse_upload_response(&text)?;
    let direct = resolve_direct_url(&http, &page_url)?;

    // Binary-identical guarantee: fetch the served bytes back and compare
    // with what was sent. A transcode/proxy in the middle fails the upload
    // instead of silently serving different bytes to Discord.
    let verify = http.get(direct.clone()).send()?;
    if !verify.status().is_success() {
        return Err(format!("tmpfiles verification failed (HTTP {})", verify.status()).into());
    }
    let served = verify.bytes()?.to_vec();
    // Reject HTML preview pages served instead of the file bytes.
    let looks_like_html = served.len() > 15
        && served
            .iter()
            .take(200)
            .all(|b| b.is_ascii_graphic() || b.is_ascii_whitespace())
        && String::from_utf8_lossy(&served[..served.len().min(200)])
            .to_lowercase()
            .contains("<html");
    if looks_like_html || served != file_bytes {
        return Err("tmpfiles verification failed (served bytes differ from upload)".into());
    }

    Ok(direct)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_link_derivation() {
        assert_eq!(
            direct_download_url("https://tmpfiles.org/12345/art.png")
                .unwrap()
                .as_str(),
            "https://tmpfiles.org/dl/12345/art.png"
        );
        // Already-direct links pass through untouched.
        assert_eq!(
            direct_download_url("https://tmpfiles.org/dl/12345/art.png")
                .unwrap()
                .as_str(),
            "https://tmpfiles.org/dl/12345/art.png"
        );
        assert!(direct_download_url("not a url").is_err());
        assert!(direct_download_url("").is_err());
    }

    #[test]
    fn response_parsing() {
        // Uploads yield the preview page; the direct link is scraped from it.
        let ok = r#"{"status":"success","data":{"url":"https://tmpfiles.org/12345/art.png"}}"#;
        assert_eq!(
            parse_upload_response(ok).unwrap().as_str(),
            "https://tmpfiles.org/12345/art.png"
        );
        // Boolean success shape tolerated too.
        let ok_bool = r#"{"status":true,"data":{"url":"https://tmpfiles.org/dl/7/x.jpg"}}"#;
        assert_eq!(
            parse_upload_response(ok_bool).unwrap().as_str(),
            "https://tmpfiles.org/dl/7/x.jpg"
        );
        // Error status, garbage body, and missing URL all fail.
        assert!(parse_upload_response(r#"{"status":"error","data":null}"#).is_err());
        assert!(parse_upload_response("<!doctype html>502").is_err());
        assert!(parse_upload_response(r#"{"status":"success","data":{}}"#).is_err());
        assert!(parse_upload_response(r#"{"status":"success"}"#).is_err());
    }

    #[test]
    fn preview_page_scraping() {
        let html = r#"<img id="img_preview" src="https://tmpfiles.org/dl/1791260874.9a53a67ba5330b25/w9Agl96wYITs/1791260564.jpg"/><p><a class="download" href="https://tmpfiles.org/dl/1791260874.9a53a67ba5330b25/w9Agl96wYITs/1791260564.jpg">Download</a></p>"#;
        assert_eq!(
            extract_direct_url_from_page(html).unwrap().as_str(),
            "https://tmpfiles.org/dl/1791260874.9a53a67ba5330b25/w9Agl96wYITs/1791260564.jpg"
        );
        // Non-tokenized preview links don't count.
        assert!(extract_direct_url_from_page("<html>no links</html>").is_none());
        assert!(extract_direct_url_from_page("").is_none());
    }
}
