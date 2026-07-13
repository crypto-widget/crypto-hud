use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

use crypto_hud_core::normalize_symbol_token;
use serde::Deserialize;
use slint::Image;

use crate::feature_flags;

const MAX_ICON_BYTES: usize = 512 * 1024;
const MAX_TOKENLIST_BYTES: usize = 12 * 1024 * 1024;
const ICON_RETRY_DELAY: Duration = Duration::from_secs(5 * 60);
const TOKENLIST_RETRY_DELAY: Duration = Duration::from_secs(5 * 60);
const USER_AGENT: &str = concat!("crypto-hud/", env!("CARGO_PKG_VERSION"));
const TRUSTWALLET_TOKENLIST_CHAINS: &[&str] = &[
    "ethereum",
    "smartchain",
    "polygon",
    "solana",
    "arbitrum",
    "optimism",
    "base",
    "avalanchec",
    "tron",
];

pub(crate) struct CoinIconRegistry {
    cache_dir: PathBuf,
    images: RefCell<HashMap<String, Option<Image>>>,
    pending: RefCell<HashSet<String>>,
    failures: RefCell<HashMap<String, IconFailure>>,
    generation: RefCell<u64>,
    requests: Sender<WorkerCommand>,
    results: RefCell<Receiver<IconResult>>,
}

struct IconRequest {
    key: String,
    proxy_url: Option<String>,
    generation: u64,
}

struct IconResult {
    key: String,
    path: Option<PathBuf>,
    error: Option<String>,
    proxy_url: Option<String>,
    generation: u64,
}

#[derive(Debug, Clone)]
struct IconFailure {
    proxy_url: Option<String>,
    retry_at: Instant,
}

