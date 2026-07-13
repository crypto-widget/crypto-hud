use std::{
    env,
    io::Read,
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use semver::Version;
use serde::Deserialize;

const DEFAULT_RELEASE_API_URL: &str =
    "https://api.github.com/repos/crypto-widget/crypto-hud/releases/latest";
const TRUSTED_RELEASE_TAG_URL_PREFIX: &str =
    "https://github.com/crypto-widget/crypto-hud/releases/tag/";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_UPDATE_RESPONSE_BYTES: u64 = 10 * 1024 * 1024;
const USER_AGENT: &str = concat!("crypto-hud/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCheckConfig {
    pub current_version: String,
    pub release_api_url: String,
    pub include_prereleases: bool,
    pub proxy_url: Option<String>,
}

impl Default for UpdateCheckConfig {
    fn default() -> Self {
        Self {
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            release_api_url: DEFAULT_RELEASE_API_URL.to_string(),
            include_prereleases: false,
            proxy_url: None,
        }
    }
}

impl UpdateCheckConfig {
    pub fn from_env() -> Option<Self> {
        if env_flag_enabled_with_legacy(
            "CRYPTO_HUD_DISABLE_UPDATE_CHECK",
            &["CRYPTO_WIDGET_DISABLE_UPDATE_CHECK"],
        ) {
            return None;
        }

        let mut config = Self::default();
        if let Some(url) = env_value_with_legacy(
            "CRYPTO_HUD_UPDATE_API_URL",
            &["CRYPTO_WIDGET_UPDATE_API_URL"],
        ) {
            if !url.trim().is_empty() {
                config.release_api_url = url;
            }
        }
        config.include_prereleases = env_flag_enabled_with_legacy(
            "CRYPTO_HUD_INCLUDE_PRERELEASE_UPDATES",
            &["CRYPTO_WIDGET_INCLUDE_PRERELEASE_UPDATES"],
        );
        Some(config)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateEvent {
    Available(UpdateInfo),
    UpToDate,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    pub tag_name: String,
    pub version: Version,
    pub html_url: String,
    pub asset_name: Option<String>,
    pub asset_url: Option<String>,
    pub checksum_asset_name: Option<String>,
    pub checksum_asset_url: Option<String>,
}

pub(crate) fn trusted_release_page_url(update: &UpdateInfo) -> Option<&str> {
    let tag_name = update
        .html_url
        .strip_prefix(TRUSTED_RELEASE_TAG_URL_PREFIX)?;
    (tag_name == update.tag_name).then_some(update.html_url.as_str())
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
}

pub fn spawn_update_check(config: UpdateCheckConfig) -> Receiver<UpdateEvent> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let event = match check_latest_release(&config) {
            Ok(Some(update)) => UpdateEvent::Available(update),
            Ok(None) => UpdateEvent::UpToDate,
            Err(error) => UpdateEvent::Error(error.to_string()),
        };
        let _ = sender.send(event);
    });
    receiver
}

pub fn check_latest_release(config: &UpdateCheckConfig) -> Result<Option<UpdateInfo>> {
    let agent = build_agent(config.proxy_url.as_deref())?;
    let deadline = request_deadline(&agent);
    let mut response = agent
        .get(&config.release_api_url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", USER_AGENT)
        .call()
        .with_context(|| format!("failed to check {}", config.release_api_url))?;
    let reader = response
        .body_mut()
        .with_config()
        .limit(MAX_UPDATE_RESPONSE_BYTES)
        .reader();
    let body = read_update_response(reader, deadline)?;

    update_from_release_json(&config.current_version, &body, config.include_prereleases)
}

fn request_deadline(agent: &ureq::Agent) -> Option<Instant> {
    agent
        .config()
        .timeouts()
        .global
        .and_then(|timeout| Instant::now().checked_add(timeout))
}

fn read_update_response(mut reader: impl Read, deadline: Option<Instant>) -> Result<String> {
    let mut body = Vec::new();
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        ensure_response_deadline(deadline)?;
        let read = reader
            .read(&mut buffer)
            .context("failed to read update response")?;
        ensure_response_deadline(deadline)?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&buffer[..read]);
    }
    String::from_utf8(body).context("update response was not valid UTF-8")
}

fn ensure_response_deadline(deadline: Option<Instant>) -> Result<()> {
    if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
        anyhow::bail!("failed to read update response: response body deadline exceeded");
    }
    Ok(())
}

