use std::{
    env,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use semver::Version;
use serde::Deserialize;

const DEFAULT_RELEASE_API_URL: &str =
    "https://api.github.com/repos/crypto-widget/crypto-hud/releases/latest";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
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

pub fn install_latest_update(proxy_url: Option<String>) -> Result<()> {
    let mut config = UpdateCheckConfig::from_env().context("update checks are disabled")?;
    config.proxy_url = proxy_url;
    let handoff_script =
        update_handoff_script().context("failed to locate update handoff script")?;
    download_and_launch_latest_update(config, &handoff_script)
}

pub fn check_latest_release(config: &UpdateCheckConfig) -> Result<Option<UpdateInfo>> {
    let agent = build_agent(config.proxy_url.as_deref())?;
    let body = agent
        .get(&config.release_api_url)
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", USER_AGENT)
        .call()
        .with_context(|| format!("failed to check {}", config.release_api_url))?
        .into_string()
        .context("failed to read update response")?;

    update_from_release_json(&config.current_version, &body, config.include_prereleases)
}

fn download_and_launch_latest_update(
    config: UpdateCheckConfig,
    handoff_script: &Path,
) -> Result<()> {
    let update = check_latest_release(&config)?.context("no newer update is available")?;
    download_and_launch_update(&update, handoff_script, config.proxy_url.as_deref())
}

fn download_and_launch_update(
    update: &UpdateInfo,
    handoff_script: &Path,
    proxy_url: Option<&str>,
) -> Result<()> {
    let asset_url = update
        .asset_url
        .as_deref()
        .context("release has no package asset")?;
    let checksum_url = update
        .checksum_asset_url
        .as_deref()
        .context("release has no checksum asset")?;
    let tag = sanitize_tag(&update.tag_name);
    let download_dir = env::temp_dir().join(format!(
        "crypto-widget-in-app-update-{}-{tag}",
        std::process::id()
    ));
    fs::create_dir_all(&download_dir).with_context(|| {
        format!(
            "failed to create update download directory {}",
            download_dir.display()
        )
    })?;

    let package_path = download_dir.join(safe_asset_file_name(
        update.asset_name.as_deref(),
        "crypto-hud-update.zip",
    ));
    let checksum_path = download_dir.join(safe_asset_file_name(
        update.checksum_asset_name.as_deref(),
        "crypto-hud-update.zip.sha256",
    ));
    download_url_to_file(asset_url, &package_path, proxy_url)
        .with_context(|| format!("failed to download update package from {asset_url}"))?;
    download_url_to_file(checksum_url, &checksum_path, proxy_url)
        .with_context(|| format!("failed to download update checksum from {checksum_url}"))?;
    launch_update_handoff(handoff_script, &package_path, &checksum_path)
}

fn build_agent(proxy_url: Option<&str>) -> Result<ureq::Agent> {
    let mut builder = ureq::AgentBuilder::new()
        .timeout_connect(REQUEST_TIMEOUT)
        .timeout_read(REQUEST_TIMEOUT);
    if let Some(proxy_url) = proxy_url.and_then(non_empty_trimmed) {
        let proxy = ureq::Proxy::new(proxy_url).context("invalid proxy URL")?;
        builder = builder.proxy(proxy);
    }
    Ok(builder.build())
}

fn non_empty_trimmed(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn download_url_to_file(url: &str, path: &Path, proxy_url: Option<&str>) -> Result<()> {
    let agent = build_agent(proxy_url)?;
    let response = agent
        .get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .with_context(|| format!("failed to GET {url}"))?;
    let mut reader = response.into_reader();
    let mut file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    io::copy(&mut reader, &mut file)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn launch_update_handoff(
    handoff_script: &Path,
    package_path: &Path,
    checksum_path: &Path,
) -> Result<()> {
    let launcher_path = package_path
        .parent()
        .context("update package path has no parent directory")?
        .join("launch-update.ps1");
    fs::write(
        &launcher_path,
        update_launcher_script(
            handoff_script,
            package_path,
            checksum_path,
            &installed_app_dir()?,
            std::process::id(),
        ),
    )
    .with_context(|| format!("failed to write {}", launcher_path.display()))?;

    let mut command = Command::new("powershell");
    command
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&launcher_path);
    hide_command_window(&mut command);
    command
        .spawn()
        .with_context(|| format!("failed to start {}", launcher_path.display()))?;
    Ok(())
}

fn update_launcher_script(
    handoff_script: &Path,
    package_path: &Path,
    checksum_path: &Path,
    install_dir: &Path,
    process_id: u32,
) -> String {
    format!(
        r#"$ErrorActionPreference = "Stop"
Wait-Process -Id {process_id} -ErrorAction SilentlyContinue
powershell -NoProfile -ExecutionPolicy Bypass -File {handoff_script} -PackageZip {package_path} -ChecksumPath {checksum_path} -InstallDir {install_dir} -StartAfterInstall
Remove-Item -LiteralPath $MyInvocation.MyCommand.Path -Force -ErrorAction SilentlyContinue
"#,
        handoff_script = ps_single_quoted_path(handoff_script),
        package_path = ps_single_quoted_path(package_path),
        checksum_path = ps_single_quoted_path(checksum_path),
        install_dir = ps_single_quoted_path(install_dir),
    )
}

fn installed_app_dir() -> Result<PathBuf> {
    env::current_exe()
        .context("failed to resolve current executable path")?
        .parent()
        .map(Path::to_path_buf)
        .context("current executable path has no parent directory")
}

fn update_handoff_script() -> Result<PathBuf> {
    let candidates = update_handoff_script_candidates();
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .context("install-update-package.ps1 was not found")
}

fn update_handoff_script_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = env_value_with_legacy(
        "CRYPTO_HUD_UPDATE_HANDOFF_SCRIPT",
        &["CRYPTO_WIDGET_UPDATE_HANDOFF_SCRIPT"],
    ) {
        if !path.trim().is_empty() {
            candidates.push(PathBuf::from(path));
        }
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("install-update-package.ps1"));
        }
    }
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join("scripts").join("install-update-package.ps1"));
    }
    candidates
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

