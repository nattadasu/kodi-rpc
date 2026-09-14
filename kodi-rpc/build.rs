//! Exposes the current commit as `KODI_RPC_GIT_SHA` for snapshot builds.
//!
//! Untagged builds report `CARGO_PKG_VERSION+<sha>` (e.g. in the presence
//! tooltip and startup log) so CI artifacts are traceable to a commit.
//! Release builds set `KODI_RPC_RELEASE=1`, which suppresses the suffix and
//! keeps the version clean.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=KODI_RPC_RELEASE");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");

    // Release builds (tags, `task build`) keep a clean version.
    if std::env::var("KODI_RPC_RELEASE").is_ok() {
        return;
    }

    // CI always provides GITHUB_SHA (and changing it re-triggers this script,
    // so the stamped commit can never go stale there).
    if let Ok(sha) = std::env::var("GITHUB_SHA") {
        let short: String = sha.chars().take(7).collect();
        if !short.is_empty() {
            println!("cargo:rustc-env=KODI_RPC_GIT_SHA={short}");
        }
        return;
    }

    if let Some(sha) = git_sha() {
        println!("cargo:rustc-env=KODI_RPC_GIT_SHA={sha}");
    }
}

fn git_sha() -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let sha = String::from_utf8(out.stdout).ok()?;
    let sha = sha.trim().to_string();
    if sha.is_empty() {
        None
    } else {
        Some(sha)
    }
}