fn build_agent(proxy_url: Option<&str>) -> Result<ureq::Agent> {
    let proxy = proxy_url
        .and_then(non_empty_trimmed)
        .map(configured_proxy)
        .transpose()
        .context("invalid proxy URL")?;
    let config = ureq::Agent::config_builder()
        // Keep proxying controlled by the application instead of ureq's environment defaults.
        .proxy(proxy)
        .timeout_global(Some(REQUEST_TIMEOUT))
        .timeout_resolve(Some(REQUEST_TIMEOUT))
        .timeout_connect(Some(REQUEST_TIMEOUT))
        .timeout_send_request(Some(REQUEST_TIMEOUT))
        .timeout_recv_response(Some(REQUEST_TIMEOUT))
        .timeout_recv_body(Some(REQUEST_TIMEOUT))
        .build();
    Ok(config.into())
}

fn configured_proxy(proxy_url: &str) -> std::result::Result<ureq::Proxy, ureq::Error> {
    let proxy = ureq::Proxy::new(proxy_url)?;
    if proxy.protocol() != ureq::ProxyProtocol::Socks5 {
        return Ok(proxy);
    }

    let mut builder = ureq::Proxy::builder(proxy.protocol())
        .host(proxy.host())
        .port(proxy.port())
        .resolve_target(false);
    if let Some(username) = proxy.username() {
        builder = builder.username(username);
    }
    if let Some(password) = proxy.password() {
        builder = builder.password(password);
    }
    builder.build()
}

fn non_empty_trimmed(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

pub fn update_from_release_json(
    current_version: &str,
    release_json: &str,
    include_prereleases: bool,
) -> Result<Option<UpdateInfo>> {
    let release = serde_json::from_str::<GitHubRelease>(release_json)
        .context("failed to parse update response")?;
    update_from_release(current_version, release, include_prereleases)
}

fn update_from_release(
    current_version: &str,
    release: GitHubRelease,
    include_prereleases: bool,
) -> Result<Option<UpdateInfo>> {
    let current = parse_version_tag(current_version).context("failed to parse current version")?;
    let latest = parse_version_tag(&release.tag_name).context("failed to parse release tag")?;
    if release.draft || (release.prerelease && !include_prereleases) || latest <= current {
        return Ok(None);
    }

    let asset = release
        .assets
        .iter()
        .find(|asset| is_package_asset(&asset.name));
    let checksum_asset = asset.and_then(|asset| find_checksum_asset(&release.assets, &asset.name));
    Ok(Some(UpdateInfo {
        tag_name: release.tag_name,
        version: latest,
        html_url: release.html_url,
        asset_name: asset.map(|asset| asset.name.clone()),
        asset_url: asset.map(|asset| asset.browser_download_url.clone()),
        checksum_asset_name: checksum_asset.map(|asset| asset.name.clone()),
        checksum_asset_url: checksum_asset.map(|asset| asset.browser_download_url.clone()),
    }))
}

fn is_package_asset(name: &str) -> bool {
    name.ends_with(".zip")
}

fn find_checksum_asset<'a>(
    assets: &'a [GitHubReleaseAsset],
    package_name: &str,
) -> Option<&'a GitHubReleaseAsset> {
    let expected_name = format!("{package_name}.sha256");
    assets
        .iter()
        .find(|asset| asset.name == expected_name)
        .or_else(|| assets.iter().find(|asset| asset.name.ends_with(".sha256")))
}

fn parse_version_tag(raw: &str) -> Result<Version> {
    let version = raw.trim().trim_start_matches(['v', 'V']);
    Version::parse(version).with_context(|| format!("invalid SemVer tag {raw}"))
}

#[cfg(test)]
fn env_flag_enabled(name: &str) -> bool {
    env_flag_value(name).unwrap_or(false)
}

fn env_flag_enabled_with_legacy(primary: &str, legacy: &[&str]) -> bool {
    env_flag_value(primary)
        .or_else(|| legacy.iter().find_map(|name| env_flag_value(name)))
        .unwrap_or(false)
}

fn env_flag_value(name: &str) -> Option<bool> {
    env::var(name)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
        })
        .ok()
}

