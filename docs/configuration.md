# Configuration

`kodi-rpc` reads one JSON file at startup (see `example.json` for a full
template). Default locations:

- Linux/macOS: `~/.config/kodi-rpc/main.json`
- Windows: `%appdata%\kodi-rpc\main.json`

Override with `-c /path/to/main.json`. A legacy `{"jellyfin": {...}}` block
from jellyfin-rpc is auto-mapped to `{"kodi": {...}}` (set
`username`/`password` for Kodi HTTP auth).

## `kodi`

| Key | Type | Default | Notes |
|---|---|---|---|
| `url` | string | — | Base URL, e.g. `http://localhost:8080`. Ignored when `instances` is set. |
| `urls` | array | `[]` | Alternate base URLs for this server (same credentials). Non-empty trumps `url`; each becomes a polled instance. |
| `username` / `password` | string | `""` | Kodi HTTP auth (Settings → Services → Control). Empty = none. |
| `self_signed_cert` | bool | `false` | Skip TLS verification. |
| `instances` | array | `[]` | Extra servers, see below. Non-empty = top-level `url`/creds ignored. |
| `music` / `movies` / `episodes` / `unknown` | object | — | Display sections, see below. |
| `blacklist` | object | — | `media_types` + `libraries`, see below. |
| `show_simple`, `append_prefix`, `add_divider` | bool | `false` | Legacy episode formatting (only when `episodes.display` is unset). |

### Instances

```json
"instances": [
  {"url": "http://localhost:8080", "username": "kodi", "password": "kodi"},
  {"url": "http://192.168.1.6:8080", "name": "bedroom"}
]
```

Every instance is polled each cycle. Whichever plays **first** owns the
presence until it stops; a second playback never steals it. When the owner
goes idle, the first playing instance in config order takes over. Dead
instances are skipped. Each entry takes `url`, `username`, `password`,
`self_signed_cert`, and `name` (log label, defaults to the URL); an entry
may set `urls` instead of `url` (same failover expansion as top-level).
Each RPC is capped at 10s so a stalled host can't freeze polling.

## Display sections

Each of `music`, `movies`, `episodes`, `unknown` takes:

| Key | Type | Notes |
|---|---|---|
| `display` | array / string / object | `["genres"]`, `"genres,year"`, or full `{details_text, state_text, image_text}`. |
| `separator` | string | Replaces `{sep}` between fields. |
| `status_display_type` | string | `name`, `state`, or `details`. Picks the member-list status line. Default `name` (app name). |
| `poster_source` | string | Episodes only: `season` (default), `series`, or `episode` (still, no lookups). |
| `show_paused` | bool | Show this section while paused. Unset = `discord.show_paused`. |
| `buttons` | array | Buttons for this section. Unset = `discord.buttons`; `[]` hides all. |

Up to 2 buttons shown per presence. `{"name": "dynamic", "url": "dynamic"}`
entries fill from series/movie info (TMDB link, trailer); static entries
show as-is. Labels cap at 32 chars, URLs over 512 are skipped, and buttons
only render for other viewers. Paused sections show a static icon with no
timestamps unless their `show_paused` is false, which clears the presence.

In a full `{details_text, state_text, image_text}` object, `{__default}`
stands for the section default: details → track / title / show-title /
title (music / movies / episodes / unknown); state → `By {artists} {sep} `
for music, `via {addon} {sep} {file-host}` for plugin playback, empty
otherwise. With the array/string form you never write it — e.g.
`"display": ["genres"], "separator": "-"` on music renders details
`Midnight City`, state `By M83 - Electropop, Synthwave`.

Unknown `{key}`s are left as-is; doubled or dangling `{sep}`s are cleaned
up automatically.

Keys per section, with example output:

**music**

| Key | Example |
|---|---|
| `{track}` | `Midnight City` |
| `{album}` | `Hurry Up, We're Dreaming` |
| `{artists}` | `M83` |
| `{genres}` | `Electropop, Synthwave` |
| `{year}` | `2011` |
| `{version}` | `0.2.0` |
| `{sep}` | separator |

