# Configuration

`kodi-rpc` reads one JSON file at startup (see `example.json` for a full template).

Default configuration locations:

- **Linux/macOS**: `~/.config/kodi-rpc/main.json`
- **Windows**: `%appdata%\kodi-rpc\main.json`

Override the default location with `-c /path/to/main.json`.

## Top-level Configuration Keys

| Key | Type | Required | Description |
| :--- | :---: | :---: | :--- |
| `kodi` | [`Kodi`](#kodi) | Yes | Kodi server options, display configurations, and blacklists. |
| `discord` | [`Discord`](#discord) | No | Discord application ID, global default buttons, and pause behavior. |
| `imgur` | [`Imgur`](#imgur) | No | Imgur API credentials (`client_id`). |
| `images` | [`Images`](#images) | No | Artwork processing and image pipeline options. |

`kodi` is the only required top-level key. Unknown keys (e.g., `_comment`) are ignored throughout the configuration file.

<details>
<summary>JSON Schema (<code>schema.json</code>, click to expand)</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://raw.githubusercontent.com/nattadasu/kodi-rpc/main/schema.json",
  "title": "kodi-rpc configuration",
  "description": "Config file for kodi-rpc (default ~/.config/kodi-rpc/main.json). Unknown keys such as _comment are ignored.",
  "type": "object",
  "properties": {
    "kodi": { "$ref": "#/$defs/Kodi" },
    "discord": { "$ref": "#/$defs/Discord" },
    "imgur": { "$ref": "#/$defs/Imgur" },
    "images": { "$ref": "#/$defs/Images" }
  },
  "required": ["kodi"],
  "additionalProperties": true,
  "$defs": {
    "Kodi": {
      "type": "object",
      "properties": {
        "url": { "type": "string" },
        "urls": { "type": "array", "items": { "type": "string" } },
        "username": { "type": "string" },
        "password": { "type": "string" },
        "self_signed_cert": { "type": "boolean" },
        "instances": { "type": "array", "items": { "$ref": "#/$defs/Instance" } },
        "music": { "$ref": "#/$defs/Display" },
        "movies": { "$ref": "#/$defs/Display" },
        "episodes": { "$ref": "#/$defs/Display" },
        "unknown": { "$ref": "#/$defs/Display" },
        "blacklist": {
          "type": "object",
          "properties": {
            "media_types": {
              "type": "array",
              "items": {
                "type": "string",
                "enum": [
                  "music",
                  "movie",
                  "episode",
                  "livetv",
                  "unknown",
                  "audio",
                  "song",
                  "musicvideo",
                  "tvchannel",
                  "channel",
                  "plugin"
                ]
              }
            },
            "libraries": { "type": "array", "items": { "type": "string" } }
          },
          "additionalProperties": true
        },
        "show_simple": { "type": "boolean" },
        "append_prefix": { "type": "boolean" },
        "add_divider": { "type": "boolean" }
      },
      "additionalProperties": true
    },
    "Instance": {
      "type": "object",
      "properties": {
        "url": { "type": "string" },
        "urls": { "type": "array", "items": { "type": "string" } },
        "username": { "type": "string" },
        "password": { "type": "string" },
        "self_signed_cert": { "type": "boolean" },
        "name": { "type": "string" }
      },
      "additionalProperties": true
    },
    "Display": {
      "type": "object",
      "properties": {
        "display": {
          "anyOf": [
            { "type": "array", "items": { "type": "string" } },
            { "type": "string" },
            {
              "type": "object",
              "properties": {
                "details_text": { "type": "string" },
                "state_text": { "type": "string" },
                "image_text": { "type": "string" }
              },
              "additionalProperties": true
            }
          ]
        },
        "separator": { "type": "string" },
        "status_display_type": {
          "type": "string",
          "enum": ["name", "state", "details"]
        },
        "poster_source": {
          "type": "string",
          "enum": ["season", "series", "episode", "still"]
        },
        "show_paused": { "type": "boolean" },
        "buttons": { "type": "array", "items": { "$ref": "#/$defs/Button" } }
      },
      "additionalProperties": true
    },
    "Button": {
      "type": "object",
      "properties": {
        "name": { "type": "string" },
        "url": { "type": "string" }
      },
      "required": ["name", "url"],
      "additionalProperties": true
    },
    "Discord": {
      "type": "object",
      "properties": {
        "application_id": { "type": "string" },
        "buttons": { "type": "array", "items": { "$ref": "#/$defs/Button" } },
        "show_paused": { "type": "boolean" }
      },
      "additionalProperties": true
    },
    "Imgur": {
      "type": "object",
      "properties": {
        "client_id": { "type": "string" }
      },
      "additionalProperties": true
    },
    "Images": {
      "type": "object",
      "properties": {
        "enable_images": { "type": "boolean" },
        "imgur_images": { "type": "boolean" },
        "litterbox_images": { "type": "boolean" },
        "process_images": { "type": "boolean" },
        "size": { "type": "integer" },
        "bg": { "type": "boolean" },
        "bg_blur": { "type": "number" },
        "corner_radius": { "type": "number" }
      },
      "additionalProperties": true
    }
  }
}
```

</details>

## Valid Keys

### `kodi`

Connection settings, display sections, and blacklists. While no individual key in `kodi` is mandatory, at least one valid server source (`url`, `urls`, or `instances`) must be provided for the application to function.

| Key | Type | Required | Default | Description |
| :--- | :---: | :---: | :---: | :--- |
| `url` | `string` | No | `""` | Base URL of the Kodi HTTP server (e.g., `http://localhost:8080`). Ignored when `instances` or `urls` is configured. |
| `urls` | `arr[string]` | No | `[]` | Alternate base URLs for this server (sharing the same credentials). Non-empty list overrides `url`; each URL is polled as an instance. |
| `username` | `string` | No | `""` | Kodi HTTP authentication username (**Settings → Services → Control**). Leave empty if no authentication is required. |
| `password` | `string` | No | `""` | Kodi HTTP authentication password (**Settings → Services → Control**). Leave empty if no authentication is required. |
| `self_signed_cert` | `boolean` | No | `false` | Skip TLS certificate verification when connecting to Kodi over HTTPS. |
| `instances` | [`arr[Instance]`](#kodiinstances) | No | `[]` | Array of additional Kodi server instances. Non-empty list overrides top-level `url`, `urls`, `username`, and `password`. |
| `music` | [`Display`](#display) | No | `{}` | Music display configuration. |
| `movies` | [`Display`](#display) | No | `{}` | Movie display configuration. |
| `episodes` | [`Display`](#display) | No | `{}` | TV episode display configuration. |
| `unknown` | [`Display`](#display) | No | `{}` | Unclassified stream and 3rd-party video plugin display configuration. |
| `blacklist` | [`Blacklist`](#kodiblacklist) | No | `{}` | Filtering rules (`media_types` and `libraries`). |
| `show_simple` | `boolean` | No | `false` | Legacy episode formatting option (only evaluated when [`episodes.display`](#display) is omitted). |
| `append_prefix` | `boolean` | No | `false` | Legacy episode formatting option: adds leading zero to season/episode numbers under 10. |
| `add_divider` | `boolean` | No | `false` | Legacy episode formatting option: adds divider between season and episode numbers. |

#### `kodi.instances`

Configures multiple Kodi servers:

```json
"instances": [
  { "url": "http://localhost:8080", "username": "kodi", "password": "kodi" },
  { "url": "http://192.168.1.6:8080", "name": "bedroom" }
]
```

| Key | Type | Required | Default | Description |
| :--- | :---: | :---: | :---: | :--- |
| `url` | `string` | No | `""` | Base URL for this server instance. May be omitted if `urls` is specified. |
| `urls` | `arr[string]` | No | `[]` | Alternate base URLs for this server instance; overrides `url`. |
| `username` | `string` | No | `""` | HTTP authentication username for this server instance. |
| `password` | `string` | No | `""` | HTTP authentication password for this server instance. |
| `self_signed_cert` | `boolean` | No | `false` | Skip TLS certificate verification for this server instance. |
| `name` | `string` | No | server URL | Display label used in log output for this server instance. |

Every instance is polled during each poll cycle. The instance that starts playback first retains presence ownership until playback stops; subsequent playback on other instances will not override active presence. When the active instance becomes idle, the first playing instance in configuration order takes over. Unreachable instances are skipped automatically, with RPC timeouts capped at 10 seconds.

#### `kodi.blacklist`

Suppresses Rich Presence display for specified media types or path substrings:

```json
"blacklist": {
  "media_types": ["livetv"],
  "libraries": ["plugin.video.sample", "Kids"]
}
```

| Key | Type | Required | Default | Description |
| :--- | :---: | :---: | :---: | :--- |
| `media_types` | `arr[enum["music", "movie", "episode", "livetv", "unknown", "audio", "song", "musicvideo", "tvchannel", "channel", "plugin"]]` | No | `[]` | List of media types to filter out. Primary types: `"music"`, `"movie"`, `"episode"`, `"livetv"`, `"unknown"`. Aliases: `"audio"`, `"song"`, `"musicvideo"`, `"tvchannel"`, `"channel"`, `"plugin"`. |
| `libraries` | `arr[string]` | No | `[]` | Case-insensitive substrings matched against the Kodi `file` path. Blacklisted content suppresses Discord presence updates. |

## Display Configuration

You can customize how playback details look in Discord presence for each media type (`music`, `movies`, `episodes`, `unknown`).

### Display Formats

`display` supports three valid variants:

#### Array Format (`arr[string]`)

An array of template field names. Each field is evaluated and appended to the standard status line, separated by your configured `separator`.

```json
"episodes": {
  "display": ["genres", "year"],
  "separator": " - "
}
```

#### String Format (`string`)

A single comma-separated string listing template field names. This is a quick shorthand for the array format.

```json
"episodes": {
  "display": "genres,year",
  "separator": " - "
}
```

#### Object Format (`object`)

A custom layout object giving you full line-by-line control over every text field in Discord presence.

```json
"episodes": {
  "display": {
    "details_text": "{show-title}",
    "state_text": "S{season-padded}E{episode-padded} - {title}",
    "image_text": "Watching on Kodi-RPC v{version}"
  },
  "separator": " - "
}
```

The `display` object format accepts the following fields:

| Key | Type | Required | Default | Description |
| :--- | :---: | :---: | :---: | :--- |
| `details_text` | `string` | No | Section default | First (top) line of text in Discord presence. |
| `state_text` | `string` | No | Section default | Second line of text in Discord presence. |
| `image_text` | `string` | No | `"Kodi-RPC v{version}"` | Hover tooltip text shown on the large cover artwork image. |

### Display Options Table

| Key | Type | Required | Default | Description |
| :--- | :---: | :---: | :---: | :--- |
| `display` | `arr[string]` \| `string` \| [`object`](#object-format-object) | No | Section default | Layout format for presence text (see variants above). |
| `separator` | `string` | No | `"-"` | Character or text replacing `{sep}` placeholders between fields. |
| `status_display_type` | `enum["name", "state", "details"]` | No | `"name"` | Controls member list status text on Discord: `"name"` (app name), `"state"`, or `"details"`. |
| `poster_source` | `enum["season", "series", "episode", "still"]` | No | `"season"` | Episode artwork source: `"season"` (season poster, then series), `"series"` (series poster only), or `"episode"` / `"still"` (episode screenshot). |
| `show_paused` | `boolean` | No | [`discord.show_paused`](#discord) | Show presence for this section while paused. Uses [`discord.show_paused`](#discord) if omitted. |
| `buttons` | `arr[object]` | No | [`discord.buttons`](#discord) | Custom buttons for this section (`{ "name": "...", "url": "..." }`, templates supported — see [Button Templates](#button-templates)). Uses [`discord.buttons`](#discord) if omitted; `[]` hides all buttons. |

### Default Line Formats

If you don't specify custom line text, Kodi-RPC uses these defaults for each media type:

- **Music (`music`)**:
  - Details line: `{track}`
  - State line: `By {artists}`
  - Artwork hover text: `Kodi-RPC v{version}`
- **Movies (`movies`)**:
  - Details line: `{title}`
  - State line: Empty
  - Artwork hover text: `Kodi-RPC v{version}`
- **Episodes (`episodes`)**:
  - Details line: `{show-title}`
  - State line: `S{season-padded}E{episode-padded} - {title}`
  - Artwork hover text: `Kodi-RPC v{version}`
- **Plugins / Streams (`unknown`)**:
  - Details line: `{title}`
  - State line: `via {addon} - {file-host}`
  - Artwork hover text: `Kodi-RPC v{version}`

Unrecognized template variables are displayed as normal text. Any duplicate or leftover `{sep}` separators are cleaned up automatically.

Up to 2 buttons per activity. `{"name": "dynamic", "url": "dynamic"}` links to the trailer and info pages (TMDB, TVDB, WeTrakr, IMDb). Labels cap at 32 characters, URLs over 512 characters are dropped.

### Button Templates

Custom (non-`dynamic`) buttons accept the same `{placeholders}` as their section's display templates, in both `name` and `url` (see [Template Placeholders Reference](#template-placeholders-reference)):

```json
"movies": {
    "buttons": [
        { "name": "dynamic", "url": "dynamic" },
        { "name": "Search {title} Trailer", "url": "https://www.youtube.com/results?search_query={title}+{year}+trailer" }
    ]
}
```

Example: watching `Blade Runner 2049 (2017)` → name `Search Blade Runner 2049 Trailer`, url `https://www.youtube.com/results?search_query=Blade%20Runner%202049+2017+trailer`

```json
"episodes": {
    "buttons": [
        { "name": "dynamic", "url": "dynamic" },
        { "name": "{show-title} S{season-padded}E{episode-padded}", "url": "https://example.com/search?q={show-title}+{title}" }
    ]
}
```

#### Button-only placeholders

Available in every section:

| Placeholder | Example Output |
| :--- | :--- |
| `{tmdb}` | `550` |
| `{tvdb}` | `400123` |
| `{imdb}` | `tt0120338` |
| `{trailer}` | Trailer URL from the library |

Example: `{ "name": "WeTrakr", "url": "https://wetrakr.com/tmdb/movies/{tmdb}" }` → `https://wetrakr.com/tmdb/movies/550`

Example: `{ "name": "Watch Trailer", "url": "{trailer}" }` → trailer button with a custom label.


#### Rendering rules

| Rule | Description |
| :--- | :--- |
| `url` encoding | Values are percent-encoded, except `{trailer}` which is already a URL. |
| `name` rendering | Values are substituted raw, like display templates. |
| Hidden buttons | A button is hidden when a `url` placeholder is empty, its `name` is empty, or the `url` is not valid `http(s)`. |
| Plain buttons | Buttons without placeholders are unchanged. |

### Template Placeholders Reference

Available placeholders per section:

#### Music (`music`)

| Placeholder | Example Output |
| :--- | :--- |
| `{track}` | `Midnight City` |
| `{album}` | `Hurry Up, We're Dreaming` |
| `{artists}` | `M83` |
| `{genres}` | `Electropop, Synthwave` |
| `{year}` | `2011` |
| `{version}` | `1.2.0` |
| `{sep}` | Separator string |

#### Movies (`movies`)

| Placeholder | Example Output |
| :--- | :--- |
| `{title}` | `Blade Runner 2049` |
| `{original-title}` | Original movie title |
| `{genres}` | `Sci-Fi, Drama` |
| `{year}` | `2017` |
| `{critic-score}` | `🍅 87/100` |
| `{community-score}` | `⭐ 8.0/10` |
| `{version}` | `1.2.0` |
| `{sep}` | Separator string |

Example: `details_text: "{title} ({year})"` → `Blade Runner 2049 (2017)`

#### Episodes (`episodes`)

| Placeholder | Example Output |
| :--- | :--- |
| `{show-title}` | `Tom Scott: England` |
| `{title}` | `Episode 26` |
| `{original-title}` | Original episode title |
| `{season}` | `1` |
| `{season-padded}` | `01` |
| `{episode}` | `26` |
| `{episode-padded}` | `26` |
| `{year}` | `2026` |
| `{genres}` | `Travel` |
| `{studio}` | `Nebula` |
| `{version}` | `1.2.0` |
| `{sep}` | Separator string |

Example: `state_text: "{season}x{episode-padded} - {title}"` → `1x26 - Episode 26`

#### Unknown / Plugin Streams (`unknown`)

| Placeholder | Example Output |
| :--- | :--- |
| `{title}` | Cleaned item label |
| `{label}` | Raw item label |
| `{addon}` | Addon ID (`retrospect`) |
| `{addon-full}` | Full addon ID (`plugin.video.retrospect`) |
| `{file-host}` | Domain host (`www.crunchyroll.com`) |
| `{genres}` | Stream genres |
| `{year}` | Release year |
| `{studio}` | Studio name |
| `{plot}` | Synopsis / plot summary |
| `{version}` | `1.2.0` |
| `{sep}` | Separator string |

Example: `state_text: "via {addon} {sep} {file-host}"` → `via www.crunchyroll.com`

> [!NOTE]
> Discord limits presence text fields to 128 characters. Strings shorter than 3 characters are automatically padded.

## `discord`

Global presence defaults and Discord app settings.

| Key | Type | Required | Default | Description |
| :--- | :---: | :---: | :---: | :--- |
| `application_id` | `string` | No | `"1549062924545556480"` | Custom Discord Application ID. |
| `buttons` | `arr[object]` | No | Dynamic links | Default buttons for all sections unless overridden in a specific display section. |
| `show_paused` | `boolean` | No | `true` | Default paused behavior unless overridden in a specific display section. |

## `imgur`

Credentials for Imgur image hosting.

| Key | Type | Required | Default | Description |
| :--- | :---: | :---: | :---: | :--- |
| `client_id` | `string` | No | `""` | Imgur API Client ID for uploading presence artwork. |

## `images`

Artwork pipeline, processing, and hosting configuration.

Precedence order for artwork: Series/Season/Movie Poster → Episode Still → Fanart.
Direct HTTP/HTTPS URLs (including remote `image://` URLs) are passed directly to Discord. Local Kodi `image://` artwork is served directly to Discord only if Kodi is publicly accessible without authentication. Otherwise, artwork must be uploaded (via Imgur or Litterbox) or falls back to default artwork.

| Key | Type | Required | Default | Description |
| :--- | :---: | :---: | :---: | :--- |
| `enable_images` | `boolean` | No | `false` | Master toggle for artwork retrieval and processing. |
| `imgur_images` | `boolean` | No | `false` | Upload artwork to Imgur (requires [`imgur.client_id`](#imgur)). Creates permanent links. |
| `litterbox_images` | `boolean` | No | `false` | Upload artwork to Litterbox (Catbox). Links expire after 72 hours; cache evicts and re-uploads automatically. |
| `process_images` | `boolean` | No | `true` | Crop and format artwork into a 1:1 square canvas before uploading. |
| `size` | `integer` | No | Original size | Output canvas square dimensions in pixels (e.g., `512`). |
| `bg` | `boolean` | No | `true` | Add a blurred background behind non-square artwork. |
| `bg_blur` | `number` | No | `3.0` | Background blur radius percentage relative to canvas size. |
| `corner_radius` | `number` | No | `4.0` | Corner rounding percentage (applied when `bg` is `false`). |

Upload mappings are cached in `urls.json` alongside `main.json` (override location with `-i`).