enum WorkerCommand {
    Fetch(IconRequest),
    ClearCache,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IconSource {
    SpotHq,
    Iconify,
    TrustWallet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IconCandidate {
    source: IconSource,
    url: String,
    extension: &'static str,
}

#[derive(Default)]
struct TrustWalletTokenIndex {
    proxy_url: Option<Option<String>>,
    chains: HashMap<&'static str, TrustWalletChainIndex>,
}

enum TrustWalletChainIndex {
    Ready(HashMap<String, Vec<String>>),
    Failed { retry_at: Instant },
}

#[derive(Deserialize)]
struct TrustWalletTokenList {
    tokens: Vec<TrustWalletToken>,
}

#[derive(Deserialize)]
struct TrustWalletToken {
    symbol: String,
    #[serde(rename = "logoURI")]
    logo_uri: Option<String>,
}

impl CoinIconRegistry {
    pub(crate) fn new(cache_dir: PathBuf) -> Self {
        if let Err(error) = fs::create_dir_all(&cache_dir) {
            eprintln!("failed to create coin icon cache {:?}: {error}", cache_dir);
        }

        let (request_sender, request_receiver) = mpsc::channel::<WorkerCommand>();
        let (result_sender, result_receiver) = mpsc::channel::<IconResult>();
        let worker_cache_dir = cache_dir.clone();
        let _ = thread::spawn(move || {
            let mut trust_wallet_index = TrustWalletTokenIndex::default();
            while let Ok(command) = request_receiver.recv() {
                match command {
                    WorkerCommand::Fetch(request) => {
                        let proxy_url = request.proxy_url.clone();
                        let result = download_icon(
                            &worker_cache_dir,
                            &request.key,
                            request.proxy_url.as_deref(),
                            &mut trust_wallet_index,
                        );
                        let icon_result = match result {
                            Ok(path) => IconResult {
                                key: request.key,
                                path,
                                error: None,
                                proxy_url,
                                generation: request.generation,
                            },
                            Err(error) => IconResult {
                                key: request.key,
                                path: None,
                                error: Some(error),
                                proxy_url,
                                generation: request.generation,
                            },
                        };
                        if result_sender.send(icon_result).is_err() {
                            break;
                        }
                    }
                    WorkerCommand::ClearCache => {
                        trust_wallet_index.clear();
                        if let Err(error) = remove_cached_icon_files(&worker_cache_dir) {
                            eprintln!("failed to clear coin icon cache in worker: {error}");
                        }
                    }
                }
            }
        });

        Self {
            cache_dir,
            images: RefCell::new(HashMap::new()),
            pending: RefCell::new(HashSet::new()),
            failures: RefCell::new(HashMap::new()),
            generation: RefCell::new(0),
            requests: request_sender,
            results: RefCell::new(result_receiver),
        }
    }

    pub(crate) fn icons_for_symbols(
        &self,
        symbols: &[String],
        proxy_url: Option<&str>,
    ) -> Vec<Image> {
        self.drain_results();
        symbols
            .iter()
            .map(|symbol| self.icon_for_symbol_with_ready(symbol, proxy_url).0)
            .collect()
    }

    pub(crate) fn icon_ready_for_symbols(
        &self,
        symbols: &[String],
        proxy_url: Option<&str>,
    ) -> Vec<bool> {
        self.drain_results();
        symbols
            .iter()
            .map(|symbol| self.icon_for_symbol_with_ready(symbol, proxy_url).1)
            .collect()
    }

    pub(crate) fn clear_cache(&self) -> Result<usize, String> {
        {
            let mut generation = self.generation.borrow_mut();
            *generation = generation.wrapping_add(1);
        }
        self.images.borrow_mut().clear();
        self.pending.borrow_mut().clear();
        self.failures.borrow_mut().clear();
        self.discard_results();

        let deleted = remove_cached_icon_files(&self.cache_dir)?;
        self.requests
            .send(WorkerCommand::ClearCache)
            .map_err(|error| error.to_string())?;

        Ok(deleted)
    }

    fn icon_for_symbol_with_ready(&self, symbol: &str, proxy_url: Option<&str>) -> (Image, bool) {
        let Some(key) = icon_key_from_symbol(symbol) else {
            return (Image::default(), false);
        };

        if let Some(Some(image)) = self.images.borrow().get(&key) {
            return (image.clone(), true);
        }
        let proxy_url = normalized_proxy_url(proxy_url);
        if self.images.borrow().get(&key).is_some_and(Option::is_none) {
            let retry_allowed = self.failures.borrow().get(&key).is_none_or(|failure| {
                icon_retry_allowed(failure, proxy_url.as_deref(), Instant::now())
            });
            if !retry_allowed {
                return (Image::default(), false);
            }
            self.images.borrow_mut().remove(&key);
            self.failures.borrow_mut().remove(&key);
        }

        if let Some(image) = load_cached_icon(&self.cache_dir, &key) {
            self.images
                .borrow_mut()
                .insert(key.clone(), Some(image.clone()));
            return (image, true);
        }

        if self.pending.borrow_mut().insert(key.clone()) {
            let request = IconRequest {
                key: key.clone(),
                proxy_url: proxy_url.clone(),
                generation: *self.generation.borrow(),
            };
            let request_proxy_url = request.proxy_url.clone();
            if self.requests.send(WorkerCommand::Fetch(request)).is_err() {
                self.pending.borrow_mut().remove(&key);
                self.images.borrow_mut().insert(key.clone(), None);
                self.record_failure(key, request_proxy_url);
            }
        }

        (Image::default(), false)
    }

    fn drain_results(&self) {
        loop {
            let result = match self.results.borrow_mut().try_recv() {
                Ok(result) => result,
                Err(_) => break,
            };
            if result.generation != *self.generation.borrow() {
                continue;
            }
            self.pending.borrow_mut().remove(&result.key);
            match result.path {
                Some(path) => match Image::load_from_path(&path) {
                    Ok(image) => {
                        self.failures.borrow_mut().remove(&result.key);
                        self.images
                            .borrow_mut()
                            .insert(result.key.clone(), Some(image));
                    }
                    Err(error) => {
                        eprintln!("failed to load cached coin icon {:?}: {error}", path);
                        self.images.borrow_mut().insert(result.key.clone(), None);
                        self.record_failure(result.key.clone(), result.proxy_url.clone());
                    }
                },
                None => {
                    if let Some(error) = result.error {
                        eprintln!("failed to resolve coin icon {}: {error}", result.key);
                    }
                    self.images.borrow_mut().insert(result.key.clone(), None);
                    self.record_failure(result.key.clone(), result.proxy_url.clone());
                }
            }
        }
    }

    fn discard_results(&self) {
        while self.results.borrow_mut().try_recv().is_ok() {}
    }

    fn record_failure(&self, key: String, proxy_url: Option<String>) {
        self.failures.borrow_mut().insert(
            key,
            IconFailure {
                proxy_url,
                retry_at: Instant::now() + ICON_RETRY_DELAY,
            },
        );
    }
}

fn normalized_proxy_url(proxy_url: Option<&str>) -> Option<String> {
    proxy_url.and_then(|proxy_url| {
        let proxy_url = proxy_url.trim();
        (!proxy_url.is_empty()).then(|| proxy_url.to_string())
    })
}

fn icon_retry_allowed(failure: &IconFailure, proxy_url: Option<&str>, now: Instant) -> bool {
    failure.proxy_url.as_deref() != proxy_url || now >= failure.retry_at
}

fn icon_key_from_symbol(symbol: &str) -> Option<String> {
    normalize_symbol_token(symbol).and_then(|asset| icon_key_from_asset(&asset))
}

fn icon_key_from_asset(asset: &str) -> Option<String> {
    let key = asset.trim().to_ascii_lowercase();
    if key.chars().all(|ch| ch.is_ascii_alphanumeric()) && !key.is_empty() {
        Some(key)
    } else {
        None
    }
}

fn load_cached_icon(cache_dir: &Path, key: &str) -> Option<Image> {
    for path in cached_icon_paths(cache_dir, key) {
        match Image::load_from_path(&path) {
            Ok(image) => return Some(image),
            Err(error) => {
                eprintln!("failed to load cached coin icon {:?}: {error}", path);
                let _ = fs::remove_file(path);
            }
        }
    }
    None
}

fn cached_icon_paths(cache_dir: &Path, key: &str) -> Vec<PathBuf> {
    ["svg", "png"]
        .into_iter()
        .map(|extension| cache_file_path(cache_dir, key, extension))
        .filter(|path| path.is_file())
        .collect()
}

fn remove_cached_icon_files(cache_dir: &Path) -> Result<usize, String> {
    let entries = match fs::read_dir(cache_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.to_string()),
    };
    let mut deleted = 0;

    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_file() && is_cached_icon_file(&path) {
            fs::remove_file(&path).map_err(|error| error.to_string())?;
            deleted += 1;
        }
    }

