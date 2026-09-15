# Development

## Tooling

- `cargo` (stable), `cargo fmt` (rustfmt), `task` ([go-task](https://taskfile.dev), optional)
- `cargo clippy` for `task lint` (needs the clippy component)

| Command | What |
|---|---|
| `task build` | Release binary → `target/release/kodi-rpc` |
| `task dev` | Fast debug build |
| `task test` | Full test suite |
| `task lint` | Clippy, all targets |
| `task fmt` / `task fmt-check` | Format / fail-if-unformatted (CI enforces) |
| `task run` | Run against `~/.config/kodi-rpc/main.json` (`CONFIG=`, `WAIT=` override) |
| `task install` | Release-build + copy to `~/.local/bin` |
| `task clean` | Remove `target/` |

## Tests

`cargo test --workspace` runs everything. One test is live and ignored by
default — it polls a real Kodi, so credentials stay in env vars, never in
the repo:

```bash
KODI_TEST_URL=http://localhost:8080 \
KODI_TEST_USER=kodi \
KODI_TEST_PASS=secret \
cargo test -p kodi-rpc -- --ignored --nocapture live_
```

## Releases

Commits follow [Conventional Commits](https://www.conventionalcommits.org/)
(`feat:`, `fix:`, `docs:`, `chore:`, …).

Releases are cut manually: bump both crate versions, commit, tag, push.

- Untagged pushes/PRs: `build.yml` runs fmt, tests, and uploads
  `kodi-rpc-<sha>-<platform>` snapshot artifacts.
- Tags `vX.Y.Z` (stable) or `vX.Y.Z-beta.N` (prerelease): `release.yml`
  builds with `KODI_RPC_RELEASE=1` (clean version, stable asset names the
  installer downloads) and attaches the binaries.
- Local builds behave like snapshots (`<version>+<sha>`); set
  `KODI_RPC_RELEASE=1` yourself for a clean local release build.
