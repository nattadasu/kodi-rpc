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
instances are skipped. Each entry takes `url` (required), `username`,
`password`, `self_signed_cert`, and `name` (log label, defaults to the URL).
Each RPC is capped at 10s so a stalled host can't freeze polling.

## Display sections

Each of `music`, `movies`, `episodes`, `unknown` takes:

| Key | Type | Notes |
|---|---|---|
| `display` | array / string / object | `["genres"]`, `"genres,year"`, or full `{details_text, state_text, image_text}`. |
| `separator` | string | Replaces `{sep}` between fields. |
| `status_display_type` | string | `name`, `state`, or `details`. Picks the member-list status line. Default `name` (app name). |
| `poster_source` | string | Episodes only: `season` (default), `series`, or `episode` (still, no lookups). |

`{__default}` inside a custom format expands to the section default
(track / title / show-title). Unknown `{key}`s are left as-is; doubled or
dangling `{sep}`s are cleaned up automatically.

Template keys per section:

- **music** (`details` defaults to track, `state` to artists):
  `{track}`, `{album}`, `{artists}`, `{genres}`, `{year}`, `{version}`, `{sep}`
- **movies** (`details` defaults to title):
  `{title}`, `{original-title}`, `{genres}`, `{year}`, `{critic-score}`,
  `{community-score}`, `{version}`, `{sep}`
- **episodes** (`details` defaults to show title):
  `{show-title}`, `{title}`, `{original-title}`, `{episode}`,
  `{episode-padded}`, `{season}`, `{season-padded}`, `{year}`, `{genres}`,
  `{studio}`, `{version}`, `{sep}`
- **unknown** — plugin/unclassified streams (`details` defaults to title):
  `{title}`, `{label}`, `{addon}`, `{addon-full}`, `{file-host}`,
  `{genres}`, `{year}`, `{studio}`, `{plot}`, `{version}`, `{sep}`

`{addon}` is the short addon id (`plugin.video.youtube` → `youtube`);
`{file-host}` is the stream host, or the proxied host for local playback
proxies. Discord caps text at 128 chars (padded to 3 when shorter).

## Blacklist

```json
"blacklist": {
  "media_types": ["livetv"],
  "libraries": ["plugin.video.sample", "Kids"]
}
```

- `media_types`: `music`, `movie`, `episode`, `livetv`, `unknown`
  (plugin catch-all), plus `musicvideo`, `channel`, `picture` and legacy
  `book`/`audiobook`.
- `libraries`: case-insensitive substrings matched against the Kodi `file`
  path. Blacklisted content is skipped (presence left untouched).

## `discord`

| Key | Type | Default | Notes |
|---|---|---|---|
| `application_id` | string | built-in | Custom Discord app ID. |
| `buttons` | array | dynamic | Up to 2 shown. `{"name": "dynamic", "url": "dynamic"}` fills from series/movie info (TMDB link, trailer). Labels cap at 32 chars; overlong URLs are skipped. Buttons only render for other viewers. |
| `show_paused` | bool | `true` | Show presence while paused (static icon, no timer). `false` clears it. |

Pause/play flips always push a fresh presence even when the text matches:
paused shows the icon with no timestamps, resume restarts them from the
live position.

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

## CLI flags

```
-c, --config <path>               Path to the config file
-i, --image-urls-file <path>      Path to image urls cache (imgur/litterbox)
-t, --wait-time <secs>            Poll interval [default: 7]
-v, --log-level <level>           trace|debug|info|warn|error|off [default: info]
```

## Versions

Untagged builds report `0.2.0+<sha>` (startup log, presence tooltip) and
CI uploads them as `kodi-rpc-<sha>-<platform>` artifacts. Tagged builds
(`vX.Y.Z`, `vX.Y.Z-beta.N` for prereleases) report the clean version and
publish stable-named binaries to the GitHub release. The update checker
stays quiet for hash-stamped builds.