    Ok(deleted)
}

fn is_cached_icon_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|file_name| file_name.to_str())
        .map(|file_name| {
            file_name.ends_with(".svg")
                || file_name.ends_with(".png")
                || file_name.ends_with(".svg.tmp")
                || file_name.ends_with(".png.tmp")
        })
        .unwrap_or(false)
}

fn download_icon(
    cache_dir: &Path,
    key: &str,
    proxy_url: Option<&str>,
    trust_wallet_index: &mut TrustWalletTokenIndex,
) -> Result<Option<PathBuf>, String> {
    if feature_flags::gui_smoke_offline_network_disabled() {
        return Ok(None);
    }

    let agent = http_agent(proxy_url)?;
    let candidates = icon_candidates(key);

    for candidate in candidates {
        if let Ok(bytes) = fetch_icon_bytes(&agent, &candidate.url, candidate.extension) {
            let path = cache_file_path(cache_dir, key, candidate.extension);
            persist_icon(&path, &bytes).map_err(|error| {
                format!(
                    "failed to cache {:?} icon at {:?}: {error}",
                    candidate.source, path
                )
            })?;
            return Ok(Some(path));
        }
    }

    for url in trust_wallet_index.logo_urls_for_symbol(&agent, key, proxy_url) {
        let extension = icon_extension_from_url(&url);
        if let Ok(bytes) = fetch_icon_bytes(&agent, &url, extension) {
            let path = cache_file_path(cache_dir, key, extension);
            persist_icon(&path, &bytes).map_err(|error| {
                format!(
                    "failed to cache TrustWallet token icon at {:?}: {error}",
                    path
                )
            })?;
            return Ok(Some(path));
        }
    }

    Ok(None)
}

