# Kodi-RPC

Program used to display what you're currently watching on Kodi in Discord.

Hard clone of [jellyfin-rpc](https://github.com/justradical/jellyfin-rpc) by
JustRadical, retargeted at the **Kodi JSON-RPC API**. The Discord presence
layer (buttons, display templates, blacklist, imgur/litterbox artwork,
paused icon, timestamps) is intentionally kept 1:1 so jellyfin-rpc configs
and habits carry over.

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

Alternatively, grab a release binary and run the installer:

```bash
python3 scripts/installer.py
```

Tagged builds (`vX.Y.Z`, `vX.Y.Z-beta.N` for prereleases) publish
stable-named binaries to the GitHub release — the names `installer.py`
downloads. Untagged builds carry the commit hash (`v0.2.0+a1b2c3d`) and go
to CI artifacts.

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

Then run:

```bash
kodi-rpc -c ~/.config/kodi-rpc/main.json
```

> Tip: if kodi-rpc runs on the Kodi box itself, prefer
> `http://localhost:8080` over the public URL — same data, no public
> round-trip, and polling keeps working if the tunnel drops.

Legacy `{"jellyfin": {...}}` configs are auto-mapped to `{"kodi": {...}}`.

## Features

- **3rd-party plugins**: `type: "unknown"` streams display instead of
  vanishing (see upstream Kodi issue
  [#16245](https://github.com/xbmc/xbmc/issues/16245)); labels are cleaned,
  addon ids extracted, progress-bar-less streams show without timestamps.
- **Multiple instances**: add an `instances` array — whichever box plays
  first owns the presence until it stops.
- **Poster-first artwork**: series/season/movie posters over episode stills
  (configurable per `poster_source`); local art uploads to imgur/litterbox.
- **Display templates** per media type, blacklist by type or path, dynamic
  TMDB/trailer buttons, pause-aware presence.

Details, template keys with examples, and every option:
[docs/configuration.md](docs/configuration.md).

## Credits

- Upstream: [JustRadical/jellyfin-rpc](https://github.com/justradical/jellyfin-rpc)
  (GPL-3.0-or-later; this fork keeps the same license).
- Kodi JSON-RPC docs: <https://kodi.wiki/view/JSON-RPC_API>
- This reimplementation was made possible with Muse Spark (`muse-spark-1.3`).
