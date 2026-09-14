use crate::kodi::{KodiItem, NowPlayingItem, PlayState, PlayerGetPropertiesResult, Session};
use crate::{Button, Client, ClientBuilder, InstanceCfg, PosterSource};

#[test]
fn owner_election_first_plays_wins() {
    // Nobody playing -> nobody owns.
    assert_eq!(Client::elect_owner(None, &[]), None);
    assert_eq!(Client::elect_owner(None, &[false, false]), None);
    // Idle owner + one playing -> the playing one wins.
    assert_eq!(Client::elect_owner(None, &[false, true]), Some(1));
    // Owner still playing -> keeps presence even if others play too.
    assert_eq!(Client::elect_owner(Some(1), &[true, true]), Some(1));
    assert_eq!(Client::elect_owner(Some(0), &[true, true]), Some(0));
    // Owner stopped -> first playing instance (config order) takes over.
    assert_eq!(Client::elect_owner(Some(1), &[true, false]), Some(0));
    assert_eq!(Client::elect_owner(Some(0), &[false, true]), Some(1));
    // Owner gone entirely -> first playing wins.
    assert_eq!(Client::elect_owner(None, &[true, true]), Some(0));
    // Stale owner index (fewer instances than before) -> first playing.
    assert_eq!(Client::elect_owner(Some(5), &[true, false]), Some(0));
}

#[test]
fn multi_instance_build() {
    let mut b = ClientBuilder::new();
    b.url("http://localhost:8080").instances(vec![InstanceCfg {
        url: "http://127.0.0.1:8080".to_string(),
        name: "second".to_string(),
        ..Default::default()
    }]);
    b.build().expect("primary + extra instance builds");
}

#[test]
fn instances_only_build() {
    let mut b = ClientBuilder::new();
    b.instances(vec![InstanceCfg {
        url: "http://localhost:8080".to_string(),
        ..Default::default()
    }]);
    b.build().expect("instances-only build works");
}

#[test]
fn poster_source_parsing() {
    assert_eq!(
        PosterSource::from("series".to_string()),
        PosterSource::Series
    );
    assert_eq!(
        PosterSource::from("EPISODE".to_string()),
        PosterSource::Episode
    );
    assert_eq!(
        PosterSource::from("bogus".to_string()),
        PosterSource::Season
    );
    assert_eq!(PosterSource::default(), PosterSource::Season);
}

/// Live test against a real Kodi instance. Only runs when explicitly asked:
/// `KODI_TEST_URL=... KODI_TEST_USER=... KODI_TEST_PASS=... cargo test -p kodi-rpc -- --ignored --nocapture live_`
/// Credentials stay in env vars so they never end up in the repo.
#[test]
#[ignore]
fn live_kodi_session_parses() {
    let url = std::env::var("KODI_TEST_URL").expect("KODI_TEST_URL must be set");
    let user = std::env::var("KODI_TEST_USER").unwrap_or_default();
    let pass = std::env::var("KODI_TEST_PASS").unwrap_or_default();

    let mut builder = ClientBuilder::new();
    builder.url(url).username(user).password(pass);
    let mut client = builder.build().expect("client builds");

    client.get_session().expect("get_session works");

    let session = client
        .session
        .as_ref()
        .expect("expected something playing on the test Kodi");
    println!("media_type : {}", session.now_playing_item.media_type);
    println!("name       : {}", session.now_playing_item.name);
    println!("file       : {}", session.now_playing_item.file);
    println!("paused     : {}", session.play_state.is_paused);
    println!(
        "poster     : {}",
        session
            .now_playing_item
            .poster
            .as_deref()
            .unwrap_or("<none>")
    );
    println!("details    : {}", client.get_details());
    println!("state      : {}", client.get_state());
    println!("image_text : {}", client.get_image_text());
    // Localhost Kodi art must NOT be handed to Discord (unfetchable) —
    // expect the fallback-icon path here.
    match client.get_image() {
        Ok(u) => println!("image_url  : {}", u),
        Err(_) => println!("image_url  : (fallback icon — local art isn't Discord-fetchable)"),
    }
    match session.get_time().expect("get_time works") {
        crate::kodi::PlayTime::Some(start, end) => {
            println!("timestamps : start={} end={}", start, end);
            assert!(end > start, "end must be after start");
        }
        crate::kodi::PlayTime::Paused => println!("timestamps : paused"),
        crate::kodi::PlayTime::None => println!("timestamps : none"),
    }
    assert!(
        !client.get_details().is_empty(),
        "details must not be empty for a playing item"
    );
}