fn http_agent(proxy_url: Option<&str>) -> Result<ureq::Agent, String> {
    let proxy = proxy_url
        .filter(|value| !value.trim().is_empty())
        .map(configured_proxy)
        .transpose()
        .map_err(|error| error.to_string())?;
    let config = ureq::Agent::config_builder()
        // Keep proxying controlled by the application instead of ureq's environment defaults.
        .proxy(proxy)
        .timeout_global(Some(Duration::from_secs(12)))
        .timeout_resolve(Some(Duration::from_secs(8)))
        .timeout_connect(Some(Duration::from_secs(8)))
        .timeout_send_request(Some(Duration::from_secs(12)))
        .timeout_recv_response(Some(Duration::from_secs(12)))
        .timeout_recv_body(Some(Duration::from_secs(12)))
        .build();

    Ok(config.into())
}

fn configured_proxy(proxy_url: &str) -> Result<ureq::Proxy, ureq::Error> {
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

fn fetch_icon_bytes(agent: &ureq::Agent, url: &str, extension: &str) -> Result<Vec<u8>, String> {
    let bytes = fetch_bytes(agent, url, accept_header(extension), MAX_ICON_BYTES)?;

    if !looks_like_icon(&bytes, extension) {
        return Err("icon response did not match the expected file type".to_string());
    }

    Ok(bytes)
}

fn fetch_bytes(
    agent: &ureq::Agent,
    url: &str,
    accept: &str,
    max_bytes: usize,
) -> Result<Vec<u8>, String> {
    let deadline = request_deadline(agent);
    let mut response = agent
        .get(url)
        .header("Accept", accept)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|error| error.to_string())?;

    read_bytes_with_deadline(response.body_mut().as_reader(), max_bytes, deadline)
}

fn request_deadline(agent: &ureq::Agent) -> Option<Instant> {
    agent
        .config()
        .timeouts()
        .global
        .and_then(|timeout| Instant::now().checked_add(timeout))
}

fn read_bytes_with_deadline(
    mut reader: impl Read,
    max_bytes: usize,
    deadline: Option<Instant>,
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8 * 1024];
    let read_limit = max_bytes.saturating_add(1);

    loop {
        ensure_response_deadline(deadline)?;
        let remaining = read_limit.saturating_sub(bytes.len());
        if remaining == 0 {
            return Err("response is too large".to_string());
        }
        let chunk_len = remaining.min(buffer.len());
        let read = reader
            .read(&mut buffer[..chunk_len])
            .map_err(|error| error.to_string())?;
        ensure_response_deadline(deadline)?;
        if read == 0 {
            return Ok(bytes);
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.len() > max_bytes {
            return Err("response is too large".to_string());
        }
    }
}

fn ensure_response_deadline(deadline: Option<Instant>) -> Result<(), String> {
    if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
        return Err("response body deadline exceeded".to_string());
    }
    Ok(())
}

fn accept_header(extension: &str) -> &'static str {
    match extension {
        "svg" => "image/svg+xml",
        "png" => "image/png",
        _ => "image/*",
    }
}

fn looks_like_icon(bytes: &[u8], extension: &str) -> bool {
    match extension {
        "svg" => {
            let head = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]);
            head.to_ascii_lowercase().contains("<svg")
        }
        "png" => bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
        _ => false,
    }
}

fn persist_icon(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        return Ok(());
    }

    let temp_path = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("icon")
    ));
    let _ = fs::remove_file(&temp_path);
    fs::write(&temp_path, bytes)?;
    fs::rename(&temp_path, path)
}

fn cache_file_path(cache_dir: &Path, key: &str, extension: &str) -> PathBuf {
    cache_dir.join(format!("{key}.{extension}"))
}

fn icon_extension_from_url(url: &str) -> &'static str {
    url.split(['?', '#'])
        .next()
        .and_then(|path| path.rsplit('.').next())
        .map(str::to_ascii_lowercase)
        .as_deref()
        .and_then(|extension| match extension {
            "svg" => Some("svg"),
            "png" => Some("png"),
            _ => None,
        })
        .unwrap_or("png")
}

