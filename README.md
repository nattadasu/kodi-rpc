# Kodi-RPC

Program used to display what you're currently watching on Kodi in Discord.

Hard clone of [jellyfin-rpc](https://github.com/justradical/jellyfin-rpc) by
JustRadical, retargeted at the **Kodi JSON-RPC API**
(`Player.GetActivePlayers` → `Player.GetItem` → `Player.GetProperties`).
The Discord presence layer (buttons, display templates, blacklist,
imgur/litterbox artwork, paused icon, timestamps) is intentionally kept 1:1
so jellyfin-rpc configs and habits carry over.

The only requirement is that Discord is open and logged in; kodi-rpc can run
on the Kodi box itself or any machine that can reach Kodi over HTTP.

## Kodi setup

1. In Kodi: **Settings → Services → Control**
   - Enable **Allow remote control via HTTP**
   - Set **Port** to `8080` (default)
   - Set a **Username** / **Password** (e.g. `kodi` / `kodi`)
   - If you expose Kodi via reverse proxy (e.g. `https://kodi.example.com`),
     make sure `/jsonrpc` and `/image/*` are forwarded.
2. Test it:

   ```bash
   curl -u kodi:kodi -X POST -H 'Content-Type: application/json' \
     -d '{"jsonrpc":"2.0","method":"Player.GetActivePlayers","id":1}' \
     http://localhost:8080/jsonrpc
   ```

## Install

```bash
cargo build --release
# binary: ./target/release/kodi-rpc
```

or with [go-task](https://taskfile.dev) (see `Taskfile.yml` for all tasks):

```bash
task build
```

Untagged builds stamp the commit into the version (`v0.1.0+a1b2c3d`, shown
at startup and in the presence tooltip) and CI uploads them as
`kodi-rpc-<sha>-<platform>` artifacts. Tagged builds (`vX.Y.Z`, or
`vX.Y.Z-beta.N` for prereleases) report the clean version and publish
stable-named binaries to the GitHub release — the names `installer.py`
downloads.

Alternatively, grab a release binary and run the installer:

```bash
python3 scripts/installer.py
```

## Setup

Copy `example.json` to your config location and edit it (full option
reference: [docs/configuration.md](docs/configuration.md)):

- Linux/macOS: `~/.config/kodi-rpc/main.json`
- Windows: `%appdata%\kodi-rpc\main.json`

```bash
mkdir -p ~/.config/kodi-rpc
cp example.json ~/.config/kodi-rpc/main.json
$EDITOR ~/.config/kodi-rpc/main.json
```

Minimal config (local Kodi, no auth):

```json
{
  "kodi": {
    "url": "http://localhost:8080",
    "username": "",
    "password": ""
  },
  "discord": { "show_paused": true },
  "images": { "enable_images": true }
}
```

Reverse-proxy Kodi with auth:

```json
{
  "kodi": {
    "url": "https://kodi.example.com",
    "username": "kodi",
    "password": "changeme",
    "self_signed_cert": false
  }
}
```

Then run:

```bash
kodi-rpc -c ~/.config/kodi-rpc/main.json
```

> Tip: if kodi-rpc runs on the Kodi box itself, prefer
> `http://localhost:8080` over the public URL — same data, no public
> round-trip, and polling keeps working if the tunnel drops.

## Usage

CLI flags:

```
-c, --config <path>               Path to the config file
-i, --image-urls-file <path>      Path to image urls cache (imgur/litterbox)
-t, --wait-time <secs>            Poll interval [default: 7]
-v, --log-level <level>           trace|debug|info|warn|error|off [default: info]
```

Legacy `{"jellyfin": {...}}` configs are auto-mapped to `{"kodi": {...}}`
(`url` is reused; set `username`/`password` for Kodi HTTP auth).

## Multiple Kodi instances

Add an `instances` array to the `kodi` block (each entry takes `url`,
`username`, `password`, `self_signed_cert`, `name`). Every instance is
polled each cycle and **whichever plays first owns the presence until it
stops** — a second playback never steals it, and when the owner goes idle
the first playing instance in config order takes over. Dead instances are
skipped without affecting the rest. When `instances` is non-empty, the
top-level `url`/`username`/`password` are ignored. Display templates,
blacklist, and image settings stay global.

## 3rd-party plugin playback (the whole point of this fork)

Kodi reports most addon streams as:

```json
{"type": "unknown", "label": "Some Title [COLOR aqua]º[/COLOR]",
 "file": "plugin://plugin.video.foo/play?..."}
```

and on repeat plays even library-mapped fields (`title`, `season`, …) go
blank (upstream Kodi issue [#16245](https://github.com/xbmc/xbmc/issues/16245)
— `Player.GetItem` falls back to the DB row which has no infolabels).

kodi-rpc handles this explicitly:

- `MediaType::Unknown` is **displayable** (only `None`/empty is hidden), so
  `plugin://` streams never vanish from Discord.
- Title resolution: `title` → cleaned `label` → file name — something always
  shows.
- `strip_kodi_tags()` removes `[COLOR]`, `[B]`, `[I]`, … from labels.
- `addon_id` is extracted (`plugin://plugin.video.youtube/…` → `youtube`)
  and exposed to templates as `{addon}` / `{addon-full}` plus `{file-host}`.
- Unknown-length streams (no `totaltime`) show **without** a progress bar
  instead of being misreported as paused.
- Artwork: `image://` thumbnails resolve via `GET /image/<encoded>` with the
  same basic-auth client; direct `http(s)` plugin art is used as-is; imgur /
  litterbox upload + cache works unchanged.
- `unknown.display` in the config customizes plugin rows independently:

```json
"unknown": {
  "display": {
    "details_text": "{title}",
    "state_text": "via {addon} {sep} {file-host}",
    "image_text": "Kodi-RPC v{version}"
  },
  "separator": "-"
}
```

Available `{…}` keys for `unknown`: `{title}`, `{label}`, `{addon}`,
`{addon-full}`, `{file-host}`, `{genres}`, `{year}`, `{studio}`, `{plot}`,
`{version}`, `{sep}` — with example output for every key in
[docs/configuration.md](docs/configuration.md).

## Display templates & blacklist

Movies / episodes / music keep the jellyfin-rpc template keys (`{title}`,
`{show-title}`, `{season}`, `{episode}`, `{track}`, `{artists}`, `{genres}`,
… — see `example.json`).

Discord field limits (enforced automatically): details/state/image tooltips
≤ 128 chars (padded to 3 when shorter), max 2 buttons with labels ≤ 32 chars.

Blacklist `libraries` entries match as **case-insensitive substrings**
against the Kodi `file` path, so you can hide addons or shares:

```json
"blacklist": {
  "media_types": ["livetv"],
  "libraries": ["plugin.video.sample", "Kids"]
}
```

`media_types` accepts `music`, `movie`, `episode`, `livetv`, `unknown`
(plugin catch-all, also `plugin`). See
[docs/configuration.md](docs/configuration.md) for aliases, per-key
examples, and the full option reference.

Dynamic (`"name": "dynamic", "url": "dynamic"`) buttons resolve from
series/movie library info — a TMDB link and, for movies, the trailer link
when scraped — never from episode-level IDs (unreliable) or playback file
URLs (often localhost proxies). (Discord only shows buttons to *other*
users, never on your own profile.)

## Artwork behavior (`show_images`)

Same fallback chain as jellyfin-rpc: imgur upload → litterbox upload →
direct URL → default icon. Two Kodi-specific rules:

- **Precedence is poster-first**: series/season poster (episodes, via
  `VideoLibrary.GetSeasons` → `GetTVShowDetails`) or movie poster (via
  `VideoLibrary.GetMovieDetails`) wins over the episode still (`thumbnail`),
  then fanart. The `episodes` section's `poster_source` picks which:
  `season` (season poster, then series — default), `series` (series poster
  only), or `episode` (episode still, no poster lookups at all). Results are
  cached per show/season/movie so this costs one extra RPC per new item,
  not one per poll. Non-library (`plugin://`) content keeps the old
  still → fanart order.
- **Only Discord-fetchable URLs go direct**: Discord loads `large_image`
  server-side with no auth, so `image://` art is only handed to Discord
  when Kodi is publicly reachable **without** HTTP auth.
  `localhost`/LAN/authed setups fall back to the default icon (a broken
  `large_image` breaks the whole presence render, not just the picture) —
  enable `imgur_images` or `litterbox_images` to show local art instead
  (upload cache keys include the art source, so a changed poster
  re-uploads instead of serving a stale still).
  Plugin/CDN `http(s)` art (e.g. TMDB fanart) always passes straight through.
  `image://`-wrapped remote art (e.g. Crunchyroll's CDN thumbnails) is
  unwrapped to the inner `https://` URL, so it displays with no upload
  round-trip. When no art applies, the fallback is the Kodi logo.

## Credits

- Upstream: [JustRadical/jellyfin-rpc](https://github.com/justradical/jellyfin-rpc)
  (GPL-3.0-or-later; this fork keeps the same license).
- Kodi JSON-RPC docs: <https://kodi.wiki/view/JSON-RPC_API>
- This reimplementation was made possible with Muse Spark (`muse-spark-1.3`).