#[test]
fn build_client_error() {
    let client = ClientBuilder::new().build();

    if client.is_ok() {
        panic!("client was constructed even though required values are missing!");
    }
}

fn fake_props() -> PlayerGetPropertiesResult {
    serde_json::from_str(
        r#"{"speed":1,
            "time":{"hours":0,"minutes":1,"seconds":0,"milliseconds":0},
            "totaltime":{"hours":0,"minutes":24,"seconds":0,"milliseconds":0}}"#,
    )
    .unwrap()
}

fn session_with_thumb(thumb: Option<&str>, fanart: Option<&str>) -> Session {
    let item = KodiItem {
        label: "Some Title".to_string(),
        item_type: "movie".to_string(),
        file: Some("/videos/movie.mkv".to_string()),
        thumbnail: thumb.map(str::to_string),
        fanart: fanart.map(str::to_string),
        ..Default::default()
    };
    let props = fake_props();
    let npi = NowPlayingItem::from_kodi(&item, &props, "video").expect("fake item must parse");
    Session {
        item_id: npi.id.clone(),
        play_state: PlayState::from_props(&props),
        now_playing_item: npi,
        source: 0,
    }
}

fn client_for(url: &str, user: &str, pass: &str) -> Client {
    let mut b = ClientBuilder::new();
    b.url(url).username(user).password(pass);
    b.build().expect("client builds")
}

/// Regression test: `http://localhost:8080/image/...` URLs killed presence
/// rendering, so local `image://` art must fall back to the default icon.
#[test]
fn localhost_art_falls_back_to_default_icon() {
    let mut c = client_for("http://localhost:8080", "kodi", "kodi");
    c.session = Some(session_with_thumb(Some("image://video@foo/"), None));
    assert!(
        c.get_image().is_err(),
        "localhost /image/ URLs must never reach Discord"
    );
}

/// Plugin/CDN hotlinks are publicly fetchable, so they pass straight through.
#[test]
fn plugin_hotlink_art_passes_through() {
    let mut c = client_for("http://localhost:8080", "kodi", "kodi");
    c.session = Some(session_with_thumb(
        None,
        Some("https://image.tmdb.org/t/p/original/abc.jpg"),
    ));
    assert_eq!(
        c.get_image().unwrap().as_str(),
        "https://image.tmdb.org/t/p/original/abc.jpg"
    );
}

/// Series/season/movie posters outrank the episode still for display.
#[test]
fn poster_outranks_episode_still_for_display() {
    let mut c = client_for("http://localhost:8080", "kodi", "kodi");
    let mut s = session_with_thumb(Some("image://thumb/"), None);
    s.now_playing_item.poster = Some("https://image.tmdb.org/t/p/original/poster.jpg".to_string());
    c.session = Some(s);
    assert_eq!(
        c.get_image().unwrap().as_str(),
        "https://image.tmdb.org/t/p/original/poster.jpg"
    );
}

/// ...and for uploads: the raw resolver must also prefer the poster.
#[test]
fn poster_outranks_episode_still_for_upload() {
    let mut c = client_for("http://localhost:8080", "kodi", "kodi");
    let mut s = session_with_thumb(Some("image://thumb/"), None);
    s.now_playing_item.poster = Some("image://poster.jpg/".to_string());
    c.session = Some(s);
    let u = c.artwork_download_url().unwrap();
    assert!(
        u.as_str().contains("poster.jpg"),
        "poster must win, got {}",
        u
    );
}

/// `image://`-wrapped CDN art (Crunchyroll-style) needs no upload round-trip.
#[test]
fn wrapped_cdn_art_needs_no_upload() {
    let mut c = client_for("http://localhost:8080", "kodi", "kodi");
    c.session = Some(session_with_thumb(
        Some("image://https%3a%2f%2fwww.crunchyroll.com%2fx%2fy.png/"),
        None,
    ));
    assert_eq!(
        c.get_image().unwrap().as_str(),
        "https://www.crunchyroll.com/x/y.png"
    );
}