fn icon_candidates(key: &str) -> Vec<IconCandidate> {
    let mut candidates = vec![
        IconCandidate {
            source: IconSource::SpotHq,
            url: format!(
                "https://cdn.jsdelivr.net/gh/spothq/cryptocurrency-icons@master/svg/color/{key}.svg"
            ),
            extension: "svg",
        },
        IconCandidate {
            source: IconSource::Iconify,
            url: format!("https://api.iconify.design/cryptocurrency/{key}.svg"),
            extension: "svg",
        },
    ];

    if let Some(chain) = trustwallet_native_chain(key) {
        candidates.push(IconCandidate {
            source: IconSource::TrustWallet,
            url: format!(
                "https://raw.githubusercontent.com/trustwallet/assets/master/blockchains/{chain}/info/logo.png"
            ),
            extension: "png",
        });
    }

    candidates
}

fn trustwallet_native_chain(key: &str) -> Option<&'static str> {
    match key {
        "ada" => Some("cardano"),
        "atom" => Some("cosmos"),
        "avax" => Some("avalanchec"),
        "bch" => Some("bitcoincash"),
        "bnb" => Some("smartchain"),
        "btc" => Some("bitcoin"),
        "doge" => Some("dogecoin"),
        "dot" => Some("polkadot"),
        "eth" => Some("ethereum"),
        "etc" => Some("classic"),
        "fil" => Some("filecoin"),
        "ltc" => Some("litecoin"),
        "matic" | "pol" => Some("polygon"),
        "near" => Some("near"),
        "sol" => Some("solana"),
        "sui" => Some("sui"),
        "ton" => Some("ton"),
        "trx" => Some("tron"),
        "xrp" => Some("xrp"),
        "xtz" => Some("tezos"),
        "zec" => Some("zcash"),
        _ => None,
    }
}

impl TrustWalletTokenIndex {
    fn clear(&mut self) {
        self.proxy_url = None;
        self.chains.clear();
    }

    fn reset_for_proxy(&mut self, proxy_url: Option<&str>) {
        let proxy_url = normalized_proxy_url(proxy_url);
        if self.proxy_url.as_ref() != Some(&proxy_url) {
            self.proxy_url = Some(proxy_url);
            self.chains.clear();
        }
    }

    fn logo_urls_for_symbol(
        &mut self,
        agent: &ureq::Agent,
        key: &str,
        proxy_url: Option<&str>,
    ) -> Vec<String> {
        self.logo_urls_for_symbol_at(agent, key, proxy_url, Instant::now())
    }

    fn logo_urls_for_symbol_at(
        &mut self,
        agent: &ureq::Agent,
        key: &str,
        proxy_url: Option<&str>,
        now: Instant,
    ) -> Vec<String> {
        self.reset_for_proxy(proxy_url);
        let mut urls = Vec::new();

        for chain in TRUSTWALLET_TOKENLIST_CHAINS {
            let should_fetch = trustwallet_chain_index_needs_fetch(self.chains.get(chain), now);
            if should_fetch {
                let index = match fetch_trustwallet_chain_index(agent, chain) {
                    Ok(index) => TrustWalletChainIndex::Ready(index),
                    Err(_) => TrustWalletChainIndex::Failed {
                        retry_at: now + TOKENLIST_RETRY_DELAY,
                    },
                };
                self.chains.insert(chain, index);
            }

            if let Some(TrustWalletChainIndex::Ready(index)) = self.chains.get(chain) {
                if let Some(chain_urls) = index.get(key) {
                    urls.extend(chain_urls.iter().cloned());
                }
            }

            if urls.len() >= 3 {
                break;
            }
        }

        urls.truncate(3);
        urls
    }
}

fn trustwallet_chain_index_needs_fetch(
    index: Option<&TrustWalletChainIndex>,
    now: Instant,
) -> bool {
    match index {
        None => true,
        Some(TrustWalletChainIndex::Ready(_)) => false,
        Some(TrustWalletChainIndex::Failed { retry_at }) => now >= *retry_at,
    }
}

