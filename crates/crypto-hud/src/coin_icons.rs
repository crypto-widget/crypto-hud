use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use crypto_hud_core::normalize_symbol_token;
use serde::Deserialize;
use slint::Image;

const MAX_ICON_BYTES: usize = 512 * 1024;
const MAX_TOKENLIST_BYTES: usize = 12 * 1024 * 1024;
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
    generation: u64,
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
    chains: HashMap<&'static str, Option<HashMap<String, Vec<String>>>>,
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
                                generation: request.generation,
                            },
                            Err(error) => IconResult {
                                key: request.key,
                                path: None,
                                error: Some(error),
                                generation: request.generation,
                            },
                        };
                        if result_sender.send(icon_result).is_err() {
                            break;
                        }
                    }
                    WorkerCommand::ClearCache => {
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
            .map(|symbol| self.icon_for_symbol(symbol, proxy_url))
            .collect()
    }

    pub(crate) fn clear_cache(&self) -> Result<usize, String> {
        {
            let mut generation = self.generation.borrow_mut();
            *generation = generation.wrapping_add(1);
        }
        self.images.borrow_mut().clear();
        self.pending.borrow_mut().clear();
        self.discard_results();

        let deleted = remove_cached_icon_files(&self.cache_dir)?;
        self.requests
            .send(WorkerCommand::ClearCache)
            .map_err(|error| error.to_string())?;

        Ok(deleted)
    }

    fn icon_for_symbol(&self, symbol: &str, proxy_url: Option<&str>) -> Image {
        let Some(key) = icon_key_from_symbol(symbol) else {
            return Image::default();
        };

        if let Some(cached) = self.images.borrow().get(&key) {
            return cached.clone().unwrap_or_default();
        }

        if let Some(image) = load_cached_icon(&self.cache_dir, &key) {
            self.images
                .borrow_mut()
                .insert(key.clone(), Some(image.clone()));
            return image;
        }

        if self.pending.borrow_mut().insert(key.clone()) {
            let request = IconRequest {
                key: key.clone(),
                proxy_url: proxy_url.map(str::to_string),
                generation: *self.generation.borrow(),
            };
            if self.requests.send(WorkerCommand::Fetch(request)).is_err() {
                self.pending.borrow_mut().remove(&key);
                self.images.borrow_mut().insert(key, None);
            }
        }

        Image::default()
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
                        self.images
                            .borrow_mut()
                            .insert(result.key.clone(), Some(image));
                    }
                    Err(error) => {
                        eprintln!("failed to load cached coin icon {:?}: {error}", path);
                        self.images.borrow_mut().insert(result.key.clone(), None);
                    }
                },
                None => {
                    if let Some(error) = result.error {
                        eprintln!("failed to resolve coin icon {}: {error}", result.key);
                    }
                    self.images.borrow_mut().insert(result.key.clone(), None);
                }
            }
        }
    }

    fn discard_results(&self) {
        loop {
            match self.results.borrow_mut().try_recv() {
                Ok(_) => {}
                Err(_) => break,
            }
        }
    }
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
    let agent = http_agent(proxy_url)?;
    let candidates = icon_candidates(key);

    for candidate in candidates {
        match fetch_icon_bytes(&agent, &candidate.url, candidate.extension) {
            Ok(bytes) => {
                let path = cache_file_path(cache_dir, key, candidate.extension);
                persist_icon(&path, &bytes).map_err(|error| {
                    format!(
                        "failed to cache {:?} icon at {:?}: {error}",
                        candidate.source, path
                    )
                })?;
                return Ok(Some(path));
            }
            Err(_) => {}
        }
    }

    for url in trust_wallet_index.logo_urls_for_symbol(&agent, key) {
        let extension = icon_extension_from_url(&url);
        match fetch_icon_bytes(&agent, &url, extension) {
            Ok(bytes) => {
                let path = cache_file_path(cache_dir, key, extension);
                persist_icon(&path, &bytes).map_err(|error| {
                    format!(
                        "failed to cache TrustWallet token icon at {:?}: {error}",
                        path
                    )
                })?;
                return Ok(Some(path));
            }
            Err(_) => {}
        }
    }

    Ok(None)
}

fn http_agent(proxy_url: Option<&str>) -> Result<ureq::Agent, String> {
    let mut builder = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(8))
        .timeout_read(Duration::from_secs(12));

    if let Some(proxy_url) = proxy_url.filter(|value| !value.trim().is_empty()) {
        let proxy = ureq::Proxy::new(proxy_url).map_err(|error| error.to_string())?;
        builder = builder.proxy(proxy);
    }

    Ok(builder.build())
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
    let response = agent
        .get(url)
        .set("Accept", accept)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|error| error.to_string())?;

    let mut bytes = Vec::new();
    response
        .into_reader()
        .take((max_bytes + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;

    if bytes.len() > max_bytes {
        return Err("response is too large".to_string());
    }

    Ok(bytes)
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
    fn logo_urls_for_symbol(&mut self, agent: &ureq::Agent, key: &str) -> Vec<String> {
        let mut urls = Vec::new();

        for chain in TRUSTWALLET_TOKENLIST_CHAINS {
            if !self.chains.contains_key(chain) {
                let index = fetch_trustwallet_chain_index(agent, chain).ok();
                self.chains.insert(chain, index);
            }

            if let Some(Some(index)) = self.chains.get(chain) {
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