fn env_value_with_legacy(primary: &str, legacy: &[&str]) -> Option<String> {
    env::var(primary)
        .ok()
        .or_else(|| legacy.iter().find_map(|name| env::var(name).ok()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn release_json(tag: &str, prerelease: bool) -> String {
        format!(
            r#"{{
              "tag_name": "{tag}",
              "html_url": "https://github.com/crypto-widget/crypto-hud/releases/tag/{tag}",
              "draft": false,
              "prerelease": {prerelease},
              "assets": [
                {{
                  "name": "crypto-hud-{tag}-windows-x64-portable.zip",
                  "browser_download_url": "https://example.test/{tag}.zip"
                }},
                {{
                  "name": "crypto-hud-{tag}-windows-x64-portable.zip.sha256",
                  "browser_download_url": "https://example.test/{tag}.zip.sha256"
                }}
              ]
            }}"#
        )
    }

    #[test]
    fn detects_newer_stable_release() {
        let update = update_from_release_json("0.1.0", &release_json("v0.1.1", false), false)
            .unwrap()
            .unwrap();

        assert_eq!(update.tag_name, "v0.1.1");
        assert_eq!(update.version, Version::parse("0.1.1").unwrap());
        assert_eq!(
            update.asset_name.as_deref(),
            Some("crypto-hud-v0.1.1-windows-x64-portable.zip")
        );
        assert_eq!(
            update.checksum_asset_name.as_deref(),
            Some("crypto-hud-v0.1.1-windows-x64-portable.zip.sha256")
        );
    }

    #[test]
    fn only_trusts_the_matching_project_release_page() {
        let mut update = update_from_release_json("0.1.0", &release_json("v0.1.1", false), false)
            .unwrap()
            .unwrap();

        assert_eq!(
            trusted_release_page_url(&update),
            Some("https://github.com/crypto-widget/crypto-hud/releases/tag/v0.1.1")
        );

        update.html_url = "https://example.com/crypto-hud/releases/tag/v0.1.1".to_string();
        assert_eq!(trusted_release_page_url(&update), None);

        update.html_url =
            "https://github.com/crypto-widget/crypto-hud/releases/tag/v0.1.2".to_string();
        assert_eq!(trusted_release_page_url(&update), None);
    }

    #[test]
    fn ignores_equal_or_older_release() {
        assert!(
            update_from_release_json("0.1.1", &release_json("v0.1.1", false), false)
                .unwrap()
                .is_none()
        );
        assert!(
            update_from_release_json("0.1.1", &release_json("v0.1.0", false), false)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn ignores_prerelease_unless_enabled() {
        assert!(
            update_from_release_json("0.1.0", &release_json("v0.2.0-alpha.1", true), false)
                .unwrap()
                .is_none()
        );

        let update = update_from_release_json("0.1.0", &release_json("v0.2.0-alpha.1", true), true)
            .unwrap()
            .unwrap();
        assert_eq!(update.version, Version::parse("0.2.0-alpha.1").unwrap());
    }

    #[test]
    fn env_flags_accept_common_truthy_values() {
        std::env::set_var("CRYPTO_HUD_TEST_FLAG", "yes");
        assert!(env_flag_enabled("CRYPTO_HUD_TEST_FLAG"));
        std::env::set_var("CRYPTO_HUD_TEST_FLAG", "0");
        assert!(!env_flag_enabled("CRYPTO_HUD_TEST_FLAG"));
        std::env::remove_var("CRYPTO_HUD_TEST_FLAG");
    }

    #[test]
    fn update_agent_accepts_http_and_socks_proxy_urls() {
        let direct_agent = build_agent(None).unwrap();
        assert!(direct_agent.config().proxy().is_none());
        assert!(build_agent(Some("http://127.0.0.1:7890")).is_ok());
        assert!(build_agent(Some("socks5://127.0.0.1:1080")).is_ok());
        assert!(build_agent(Some("socks5h://127.0.0.1:1080")).is_ok());
        assert!(build_agent(Some("ftp://127.0.0.1:21")).is_err());
    }

    #[test]
    fn update_response_reader_enforces_the_global_deadline() {
        assert_eq!(
            read_update_response(Cursor::new(b"{}"), None).unwrap(),
            "{}"
        );
        assert!(
            read_update_response(Cursor::new(b"{}"), Some(Instant::now()))
                .unwrap_err()
                .to_string()
                .contains("response body deadline exceeded")
        );
    }

    #[test]
    fn update_agent_preserves_proxy_side_dns_for_legacy_socks_schemes() {
        assert!(!configured_proxy("socks5://127.0.0.1:1080")
            .unwrap()
            .resolve_target());
        assert!(!configured_proxy("socks://127.0.0.1:1080")
            .unwrap()
            .resolve_target());
        assert!(!configured_proxy("socks5h://127.0.0.1:1080")
            .unwrap()
            .resolve_target());
    }

    #[test]
    fn default_update_config_has_no_proxy() {
        assert_eq!(UpdateCheckConfig::default().proxy_url, None);
    }
}