fn fetch_trustwallet_chain_index(
    agent: &ureq::Agent,
    chain: &'static str,
) -> Result<HashMap<String, Vec<String>>, String> {
    let url = format!(
        "https://raw.githubusercontent.com/trustwallet/assets/master/blockchains/{chain}/tokenlist.json"
    );
    let bytes = fetch_bytes(agent, &url, "application/json", MAX_TOKENLIST_BYTES)?;
    parse_trustwallet_tokenlist(&bytes)
}

fn parse_trustwallet_tokenlist(bytes: &[u8]) -> Result<HashMap<String, Vec<String>>, String> {
    let token_list =
        serde_json::from_slice::<TrustWalletTokenList>(bytes).map_err(|error| error.to_string())?;
    let mut index = HashMap::<String, Vec<String>>::new();

    for token in token_list.tokens {
        let Some(key) = icon_key_from_asset(&token.symbol) else {
            continue;
        };
        let Some(logo_uri) = token
            .logo_uri
            .filter(|url| trusted_trustwallet_logo_url(url))
        else {
            continue;
        };
        index.entry(key).or_default().push(logo_uri);
    }

    Ok(index)
}

fn trusted_trustwallet_logo_url(url: &str) -> bool {
    url.starts_with("https://assets-cdn.trustwallet.com/")
        || url.starts_with("https://raw.githubusercontent.com/trustwallet/assets/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn icon_agent_disables_environment_proxy_discovery() {
        let agent = http_agent(None).unwrap();

        assert!(agent.config().proxy().is_none());
    }

    #[test]
    fn icon_response_reader_enforces_size_and_deadline_limits() {
        assert_eq!(
            read_bytes_with_deadline(Cursor::new(b"icon"), 4, None).unwrap(),
            b"icon"
        );
        assert_eq!(
            read_bytes_with_deadline(Cursor::new(b"large"), 4, None).unwrap_err(),
            "response is too large"
        );
        assert_eq!(
            read_bytes_with_deadline(Cursor::new(b"icon"), 4, Some(Instant::now())).unwrap_err(),
            "response body deadline exceeded"
        );
    }

    #[test]
    fn icon_key_uses_market_pair_base_asset() {
        assert_eq!(
            icon_key_from_symbol("binance:spot:BTC/USDT").as_deref(),
            Some("btc")
        );
        assert_eq!(icon_key_from_symbol("ETH/USDT").as_deref(), Some("eth"));
        assert_eq!(icon_key_from_symbol("1INCH/USDT").as_deref(), Some("1inch"));
    }

    #[test]
    fn failed_icons_retry_after_delay_or_proxy_change() {
        let now = Instant::now();
        let failure = IconFailure {
            proxy_url: Some("http://127.0.0.1:7890".to_string()),
            retry_at: now + ICON_RETRY_DELAY,
        };

        assert!(!icon_retry_allowed(
            &failure,
            Some("http://127.0.0.1:7890"),
            now
        ));
        assert!(icon_retry_allowed(
            &failure,
            Some("socks5://127.0.0.1:1080"),
            now
        ));
        assert!(icon_retry_allowed(
            &failure,
            Some("http://127.0.0.1:7890"),
            now + ICON_RETRY_DELAY
        ));
    }

    #[test]
    fn failed_trustwallet_chain_index_retries_only_after_expiry() {
        let now = Instant::now();
        let failed = TrustWalletChainIndex::Failed {
            retry_at: now + TOKENLIST_RETRY_DELAY,
        };

        assert!(!trustwallet_chain_index_needs_fetch(Some(&failed), now));
        assert!(trustwallet_chain_index_needs_fetch(
            Some(&failed),
            now + TOKENLIST_RETRY_DELAY
        ));
        assert!(trustwallet_chain_index_needs_fetch(None, now));
    }

    #[test]
    fn trustwallet_index_resets_on_proxy_change_and_cache_clear() {
        let mut index = TrustWalletTokenIndex::default();
        index.reset_for_proxy(Some("http://127.0.0.1:7890"));
        index.chains.insert(
            "ethereum",
            TrustWalletChainIndex::Ready(HashMap::from([(
                "token".to_string(),
                vec!["https://assets-cdn.trustwallet.com/token.png".to_string()],
            )])),
        );

        index.reset_for_proxy(Some("socks5://127.0.0.1:1080"));
        assert!(index.chains.is_empty());

        index.chains.insert(
            "ethereum",
            TrustWalletChainIndex::Failed {
                retry_at: Instant::now() + TOKENLIST_RETRY_DELAY,
            },
        );
        index.clear();
        assert!(index.chains.is_empty());
        assert_eq!(index.proxy_url, None);
    }

    #[test]
    fn candidates_try_cc0_sources_before_trustwallet() {
        let candidates = icon_candidates("btc");

        assert_eq!(candidates[0].source, IconSource::SpotHq);
        assert_eq!(candidates[1].source, IconSource::Iconify);
        assert_eq!(candidates[2].source, IconSource::TrustWallet);
        assert!(candidates[2]
            .url
            .ends_with("/blockchains/bitcoin/info/logo.png"));
    }

    #[test]
    fn candidates_skip_trustwallet_without_native_chain_mapping() {
        let candidates = icon_candidates("pepe");

        assert_eq!(candidates.len(), 2);
        assert!(candidates
            .iter()
            .all(|candidate| candidate.extension == "svg"));
    }

    #[test]
    fn trustwallet_tokenlist_indexes_logo_urls_by_symbol() {
        let index = parse_trustwallet_tokenlist(
            br#"{
                "tokens": [
                    {
                        "symbol": "1INCH",
                        "logoURI": "https://assets-cdn.trustwallet.com/blockchains/ethereum/assets/0x111111111117dC0aa78b770fA6A738034120C302/logo.png"
                    },
                    {
                        "symbol": "BAD",
                        "logoURI": "https://example.com/bad.png"
                    }
                ]
            }"#,
        )
        .expect("parse tokenlist");

        assert_eq!(
            index
                .get("1inch")
                .and_then(|urls| urls.first())
                .map(String::as_str),
            Some("https://assets-cdn.trustwallet.com/blockchains/ethereum/assets/0x111111111117dC0aa78b770fA6A738034120C302/logo.png")
        );
        assert!(!index.contains_key("bad"));
    }

    #[test]
    fn trustwallet_logo_extension_defaults_to_png() {
        assert_eq!(
            icon_extension_from_url("https://assets-cdn.trustwallet.com/token/logo.svg?x=1"),
            "svg"
        );
        assert_eq!(
            icon_extension_from_url("https://assets-cdn.trustwallet.com/token/logo"),
            "png"
        );
    }

    #[test]
    fn cached_icon_paths_prefer_svg_before_png() {
        let dir =
            std::env::temp_dir().join(format!("crypto-hud-coin-icon-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        fs::write(dir.join("btc.png"), b"png").expect("write png");
        fs::write(dir.join("btc.svg"), b"svg").expect("write svg");

        let paths = cached_icon_paths(&dir, "btc");

        assert_eq!(paths[0], dir.join("btc.svg"));
        assert_eq!(paths[1], dir.join("btc.png"));

        fs::remove_dir_all(dir).expect("remove temp dir");
    }

    #[test]
    fn remove_cached_icon_files_deletes_icons_and_temp_files_only() {
        let dir =
            std::env::temp_dir().join(format!("crypto-hud-clear-icon-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        fs::write(dir.join("btc.svg"), b"svg").expect("write svg");
        fs::write(dir.join("eth.png"), b"png").expect("write png");
        fs::write(dir.join("sol.svg.tmp"), b"tmp").expect("write temp");
        fs::write(dir.join("notes.txt"), b"keep").expect("write txt");

        let deleted = remove_cached_icon_files(&dir).expect("remove icons");

        assert_eq!(deleted, 3);
        assert!(!dir.join("btc.svg").exists());
        assert!(!dir.join("eth.png").exists());
        assert!(!dir.join("sol.svg.tmp").exists());
        assert!(dir.join("notes.txt").exists());

        fs::remove_dir_all(dir).expect("remove temp dir");
    }

    #[test]
    fn icon_type_sniffing_rejects_html_errors() {
        assert!(looks_like_icon(br#"<svg viewBox="0 0 1 1"></svg>"#, "svg"));
        assert!(looks_like_icon(
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1],
            "png"
        ));
        assert!(!looks_like_icon(b"<html>not found</html>", "svg"));
    }
}
