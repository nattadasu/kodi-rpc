use crate::VERSION;
use colored::Colorize;
use kodi_rpc::version_string;
use log::warn;

pub fn checker() {
    // Snapshot builds (commit hash stamped in) can never match a release
    // tag, so there is nothing to compare — stay quiet.
    if version_string().contains('+') {
        return;
    }
    let current = VERSION.unwrap_or("0.0.0").to_string();
    // Tags carry a `v` prefix (`v1.1.0`) while the package version is bare
    // (`1.1.0`); compare and display the trimmed form on both sides.
    let latest = get_latest_github()
        .unwrap_or(current.clone())
        .trim_start_matches('v')
        .to_string();
    if latest != current {
        warn!(
            "{} (Current: v{}, Latest: v{})",
            "You are not running the latest version of Kodi-RPC"
                .red()
                .bold(),
            current,
            latest,
        );
        warn!("{}", "A newer version can be found at".red().bold());
        warn!(
            "{}",
            "https://github.com/nattadasu/kodi-rpc/releases/latest"
                .green()
                .bold()
        );
        warn!(
            "{}",
            "This can be safely ignored if you are running a prerelease version".bold()
        );
    }
}

fn get_latest_github() -> Result<String, reqwest::Error> {
    let final_url =
        reqwest::blocking::get("https://github.com/nattadasu/kodi-rpc/releases/latest")?
            .url()
            .to_string();
    // With no releases published yet, GitHub redirects /releases/latest to
    // /releases (no tag) — stay quiet instead of warning with a garbage version.
    match final_url.split_once("/releases/tag/") {
        Some((_, tag)) if !tag.is_empty() => Ok(tag.to_string()),
        _ => Ok(VERSION.unwrap_or("0.0.0").to_string()),
    }
}