fn safe_asset_file_name(name: Option<&str>, fallback: &str) -> String {
    name.and_then(|name| {
        Path::new(name)
            .file_name()
            .and_then(|file_name| file_name.to_str())
            .filter(|file_name| *file_name == name && !file_name.trim().is_empty())
            .map(ToOwned::to_owned)
    })
    .unwrap_or_else(|| fallback.to_string())
}

fn sanitize_tag(tag: &str) -> String {
    let sanitized = tag
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "unknown".to_string()
    } else {
        sanitized
    }
}

fn ps_single_quoted_path(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "''"))
}

#[cfg(windows)]
fn hide_command_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_command_window(_: &mut Command) {}

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

    fn release_json(tag: &str, prerelease: bool) -> String {
        format!(
            r#"{{
              "tag_name": "{tag}",
              "html_url": "https://github.com/crypto-widget/crypto-hud/releases/tag/{tag}",
              "draft": false,
              "prerelease": {prerelease},
              "assets": [
                {{
                  "name": "crypto-hud-{tag}-windows-x64.zip",
                  "browser_download_url": "https://example.test/{tag}.zip"
                }},
                {{
                  "name": "crypto-hud-{tag}-windows-x64.zip.sha256",
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
            Some("crypto-hud-v0.1.1-windows-x64.zip")
        );
        assert_eq!(
            update.checksum_asset_name.as_deref(),
            Some("crypto-hud-v0.1.1-windows-x64.zip.sha256")
        );
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
    fn asset_file_name_rejects_path_segments() {
        assert_eq!(
            safe_asset_file_name(Some("crypto-hud.zip"), "fallback.zip"),
            "crypto-hud.zip"
        );
        assert_eq!(
            safe_asset_file_name(Some("../crypto-hud.zip"), "fallback.zip"),
            "fallback.zip"
        );
        assert_eq!(
            safe_asset_file_name(Some("nested/crypto-hud.zip"), "fallback.zip"),
            "fallback.zip"
        );
        assert_eq!(safe_asset_file_name(None, "fallback.zip"), "fallback.zip");
    }

    #[test]
    fn launcher_script_waits_then_runs_handoff() {
        let script = update_launcher_script(
            Path::new(r"C:\Install Path\install-update-package.ps1"),
            Path::new(r"C:\Temp\crypto-hud.zip"),
            Path::new(r"C:\Temp\crypto-hud.zip.sha256"),
            Path::new(r"C:\Install Path"),
            42,
        );

        assert!(script.contains("Wait-Process -Id 42"));
        assert!(script.contains("-PackageZip 'C:\\Temp\\crypto-hud.zip'"));
        assert!(script.contains("-ChecksumPath 'C:\\Temp\\crypto-hud.zip.sha256'"));
        assert!(script.contains("-InstallDir 'C:\\Install Path'"));
        assert!(script.contains("-StartAfterInstall"));
    }

    #[test]
    fn update_agent_accepts_http_and_socks_proxy_urls() {
        assert!(build_agent(None).is_ok());
        assert!(build_agent(Some("http://127.0.0.1:7890")).is_ok());
        assert!(build_agent(Some("socks5://127.0.0.1:1080")).is_ok());
        assert!(build_agent(Some("ftp://127.0.0.1:21")).is_err());
    }

    #[test]
    fn default_update_config_has_no_proxy() {
        assert_eq!(UpdateCheckConfig::default().proxy_url, None);
    }
}