/// Proxy `file` URLs display the proxied host, not the loopback address.
#[test]
fn proxy_file_shows_proxied_host() {
    let item = KodiItem {
        label: "Show - S01E07 - Title".to_string(),
        item_type: "unknown".to_string(),
        file: Some(
            "http://127.0.0.1:49543/proxy?url=https%3A%2F%2Fwww.crunchyroll.com%2Fmanifest"
                .to_string(),
        ),
        ..Default::default()
    };
    let npi = NowPlayingItem::from_kodi(&item, &fake_props(), "video").expect("parses");
    assert_eq!(npi.media_type, crate::MediaType::Unknown);
    assert_eq!(npi.file_host_display(), "www.crunchyroll.com");
    // Playback file URLs never become buttons; dynamics come from
    // series/movie library info instead.
    assert!(npi.external_urls.is_none());
}

/// A publicly reachable Kodi *without* auth can serve art to Discord's proxy.
#[test]
fn public_noauth_kodi_may_use_direct_art() {
    let mut c = client_for("https://kodi.example.com", "", "");
    c.session = Some(session_with_thumb(Some("image://video@foo/"), None));
    let u = c
        .get_image()
        .expect("public no-auth Kodi art is Discord-fetchable");
    assert!(u.as_str().starts_with("https://kodi.example.com/image/"));
}

/// Same public host *with* auth: Discord would get a 401, so fall back.
#[test]
fn public_kodi_with_auth_falls_back() {
    let mut c = client_for("https://kodi.example.com", "kodi", "secret");
    c.session = Some(session_with_thumb(Some("image://video@foo/"), None));
    assert!(c.get_image().is_err());
}

#[test]
fn invalid_url() {
    let mut builder = ClientBuilder::new();
    builder.url("url_without_base.com");

    let client = builder.build();

    if client.is_ok() {
        panic!("client constructed without a valid url!")
    }
}

#[test]
fn valid_url_with_auth() {
    let mut builder = ClientBuilder::new();
    builder
        .url("http://localhost:8080")
        .username("kodi")
        .password("kodi");

    builder
        .build()
        .expect("client should build with url + auth");
}

#[test]
fn https_reverse_proxy_url() {
    let mut builder = ClientBuilder::new();
    builder.url("https://kodi.example.com");

    let client = builder.build().expect("https url should build");
    drop(client);
}

#[test]
fn overlong_button_urls_dropped() {
    let mut b = ClientBuilder::new();
    b.url("http://localhost:8080");
    b.episodes_buttons(vec![
        Button::new("ok".to_string(), "https://x.co/".to_string()),
        Button::new(
            "long".to_string(),
            format!("https://x.co/{}", "y".repeat(600)),
        ),
    ]);
    let mut c = b.build().unwrap();
    let mut s = session_with_thumb(None, None);
    s.now_playing_item.media_type = crate::MediaType::Episode;
    c.session = Some(s);
    let btns = c.get_buttons().unwrap();
    assert_eq!(btns.len(), 1);
    assert_eq!(btns[0].name, "ok");
}

#[test]
fn per_section_show_paused() {
    let mut b = ClientBuilder::new();
    b.url("http://localhost:8080").episodes_show_paused(false);
    let c = b.build().unwrap();
    assert!(!c.episodes_display_options.show_paused);
    assert!(c.music_display_options.show_paused);
    assert!(c.movies_display_options.show_paused);
    assert!(c.unknown_display_options.show_paused);
}

#[test]
fn per_section_buttons() {
    let mut b = ClientBuilder::new();
    b.url("http://localhost:8080")
        .movies_buttons(vec![Button::new(
            "site".to_string(),
            "https://x.co/".to_string(),
        )]);
    let c = b.build().unwrap();
    assert_eq!(c.movies_display_options.buttons.unwrap().len(), 1);
    assert!(c.music_display_options.buttons.is_none());
    assert!(c.episodes_display_options.buttons.is_none());
    assert!(c.unknown_display_options.buttons.is_none());
}

#[test]
fn is_paused_reflects_session() {
    let mut c = client_for("http://localhost:8080", "kodi", "kodi");
    assert!(!c.is_paused());
    c.session = Some(session_with_thumb(None, None));
    assert!(!c.is_paused());
    let mut s = session_with_thumb(None, None);
    s.play_state = crate::kodi::PlayState {
        is_paused: true,
        position_ticks: None,
    };
    c.session = Some(s);
    assert!(c.is_paused());
}
