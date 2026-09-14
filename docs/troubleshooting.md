# Troubleshooting

Symptom-first fixes for problems actually hit with this app. Run with
`-v debug` first — every poll, election, upload, and fallback is logged.

## Nothing shows in Discord at all

1. **Wrong client owns the socket.** Only one app can hold
   `discord-ipc-0`, and kodi-rpc talks to whoever holds it. If you run
   more than one Discord client, check `ss -x -l -p | grep discord-ipc`
   and look at that client, not the other one.
2. **Activity sharing is off.** Discord settings → Activity Privacy →
   *Display current activity as a status message* must be enabled, or
   nothing renders for anyone.
3. **Discord isn't on this machine.** Rich Presence works over a local IPC
   socket, so kodi-rpc must run where Discord desktop runs. Point it at a
   remote Kodi via `url` instead.
4. **Confirm the connection.** Discord's `renderer_js.log` prints
   `RPCServer:IPC Socket Opened … <your app id>` every time kodi-rpc
   connects, plus a `Socket Message`/`Socket Emit` pair per update.

## Presence disappeared after enabling images

An unfetchable `large_image` kills the whole render, not just the picture.
`localhost`/LAN/authed Kodi artwork URLs can never load in Discord — the
debug line `didnt return an image, using default..` means the fallback
icon was used instead. Fix: enable `litterbox_images` (or `imgur_images`)
so local art uploads first, or turn images off.

## Polls stall or presence freezes

Prefer `http://localhost:8080` when kodi-rpc runs on the Kodi box — no
tunnel to drop. Every RPC is capped at 10s and dead instances are skipped
(`Watching N Kodi instance(s)` in the debug log tells you how many
parsed), so a stall past ~30s means all instances are unreachable, not one.

## 401 Unauthorized

Wrong Kodi username/password, or HTTP control is off (Kodi → Settings →
Services → Control → Allow remote control via HTTP).

## Buttons don't appear

Discord renders buttons for *other* viewers only — never on your own
profile. Verify with a second account. Also: labels cap at 32 chars and
URLs over 512 chars are dropped, so overlong entries silently vanish.

## Config changes do nothing

The config loads once at startup — restart kodi-rpc. Validate JSON first:
`python3 -m json.tool ~/.config/kodi-rpc/main.json`.

## Wrong or stale artwork

Litterbox links die server-side after 72h (cache evicts then). Uploads are
cached in `urls.json` keyed by item + art source; delete it to force
re-uploads. Plugin/CDN `https://` art needs no upload at all.

## Paused timer keeps running, elapsed jumps

Pause/play flips push a fresh presence (static paused icon, no timer;
timestamps restart from the live position on resume). A visible jump right
after a reconnect is the same mechanism: timestamps always derive from the
current Kodi position, never stored.