**movies**

| Key | Example |
|---|---|
| `{title}` | `Blade Runner 2049` |
| `{original-title}` | original title |
| `{genres}` | `Sci-Fi, Drama` |
| `{year}` | `2017` |
| `{critic-score}` | `🍅 87/100` |
| `{community-score}` | `⭐ 8.0/10` |
| `{version}` | `0.2.0` |
| `{sep}` | separator |

E.g. details `{title} ({year})` → `Blade Runner 2049 (2017)`.

**episodes**

| Key | Example |
|---|---|
| `{show-title}` | `Tom Scott: England` |
| `{title}` | `Episode 26` |
| `{original-title}` | original title |
| `{season}` | `1` |
| `{season-padded}` | `01` |
| `{episode}` | `26` |
| `{episode-padded}` | `26` |
| `{year}` | `2026` |
| `{genres}` | `Travel` |
| `{studio}` | `Nebula` |
| `{version}` | `0.2.0` |
| `{sep}` | separator |

E.g. state `{season}x{episode-padded} - {title}` → `1x26 - Episode 26`.

**unknown** (plugin/unclassified streams)

| Key | Example |
|---|---|
| `{title}` | cleaned label |
| `{label}` | raw label |
| `{addon}` | `retrospect` (short id) |
| `{addon-full}` | `plugin.video.retrospect` |
| `{file-host}` | `www.crunchyroll.com` (proxied host for local playback proxies) |
| `{genres}` | genres |
| `{year}` | year |
| `{studio}` | studio |
| `{plot}` | synopsis |
| `{version}` | `0.2.0` |
| `{sep}` | separator |

E.g. state `via {addon} {sep} {file-host}` → `via www.crunchyroll.com`
when no addon id applies.

Discord caps text at 128 chars (padded to 3 when shorter).

## Blacklist

```json
"blacklist": {
  "media_types": ["livetv"],
  "libraries": ["plugin.video.sample", "Kids"]
}
```

- `media_types`: `music` (also `audio`, `song`, `musicvideo`), `movie`,
  `episode`, `livetv` (also `tvchannel`, `channel`), `unknown` (also
  `plugin`; catches all plugin/unrecognized streams).
- `libraries`: case-insensitive substrings matched against the Kodi `file`
  path. Blacklisted content is skipped (presence left untouched).

## `discord`

| Key | Type | Default | Notes |
|---|---|---|---|
| `application_id` | string | built-in | Custom Discord app ID. |
| `buttons` | array | dynamic | Default buttons (see section overrides above). |
| `show_paused` | bool | `true` | Default paused behavior (see section overrides above). |

## Images

Artwork precedence: series/season/movie poster → episode still → fanart.
Plugin/CDN `https://` art (including `image://`-wrapped remote art) is used
directly. Kodi `image://` art is handed to Discord only when Kodi is
publicly reachable *without* auth; otherwise it uploads (below) or falls
back to the Kodi logo — an unfetchable `large_image` breaks the whole
presence, not just the picture.

| Key | Type | Default | Notes |
|---|---|---|---|
| `enable_images` | bool | `false` | Master toggle. |
| `imgur_images` | bool | `false` | Upload art to Imgur (needs `imgur.client_id`). Permanent links. |
| `litterbox_images` | bool | `false` | Upload to litterbox (catbox). Links die after 72h; cache evicts then. Corrupt entries evict and re-upload instead of erroring. |
| `process_images` | bool | `true` | Square + blur into a 1:1 image before upload. |
| `size` | number | original | Output canvas, e.g. `512`. |
| `bg` / `bg_blur` / `corner_radius` | bool / number / number | `true` / `3.0` / `4.0` | Blurred background vs rounded corners. |

Uploads are cached in `urls.json` (next to the config; override with
`-i`), keyed by item + art source, so changed art re-uploads instead of
serving stale uploads.
