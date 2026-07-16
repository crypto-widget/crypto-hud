use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt, fs,
    hash::{Hash, Hasher},
    io,
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::UNIX_EPOCH,
};

use anyhow::{anyhow, bail, Context, Result};
use crypto_hud_core::{default_market_symbols, normalize_market_pair_key};
pub use crypto_hud_runtime::{
    parse_manifest, validate_manifest, PluginDataRequirement, PluginManifest, PluginParameter,
    PluginSize, PluginSizePolicy, PluginTheme, PluginThemeRole, HOST_PLUGIN_API_VERSION,
    MAX_PREVIEW_IMAGES, MIN_SYMBOL_LIMIT,
};
use i_slint_compiler::{
    diagnostics::BuildDiagnostics,
    parser::{self, SyntaxKind, SyntaxNode},
    EmbedResourcesKind,
};
use i_slint_core::InternalToken;
use semver::{Version, VersionReq};
use slint_interpreter::{Compiler, ComponentDefinition, ValueType};

pub use crypto_hud_shell_state::{BUILTIN_MINI_TICKER_PLUGIN_ID, BUILTIN_QUOTE_BOARD_PLUGIN_ID};

pub const MANIFEST_FILE_NAME: &str = "widget.json";
pub const USER_PLUGIN_DEVELOPMENT_GUIDE_FILE_NAME: &str = "CUSTOM_UI_PLUGIN_DEVELOPMENT.md";
pub const MANIFEST_MAX_BYTES: u64 = 64 * 1024;
pub const SLINT_FILE_MAX_BYTES: u64 = 256 * 1024;
pub const ASSET_MAX_BYTES: u64 = 1024 * 1024;
pub const PLUGIN_DIR_MAX_BYTES: u64 = 5 * 1024 * 1024;
pub const PLUGIN_MANIFEST_SCHEMA_VERSION: u32 = 3;
const PLUGIN_SCAN_MAX_CANDIDATES: usize = 256;
const PLUGIN_SCAN_MAX_ENTRIES: usize = 8_192;
const PLUGIN_SCAN_MAX_HASH_BYTES: u64 = 64 * 1024 * 1024;
const PLUGIN_SCAN_MAX_ENTRIES_PER_CANDIDATE: usize = 4_096;
pub(crate) const SLINT_RENDERER_UNCOMPILED_REASON: &str = "Slint renderer has not been compiled";
const USER_PLUGIN_DEVELOPMENT_GUIDE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../CUSTOM_UI_PLUGIN_DEVELOPMENT.md"
));
#[cfg(test)]
const REPO_PLUGIN_DEVELOPMENT_GUIDE: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/plugins/README.md"));
const HIDDEN_MARKET_PLUGIN_IDS: &[&str] = &[
    BUILTIN_MINI_TICKER_PLUGIN_ID,
    "com.cryptohud.market-board",
    "com.example.stage3-price-card",
];
const DISABLED_PROTOTYPE_PLUGIN_IDS: &[&str] = &[
    "com.cryptohud.market-board",
    "com.example.stage3-price-card",
];
const BUNDLED_BUILTIN_SLINT_PLUGIN_IDS: &[&str] = &[
    "com.cryptohud.focus-ticker",
    "com.cryptohud.market-compass",
    "com.cryptohud.trust-card",
    "com.cryptohud.status-strip",
];
const BUNDLED_PLUGIN_DIRECTORY_NAME: &str = "plugins";
const BUNDLED_RESOURCE_DIRECTORY_NAME: &str = "resources";
#[cfg(test)]
const HOST_SCALE_REPO_PLUGIN_IDS: &[&str] = &[
    "com.cryptohud.focus-ticker",
    "com.cryptohud.market-board",
    "com.cryptohud.market-compass",
    "com.cryptohud.status-strip",
    "com.cryptohud.trust-card",
    "com.example.stage3-price-card",
];
#[cfg(test)]
const DIRECT_SCALE_REPO_PLUGIN_IDS: &[&str] = &[
    "com.cryptohud.focus-ticker",
    "com.cryptohud.status-strip",
    "com.cryptohud.trust-card",
];
const ALLOWED_EXTENSIONS: &[&str] = &["json", "slint", "png", "jpg", "jpeg", "svg"];
const REQUIRED_PROPERTIES: &[(&str, ValueType)] = &[
    ("widget-id", ValueType::String),
    ("quote-rows", ValueType::Model),
    ("pairs-heading-text", ValueType::String),
    ("source-text", ValueType::String),
    ("updated-text", ValueType::String),
    ("empty-text", ValueType::String),
    ("pin-to-top", ValueType::Bool),
    ("layout-locked", ValueType::Bool),
    ("widget-width", ValueType::Number),
    ("widget-height", ValueType::Number),
    ("theme-name", ValueType::String),
    ("red-up-enabled", ValueType::Bool),
    ("content-opacity", ValueType::Number),
];
const REQUIRED_CALLBACKS: &[&str] = &["drag-move", "toggle-layout-lock"];

#[derive(Debug, Clone)]
struct PluginCandidateRecord {
    active: Option<PluginDefinition>,
    revision: u64,
    diagnostic: Option<PluginCatalogError>,
    blocked: Option<BlockedPluginCandidate>,
}

#[derive(Debug, Clone)]
struct BlockedPluginCandidate {
    plugin: PluginDefinition,
    revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EffectivePluginRevision {
    Static,
    Local {
        source: PluginSourceKey,
        generation: u64,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct PluginCatalogReload {
    pub(crate) catalog: PluginCatalog,
    pub(crate) changed_plugin_ids: BTreeSet<String>,
    pub(crate) retained_plugin_ids: BTreeSet<String>,
    pub(crate) compiled_source_count: usize,
}

#[derive(Debug, Clone)]
pub struct PluginCatalog {
    plugins: RefCell<Vec<PluginDefinition>>,
    errors: RefCell<Vec<PluginCatalogError>>,
    candidates: RefCell<BTreeMap<PluginSourceKey, PluginCandidateRecord>>,
    snapshot: RefCell<PluginTreeSnapshot>,
    effective_revisions: RefCell<HashMap<String, EffectivePluginRevision>>,
    required_bundle_state_dir: Option<PathBuf>,
}

impl PluginCatalog {
    #[cfg(test)]
    pub fn builtins() -> Self {
        let plugins = builtin_plugins();
        let effective_revisions = plugins
            .iter()
            .map(|plugin| (plugin.id.clone(), EffectivePluginRevision::Static))
            .collect();
        Self {
            plugins: RefCell::new(plugins),
            errors: RefCell::new(Vec::new()),
            candidates: RefCell::new(BTreeMap::new()),
            snapshot: RefCell::new(PluginTreeSnapshot::default()),
            effective_revisions: RefCell::new(effective_revisions),
            required_bundle_state_dir: None,
        }
    }

    pub fn load(state_dir: &Path) -> Self {
        let snapshot = plugin_tree_snapshot(state_dir);
        Self::from_snapshot(snapshot, Some(state_dir.to_path_buf()), 0)
    }

    #[cfg(test)]
    pub fn discover(plugin_roots: Vec<PathBuf>) -> Self {
        let snapshot = scan_plugin_roots(&plugin_roots);
        Self::from_snapshot(snapshot, None, 0)
    }

    pub fn plugins(&self) -> Vec<PluginDefinition> {
        self.plugins.borrow().clone()
    }

    pub fn market_plugins(&self) -> impl Iterator<Item = PluginDefinition> {
        self.plugins
            .borrow()
            .clone()
            .into_iter()
            .filter(|plugin| is_market_plugin_visible(&plugin.id))
    }

    pub fn available_replacements(&self, current_plugin_id: &str) -> Vec<PluginDefinition> {
        self.plugins()
            .into_iter()
            .filter(|plugin| plugin.id != current_plugin_id && plugin.is_available())
            .collect()
    }

    pub fn errors(&self) -> Vec<PluginCatalogError> {
        self.errors.borrow().clone()
    }

    pub(crate) fn tree_snapshot(&self) -> PluginTreeSnapshot {
        self.snapshot.borrow().clone()
    }

    pub fn diagnostic_messages(&self, state_dir: &Path) -> Vec<String> {
        self.errors
            .borrow()
            .iter()
            .map(|error| {
                let path = redacted_plugin_path(&error.path, state_dir);
                let message = redact_plugin_roots(&error.message, state_dir);
                format!("{path}: {message}")
            })
            .collect()
    }

    pub fn find(&self, plugin_id: &str) -> Option<PluginDefinition> {
        self.plugins
            .borrow()
            .iter()
            .find(|plugin| plugin.id == plugin_id)
            .cloned()
    }

    pub fn replace_with(&self, replacement: Self) {
        self.plugins.replace(replacement.plugins.into_inner());
        self.errors.replace(replacement.errors.into_inner());
        self.candidates.replace(replacement.candidates.into_inner());
        self.snapshot.replace(replacement.snapshot.into_inner());
        self.effective_revisions
            .replace(replacement.effective_revisions.into_inner());
    }

    pub(crate) fn reload_incremental(&self, plan: &PluginReloadPlan) -> PluginCatalogReload {
        let previous_candidates = self.candidates.borrow();
        let mut candidates = BTreeMap::new();
        let mut retained_plugin_ids = BTreeSet::new();
        let mut compiled_source_count = 0;
        let mut higher_priority_ids = builtin_plugins()
            .into_iter()
            .map(|plugin| plugin.id)
            .collect::<HashSet<_>>();
        let mut sources = plan
            .snapshot
            .candidates
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        sources.extend(
            previous_candidates
                .keys()
                .filter(|source| {
                    plan.snapshot
                        .incomplete_root_ranks
                        .contains(&source.root_rank)
                        || plan.snapshot.incomplete_sources.contains(*source)
                })
                .cloned(),
        );

        for source in sources {
            let previous = previous_candidates.get(&source);
            let source_is_incomplete = plan.snapshot.incomplete_sources.contains(&source)
                || (plan
                    .snapshot
                    .incomplete_root_ranks
                    .contains(&source.root_rank)
                    && !plan.snapshot.candidates.contains_key(&source));
            let mut record = if source_is_incomplete {
                if let Some(plugin) = previous
                    .and_then(|record| record.active.as_ref())
                    .filter(|plugin| plugin.is_available())
                {
                    retained_plugin_ids.insert(plugin.id.clone());
                }
                previous.cloned().unwrap_or_else(|| PluginCandidateRecord {
                    active: None,
                    revision: plan.generation,
                    diagnostic: None,
                    blocked: None,
                })
            } else if plan.changed_sources.contains(&source) || previous.is_none() {
                compiled_source_count += 1;
                let mut fresh = load_plugin_candidate(&source, plan.generation);
                let fresh_failed = fresh.diagnostic.is_some();
                let fresh_id = fresh.active.as_ref().map(|plugin| plugin.id.clone());
                let conflicts_with_higher_priority = fresh_id
                    .as_ref()
                    .is_some_and(|plugin_id| higher_priority_ids.contains(plugin_id));
                let previous_is_runnable = previous
                    .and_then(|record| record.active.as_ref())
                    .is_some_and(PluginDefinition::is_available);
                let fresh_is_runnable = fresh
                    .active
                    .as_ref()
                    .is_some_and(PluginDefinition::is_available);
                let changed_to_duplicate_id = fresh_is_runnable
                    && conflicts_with_higher_priority
                    && previous
                        .and_then(|record| record.active.as_ref())
                        .is_some_and(|plugin| Some(&plugin.id) != fresh_id.as_ref());

                if previous_is_runnable && (fresh_failed || changed_to_duplicate_id) {
                    if let Some(previous) = previous {
                        if changed_to_duplicate_id {
                            if let Some(plugin) = fresh.active.take() {
                                fresh.blocked = Some(BlockedPluginCandidate {
                                    plugin,
                                    revision: fresh.revision,
                                });
                            }
                        }
                        fresh.active = previous.active.clone();
                        fresh.revision = previous.revision;
                        if let Some(plugin) = &fresh.active {
                            retained_plugin_ids.insert(plugin.id.clone());
                        }
                    }
                }
                fresh
            } else {
                previous.cloned().unwrap_or_else(|| PluginCandidateRecord {
                    active: None,
                    revision: plan.generation,
                    diagnostic: None,
                    blocked: None,
                })
            };

            if let Some(blocked) = record.blocked.take() {
                if higher_priority_ids.contains(&blocked.plugin.id) {
                    record.blocked = Some(blocked);
                } else {
                    record.active = Some(blocked.plugin);
                    record.revision = blocked.revision;
                    record.diagnostic = None;
                }
            }
            if let Some(plugin) = &record.active {
                higher_priority_ids.insert(plugin.id.clone());
            }
            candidates.insert(source, record);
        }
        drop(previous_candidates);

        let catalog = Self::from_records(
            plan.snapshot.clone(),
            self.required_bundle_state_dir.clone(),
            candidates,
        );
        let previous_revisions = self.effective_revisions.borrow();
        let next_revisions = catalog.effective_revisions.borrow();
        let changed_plugin_ids = previous_revisions
            .keys()
            .chain(next_revisions.keys())
            .filter(|plugin_id| {
                previous_revisions.get(*plugin_id) != next_revisions.get(*plugin_id)
            })
            .cloned()
            .collect();
        drop(next_revisions);
        drop(previous_revisions);

        PluginCatalogReload {
            catalog,
            changed_plugin_ids,
            retained_plugin_ids,
            compiled_source_count,
        }
    }

    fn from_snapshot(
        snapshot: PluginTreeSnapshot,
        required_bundle_state_dir: Option<PathBuf>,
        revision: u64,
    ) -> Self {
        let candidates = snapshot
            .candidates
            .keys()
            .map(|source| {
                let record = if snapshot.incomplete_sources.contains(source) {
                    PluginCandidateRecord {
                        active: None,
                        revision,
                        diagnostic: None,
                        blocked: None,
                    }
                } else {
                    load_plugin_candidate(source, revision)
                };
                (source.clone(), record)
            })
            .collect();
        Self::from_records(snapshot, required_bundle_state_dir, candidates)
    }

    fn from_records(
        snapshot: PluginTreeSnapshot,
        required_bundle_state_dir: Option<PathBuf>,
        candidates: BTreeMap<PluginSourceKey, PluginCandidateRecord>,
    ) -> Self {
        let mut plugins = builtin_plugins();
        let mut errors = snapshot.scan_errors.clone();
        let mut effective_revisions = plugins
            .iter()
            .map(|plugin| (plugin.id.clone(), EffectivePluginRevision::Static))
            .collect::<HashMap<_, _>>();
        let mut seen_ids = plugins
            .iter()
            .map(|plugin| plugin.id.clone())
            .collect::<HashSet<_>>();

        for (source, record) in &candidates {
            if let Some(diagnostic) = &record.diagnostic {
                errors.push(diagnostic.clone());
            }
            if let Some(blocked) = &record.blocked {
                errors.push(PluginCatalogError {
                    path: source.directory.join(MANIFEST_FILE_NAME),
                    message: format!("duplicate plugin id {}", blocked.plugin.id),
                });
            }
            let Some(plugin) = &record.active else {
                continue;
            };
            if !seen_ids.insert(plugin.id.clone()) {
                errors.push(PluginCatalogError {
                    path: source.directory.join(MANIFEST_FILE_NAME),
                    message: format!("duplicate plugin id {}", plugin.id),
                });
                continue;
            }
            effective_revisions.insert(
                plugin.id.clone(),
                EffectivePluginRevision::Local {
                    source: source.clone(),
                    generation: record.revision,
                },
            );
            plugins.push(plugin.clone());
        }

        if let Some(state_dir) = required_bundle_state_dir.as_deref() {
            append_required_bundled_diagnostics(&plugins, &mut errors, state_dir);
        }

        Self {
            plugins: RefCell::new(plugins),
            errors: RefCell::new(errors),
            candidates: RefCell::new(candidates),
            snapshot: RefCell::new(snapshot),
            effective_revisions: RefCell::new(effective_revisions),
            required_bundle_state_dir,
        }
    }

    #[cfg(test)]
    pub fn from_plugins_for_tests(plugins: Vec<PluginDefinition>) -> Self {
        let effective_revisions = plugins
            .iter()
            .map(|plugin| (plugin.id.clone(), EffectivePluginRevision::Static))
            .collect();
        Self {
            plugins: RefCell::new(plugins),
            errors: RefCell::new(Vec::new()),
            candidates: RefCell::new(BTreeMap::new()),
            snapshot: RefCell::new(PluginTreeSnapshot::default()),
            effective_revisions: RefCell::new(effective_revisions),
            required_bundle_state_dir: None,
        }
    }
}

fn redacted_plugin_path(path: &Path, state_dir: &Path) -> String {
    for root in plugin_roots(state_dir) {
        let relative = path
            .strip_prefix(&root)
            .map(Path::to_path_buf)
            .ok()
            .or_else(|| {
                root.canonicalize()
                    .ok()
                    .and_then(|canonical| path.strip_prefix(canonical).ok().map(Path::to_path_buf))
            });
        if let Some(relative) = relative {
            let label = if root == user_plugin_root(state_dir) {
                "<user-plugins>"
            } else {
                "<bundled-plugins>"
            };
            return if relative.as_os_str().is_empty() {
                label.to_string()
            } else {
                format!("{label}/{}", relative.display())
            };
        }
    }
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "<plugin>".to_string())
}

fn redact_plugin_roots(message: &str, state_dir: &Path) -> String {
    let mut message = message.to_string();
    for root in plugin_roots(state_dir) {
        let label = if root == user_plugin_root(state_dir) {
            "<user-plugins>"
        } else {
            "<bundled-plugins>"
        };
        if let Ok(canonical) = root.canonicalize() {
            message = redact_path_text(message, &canonical, label);
        }
        message = redact_path_text(message, &root, label);
    }
    message
}

fn redact_path_text(mut message: String, path: &Path, label: &str) -> String {
    let raw = path.to_string_lossy();
    message = message.replace(raw.as_ref(), label);
    let debug_escaped = raw.escape_debug().to_string();
    message.replace(&debug_escaped, label)
}

pub fn is_market_plugin_visible(plugin_id: &str) -> bool {
    !HIDDEN_MARKET_PLUGIN_IDS.contains(&plugin_id)
}

pub fn is_prototype_plugin(plugin_id: &str) -> bool {
    DISABLED_PROTOTYPE_PLUGIN_IDS.contains(&plugin_id)
}

#[derive(Debug, Clone)]
pub struct PluginDefinition {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub schema_version: u32,
    pub host_api_version: VersionReq,
    pub source: PluginSource,
    pub renderer: PluginRendererDefinition,
    pub default_size: PluginSize,
    pub size_policy: PluginSizePolicy,
    pub min_symbol_limit: usize,
    pub symbol_limit: usize,
    pub default_symbols: Vec<String>,
    pub preview_images: Vec<PathBuf>,
    pub themes: Vec<PluginTheme>,
    pub data_requirements: Vec<PluginDataRequirement>,
    pub parameters: Vec<PluginParameter>,
    pub status: PluginStatus,
}

impl PluginDefinition {
    pub fn is_available(&self) -> bool {
        matches!(self.status, PluginStatus::Available)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginSource {
    Builtin,
    LocalUnsigned,
    #[allow(dead_code)]
    TrustedSigned,
}

#[derive(Clone)]
pub enum PluginRendererDefinition {
    Builtin(BuiltinRenderer),
    Slint {
        root_dir: PathBuf,
        entry: PathBuf,
        component: String,
        definition: Option<ComponentDefinition>,
    },
}

impl fmt::Debug for PluginRendererDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Builtin(renderer) => f.debug_tuple("Builtin").field(renderer).finish(),
            Self::Slint {
                root_dir,
                entry,
                component,
                definition,
            } => f
                .debug_struct("Slint")
                .field("root_dir", root_dir)
                .field("entry", entry)
                .field("component", component)
                .field("compiled", &definition.is_some())
                .finish(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinRenderer {
    QuoteBoard,
    MiniTicker,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginStatus {
    Available,
    Disabled(String),
    Unavailable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginCatalogError {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PluginSourceKey {
    root_rank: usize,
    directory: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginTreeFingerprint {
    entries: Vec<PluginTreeFingerprintEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PluginTreeFingerprintEntry {
    path: PathBuf,
    kind: u8,
    length: u64,
    modified_nanos: u128,
    content_hash: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PluginTreeSnapshot {
    candidates: BTreeMap<PluginSourceKey, PluginTreeFingerprint>,
    scan_errors: Vec<PluginCatalogError>,
    incomplete_root_ranks: BTreeSet<usize>,
    incomplete_sources: BTreeSet<PluginSourceKey>,
}

impl PluginTreeSnapshot {
    fn changed_sources(&self, next: &Self) -> BTreeSet<PluginSourceKey> {
        self.candidates
            .keys()
            .chain(next.candidates.keys())
            .filter(|source| {
                self.candidates.get(*source) != next.candidates.get(*source)
                    || self.incomplete_sources.contains(*source)
                        != next.incomplete_sources.contains(*source)
            })
            .cloned()
            .collect()
    }

    fn all_sources_with(&self, next: &Self) -> BTreeSet<PluginSourceKey> {
        self.candidates
            .keys()
            .chain(next.candidates.keys())
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PluginReloadPlan {
    pub(crate) generation: u64,
    snapshot: PluginTreeSnapshot,
    changed_sources: BTreeSet<PluginSourceKey>,
}

impl PluginReloadPlan {
    pub(crate) fn is_current(&self, state_dir: &Path) -> bool {
        plugin_tree_snapshot(state_dir) == self.snapshot
    }
}

#[derive(Debug)]
pub(crate) struct PluginReloadTracker {
    observed: PluginTreeSnapshot,
    committed: PluginTreeSnapshot,
    stable_ticks: u8,
    next_generation: u64,
    pending: Option<PluginReloadPlan>,
    in_flight: Option<PluginReloadPlan>,
}

impl PluginReloadTracker {
    pub(crate) fn new(initial: PluginTreeSnapshot) -> Self {
        Self {
            observed: initial.clone(),
            committed: initial,
            stable_ticks: 0,
            next_generation: 0,
            pending: None,
            in_flight: None,
        }
    }

    pub(crate) fn observe(&mut self, current: PluginTreeSnapshot) -> bool {
        if current != self.observed {
            self.observed = current;
            self.stable_ticks = 0;
            self.pending = None;
            return false;
        }
        if current == self.committed
            || self
                .pending
                .as_ref()
                .is_some_and(|plan| plan.snapshot == current)
            || self
                .in_flight
                .as_ref()
                .is_some_and(|plan| plan.snapshot == current)
        {
            return false;
        }
        self.stable_ticks = self.stable_ticks.saturating_add(1);
        if self.stable_ticks < 2 {
            return false;
        }

        self.next_generation = self.next_generation.wrapping_add(1).max(1);
        let changed_sources = self.committed.changed_sources(&current);
        self.pending = Some(PluginReloadPlan {
            generation: self.next_generation,
            snapshot: current,
            changed_sources,
        });
        self.stable_ticks = 0;
        true
    }

    pub(crate) fn take_pending_or_force(
        &mut self,
        current: PluginTreeSnapshot,
    ) -> PluginReloadPlan {
        let plan = self.pending.take().unwrap_or_else(|| {
            self.next_generation = self.next_generation.wrapping_add(1).max(1);
            let changed_sources = self.committed.all_sources_with(&current);
            self.observed = current.clone();
            self.stable_ticks = 0;
            PluginReloadPlan {
                generation: self.next_generation,
                snapshot: current,
                changed_sources,
            }
        });
        self.in_flight = Some(plan.clone());
        plan
    }

    pub(crate) fn finish(&mut self, generation: u64, applied: bool) {
        let Some(plan) = self.in_flight.take() else {
            return;
        };
        if plan.generation != generation {
            self.in_flight = Some(plan);
            return;
        }
        if applied {
            self.committed = plan.snapshot;
        }
        self.stable_ticks = 0;
    }
}

pub(crate) fn plugin_tree_snapshot(state_dir: &Path) -> PluginTreeSnapshot {
    scan_plugin_roots(&plugin_roots(state_dir))
}

pub(crate) fn plugin_tree_snapshot_cancellable(
    state_dir: &Path,
    cancelled: &AtomicBool,
) -> Option<PluginTreeSnapshot> {
    scan_plugin_roots_with_cancel(&plugin_roots(state_dir), Some(cancelled))
}

fn scan_plugin_roots(roots: &[PathBuf]) -> PluginTreeSnapshot {
    scan_plugin_roots_with_cancel(roots, None).unwrap_or_default()
}

struct PluginScanControl<'a> {
    cancelled: Option<&'a AtomicBool>,
    remaining_entries: usize,
    remaining_hash_bytes: u64,
    remaining_candidates: usize,
}

impl PluginScanControl<'_> {
    fn is_cancelled(&self) -> bool {
        self.cancelled
            .is_some_and(|cancelled| cancelled.load(Ordering::Relaxed))
    }

    fn take_entry(&mut self) -> bool {
        if self.remaining_entries == 0 {
            return false;
        }
        self.remaining_entries -= 1;
        true
    }

    fn take_candidate(&mut self) -> bool {
        if self.remaining_candidates == 0 {
            return false;
        }
        self.remaining_candidates -= 1;
        true
    }
}

fn scan_plugin_roots_with_cancel(
    roots: &[PathBuf],
    cancelled: Option<&AtomicBool>,
) -> Option<PluginTreeSnapshot> {
    let mut snapshot = PluginTreeSnapshot::default();
    let mut control = PluginScanControl {
        cancelled,
        remaining_entries: PLUGIN_SCAN_MAX_ENTRIES,
        remaining_hash_bytes: PLUGIN_SCAN_MAX_HASH_BYTES,
        remaining_candidates: PLUGIN_SCAN_MAX_CANDIDATES,
    };

    for (root_rank, root) in roots.iter().enumerate() {
        if control.is_cancelled() {
            return None;
        }
        match fs::symlink_metadata(root) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => {
                mark_incomplete_root(
                    &mut snapshot,
                    root_rank,
                    root,
                    "plugin root is not a directory".to_string(),
                );
                continue;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => {
                mark_incomplete_root(&mut snapshot, root_rank, root, error.to_string());
                continue;
            }
        }
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) => {
                mark_incomplete_root(&mut snapshot, root_rank, root, error.to_string());
                continue;
            }
        };
        let mut directories = Vec::new();
        for entry in entries {
            if control.is_cancelled() {
                return None;
            }
            if !control.take_entry() {
                mark_incomplete_root(
                    &mut snapshot,
                    root_rank,
                    root,
                    format!("plugin scan exceeds {PLUGIN_SCAN_MAX_ENTRIES} total entries"),
                );
                break;
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    mark_incomplete_root(&mut snapshot, root_rank, root, error.to_string());
                    continue;
                }
            };
            let path = entry.path();
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    mark_incomplete_root(&mut snapshot, root_rank, &path, error.to_string());
                    continue;
                }
            };
            if !metadata.is_dir() {
                continue;
            }
            let manifest_path = path.join(MANIFEST_FILE_NAME);
            match fs::metadata(&manifest_path) {
                Ok(metadata) if metadata.is_file() => directories.push(path),
                Ok(_) => {
                    mark_incomplete_root(
                        &mut snapshot,
                        root_rank,
                        &manifest_path,
                        "plugin manifest is not a regular file".to_string(),
                    );
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => {
                    mark_incomplete_root(
                        &mut snapshot,
                        root_rank,
                        &manifest_path,
                        error.to_string(),
                    );
                }
            }
        }
        directories.sort_unstable();
        for directory in directories {
            if control.is_cancelled() {
                return None;
            }
            if !control.take_candidate() {
                mark_incomplete_root(
                    &mut snapshot,
                    root_rank,
                    root,
                    format!(
                        "plugin scan exceeds {PLUGIN_SCAN_MAX_CANDIDATES} candidate directories"
                    ),
                );
                break;
            }
            let directory = match directory.canonicalize() {
                Ok(directory) => directory,
                Err(error) => {
                    mark_incomplete_root(&mut snapshot, root_rank, &directory, error.to_string());
                    continue;
                }
            };
            let source = PluginSourceKey {
                root_rank,
                directory: directory.clone(),
            };
            let fingerprint = fingerprint_plugin_directory(&directory, &mut control)?;
            if !fingerprint.complete {
                snapshot.incomplete_sources.insert(source.clone());
            }
            snapshot.scan_errors.extend(fingerprint.errors);
            snapshot.candidates.insert(source, fingerprint.fingerprint);
        }
    }

    snapshot.scan_errors.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.message.cmp(&right.message))
    });
    Some(snapshot)
}

fn mark_incomplete_root(
    snapshot: &mut PluginTreeSnapshot,
    root_rank: usize,
    path: &Path,
    message: String,
) {
    snapshot.incomplete_root_ranks.insert(root_rank);
    snapshot.scan_errors.push(PluginCatalogError {
        path: path.to_path_buf(),
        message,
    });
}

struct PluginFingerprintScan {
    fingerprint: PluginTreeFingerprint,
    errors: Vec<PluginCatalogError>,
    complete: bool,
}

fn fingerprint_plugin_directory(
    root: &Path,
    control: &mut PluginScanControl<'_>,
) -> Option<PluginFingerprintScan> {
    let mut scan = PluginFingerprintScan {
        fingerprint: PluginTreeFingerprint {
            entries: Vec::new(),
        },
        errors: Vec::new(),
        complete: true,
    };
    let mut remaining_entries = PLUGIN_SCAN_MAX_ENTRIES_PER_CANDIDATE;
    let mut remaining_hash_bytes = PLUGIN_DIR_MAX_BYTES.saturating_add(1);
    if !collect_plugin_tree_fingerprint(
        root,
        root,
        &mut scan,
        control,
        &mut remaining_entries,
        &mut remaining_hash_bytes,
    ) {
        return None;
    }
    scan.fingerprint.entries.sort_unstable();
    Some(scan)
}

fn mark_incomplete_fingerprint(
    scan: &mut PluginFingerprintScan,
    path: &Path,
    message: impl Into<String>,
) {
    scan.complete = false;
    scan.errors.push(PluginCatalogError {
        path: path.to_path_buf(),
        message: message.into(),
    });
}

fn collect_plugin_tree_fingerprint(
    root: &Path,
    path: &Path,
    scan: &mut PluginFingerprintScan,
    control: &mut PluginScanControl<'_>,
    remaining_entries: &mut usize,
    remaining_hash_bytes: &mut u64,
) -> bool {
    if control.is_cancelled() {
        return false;
    }
    if *remaining_entries == 0 || !control.take_entry() {
        mark_incomplete_fingerprint(
            scan,
            root,
            format!(
                "plugin scan exceeds {PLUGIN_SCAN_MAX_ENTRIES_PER_CANDIDATE} entries per candidate or {PLUGIN_SCAN_MAX_ENTRIES} total entries"
            ),
        );
        return true;
    }
    *remaining_entries -= 1;
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            mark_incomplete_fingerprint(scan, path, error.to_string());
            return true;
        }
    };
    let kind = {
        if metadata.is_dir() {
            1
        } else if metadata.is_file() {
            2
        } else {
            3
        }
    };
    let length = metadata.len();
    let modified_nanos = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_nanos());
    let content_hash = if kind == 2 {
        let hash_bytes = length.min(ASSET_MAX_BYTES.saturating_add(1));
        if hash_bytes > *remaining_hash_bytes || hash_bytes > control.remaining_hash_bytes {
            mark_incomplete_fingerprint(
                scan,
                path,
                format!(
                    "plugin scan exceeds {PLUGIN_DIR_MAX_BYTES} bytes per candidate or {PLUGIN_SCAN_MAX_HASH_BYTES} total hash bytes"
                ),
            );
            0
        } else {
            *remaining_hash_bytes -= hash_bytes;
            control.remaining_hash_bytes -= hash_bytes;
            match bounded_file_content_hash(path, hash_bytes, control.cancelled) {
                FileHashResult::Complete(hash) => hash,
                FileHashResult::Error(error) => {
                    mark_incomplete_fingerprint(scan, path, error.to_string());
                    0
                }
                FileHashResult::Cancelled => return false,
            }
        }
    } else {
        0
    };
    scan.fingerprint.entries.push(PluginTreeFingerprintEntry {
        path: path.strip_prefix(root).unwrap_or(path).to_path_buf(),
        kind,
        length,
        modified_nanos,
        content_hash,
    });

    if kind != 1 {
        return true;
    }
    let children = match fs::read_dir(path) {
        Ok(children) => children,
        Err(error) => {
            mark_incomplete_fingerprint(scan, path, error.to_string());
            return true;
        }
    };
    let child_limit = (*remaining_entries).min(control.remaining_entries);
    let mut child_paths = Vec::new();
    for child in children {
        if control.is_cancelled() {
            return false;
        }
        if child_paths.len() >= child_limit {
            mark_incomplete_fingerprint(
                scan,
                path,
                format!(
                    "plugin scan exceeds {PLUGIN_SCAN_MAX_ENTRIES_PER_CANDIDATE} entries per candidate or {PLUGIN_SCAN_MAX_ENTRIES} total entries"
                ),
            );
            break;
        }
        match child {
            Ok(child) => child_paths.push(child.path()),
            Err(error) => mark_incomplete_fingerprint(scan, path, error.to_string()),
        }
    }
    let mut children = child_paths;
    children.sort_unstable();
    for child in children {
        if !collect_plugin_tree_fingerprint(
            root,
            &child,
            scan,
            control,
            remaining_entries,
            remaining_hash_bytes,
        ) {
            return false;
        }
    }
    true
}

enum FileHashResult {
    Complete(u64),
    Error(io::Error),
    Cancelled,
}

fn bounded_file_content_hash(
    path: &Path,
    hash_bytes: u64,
    cancelled: Option<&AtomicBool>,
) -> FileHashResult {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) => return FileHashResult::Error(error),
    };
    let mut reader = file.take(hash_bytes);
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        if cancelled.is_some_and(|cancelled| cancelled.load(Ordering::Relaxed)) {
            return FileHashResult::Cancelled;
        }
        let read = match reader.read(&mut buffer) {
            Ok(read) => read,
            Err(error) => return FileHashResult::Error(error),
        };
        if read == 0 {
            break;
        }
        buffer[..read].hash(&mut hasher);
    }
    FileHashResult::Complete(hasher.finish())
}

impl fmt::Display for PluginCatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.message)
    }
}

impl std::error::Error for PluginCatalogError {}

pub fn builtin_plugins() -> Vec<PluginDefinition> {
    vec![
        PluginDefinition {
            id: BUILTIN_QUOTE_BOARD_PLUGIN_ID.to_string(),
            name: "Quote Board".to_string(),
            version: Version::new(0, 1, 0),
            schema_version: PLUGIN_MANIFEST_SCHEMA_VERSION,
            host_api_version: builtin_host_api_requirement(),
            source: PluginSource::Builtin,
            renderer: PluginRendererDefinition::Builtin(BuiltinRenderer::QuoteBoard),
            default_size: PluginSize {
                width: 286,
                height: 194,
            },
            size_policy: PluginSizePolicy::Fixed,
            min_symbol_limit: MIN_SYMBOL_LIMIT,
            symbol_limit: crypto_hud_shell_state::WidgetKind::QuoteBoard.symbol_limit(),
            default_symbols: default_market_symbols(),
            preview_images: builtin_preview_images("quote-board"),
            themes: builtin_light_dark_themes(),
            data_requirements: vec![PluginDataRequirement {
                capability: "market.price".to_string(),
            }],
            parameters: Vec::new(),
            status: PluginStatus::Available,
        },
        PluginDefinition {
            id: BUILTIN_MINI_TICKER_PLUGIN_ID.to_string(),
            name: "Mini Ticker".to_string(),
            version: Version::new(0, 1, 0),
            schema_version: PLUGIN_MANIFEST_SCHEMA_VERSION,
            host_api_version: builtin_host_api_requirement(),
            source: PluginSource::Builtin,
            renderer: PluginRendererDefinition::Builtin(BuiltinRenderer::MiniTicker),
            default_size: PluginSize {
                width: 236,
                height: 112,
            },
            size_policy: PluginSizePolicy::Fixed,
            min_symbol_limit: MIN_SYMBOL_LIMIT,
            symbol_limit: MIN_SYMBOL_LIMIT,
            default_symbols: default_market_symbols().into_iter().take(1).collect(),
            preview_images: builtin_preview_images("mini-ticker"),
            themes: builtin_light_dark_themes(),
            data_requirements: vec![PluginDataRequirement {
                capability: "market.price".to_string(),
            }],
            parameters: Vec::new(),
            status: PluginStatus::Available,
        },
    ]
}

fn builtin_host_api_requirement() -> VersionReq {
    VersionReq::parse(&format!("={HOST_PLUGIN_API_VERSION}")).unwrap_or(VersionReq::STAR)
}

pub fn default_theme_id(plugin: &PluginDefinition) -> &str {
    plugin
        .themes
        .iter()
        .find(|theme| theme.is_default)
        .or_else(|| plugin.themes.first())
        .map(|theme| theme.id.as_str())
        .unwrap_or("default")
}

#[cfg(test)]
pub fn single_default_theme() -> Vec<PluginTheme> {
    vec![PluginTheme {
        id: "default".to_string(),
        name: "Default".to_string(),
        role: PluginThemeRole::Default,
        is_default: true,
    }]
}

fn builtin_light_dark_themes() -> Vec<PluginTheme> {
    vec![
        PluginTheme {
            id: "light".to_string(),
            name: "Light".to_string(),
            role: PluginThemeRole::Light,
            is_default: false,
        },
        PluginTheme {
            id: "dark".to_string(),
            name: "Dark".to_string(),
            role: PluginThemeRole::Dark,
            is_default: true,
        },
    ]
}

fn builtin_preview_images(prefix: &str) -> Vec<PathBuf> {
    ["light", "dark"]
        .into_iter()
        .map(|theme| {
            bundled_resource_path(Path::new("previews").join(format!("{prefix}-{theme}.png")))
        })
        .collect()
}

pub fn plugin_roots(state_dir: &Path) -> Vec<PathBuf> {
    let mut roots = bundled_plugin_roots();
    roots.push(user_plugin_root(state_dir));
    roots
}

fn executable_directory() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
}

fn installed_plugin_root() -> Option<PathBuf> {
    executable_directory().map(|directory| directory.join(BUNDLED_PLUGIN_DIRECTORY_NAME))
}

#[cfg(any(debug_assertions, test))]
fn development_plugin_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(BUNDLED_PLUGIN_DIRECTORY_NAME)
}

fn bundled_plugin_roots() -> Vec<PathBuf> {
    let installed_roots = installed_plugin_root().into_iter().collect::<Vec<_>>();
    #[cfg(any(debug_assertions, test))]
    {
        let mut roots = installed_roots;
        let development_root = development_plugin_root();
        if !roots.contains(&development_root) {
            roots.push(development_root);
        }
        roots
    }
    #[cfg(not(any(debug_assertions, test)))]
    {
        installed_roots
    }
}

pub(crate) fn bundled_resource_path(relative_path: impl AsRef<Path>) -> PathBuf {
    let relative_path = relative_path.as_ref();
    let installed_path = executable_directory()
        .map(|directory| {
            directory
                .join(BUNDLED_RESOURCE_DIRECTORY_NAME)
                .join(relative_path)
        })
        .unwrap_or_else(|| PathBuf::from(BUNDLED_RESOURCE_DIRECTORY_NAME).join(relative_path));
    if installed_path.exists() {
        return installed_path;
    }

    #[cfg(any(debug_assertions, test))]
    {
        let development_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("ui")
            .join(relative_path);
        if development_path.exists() {
            return development_path;
        }
    }

    installed_path
}

pub fn user_plugin_root(state_dir: &Path) -> PathBuf {
    state_dir.join("plugins")
}

pub fn sync_user_plugin_development_guide(state_dir: &Path) -> Result<PathBuf> {
    let root = user_plugin_root(state_dir);
    fs::create_dir_all(&root)
        .with_context(|| format!("failed to create plugin directory {}", root.display()))?;
    let guide_path = root.join(USER_PLUGIN_DEVELOPMENT_GUIDE_FILE_NAME);
    fs::write(&guide_path, USER_PLUGIN_DEVELOPMENT_GUIDE)
        .with_context(|| format!("failed to write {}", guide_path.display()))?;
    Ok(guide_path)
}

fn load_plugin_candidate(source: &PluginSourceKey, revision: u64) -> PluginCandidateRecord {
    let manifest_path = source.directory.join(MANIFEST_FILE_NAME);
    match load_local_plugin(&source.directory) {
        Ok(plugin) => {
            let diagnostic = match &plugin.status {
                PluginStatus::Unavailable(message) => Some(PluginCatalogError {
                    path: manifest_path,
                    message: message.clone(),
                }),
                PluginStatus::Available | PluginStatus::Disabled(_) => None,
            };
            PluginCandidateRecord {
                active: Some(plugin),
                revision,
                diagnostic,
                blocked: None,
            }
        }
        Err(error) => PluginCandidateRecord {
            active: None,
            revision,
            diagnostic: Some(PluginCatalogError {
                path: manifest_path,
                message: error.to_string(),
            }),
            blocked: None,
        },
    }
}

fn append_required_bundled_diagnostics(
    plugins: &[PluginDefinition],
    errors: &mut Vec<PluginCatalogError>,
    _state_dir: &Path,
) {
    let bundled_roots = bundled_plugin_roots();
    let bundled_root_available = bundled_roots.iter().any(|root| root.is_dir());
    let expected_bundled_root = bundled_roots
        .iter()
        .find(|root| root.is_dir())
        .or_else(|| bundled_roots.first())
        .cloned();
    if !bundled_root_available {
        if let Some(root) = expected_bundled_root.as_ref() {
            errors.push(PluginCatalogError {
                path: root.clone(),
                message: "bundled plugin directory is missing".to_string(),
            });
        }
    }
    for plugin_id in BUNDLED_BUILTIN_SLINT_PLUGIN_IDS {
        let bundled_plugin_loaded = plugins.iter().any(|plugin| {
            plugin.id == *plugin_id
                && matches!(
                    &plugin.renderer,
                    PluginRendererDefinition::Slint { root_dir, .. }
                        if is_bundled_builtin_slint_plugin(plugin_id, root_dir)
                )
        });
        if !bundled_plugin_loaded {
            let path = expected_bundled_root
                .as_ref()
                .map(|root| root.join(plugin_id))
                .unwrap_or_else(|| PathBuf::from(plugin_id));
            errors.push(PluginCatalogError {
                path,
                message: "required bundled plugin is missing or invalid".to_string(),
            });
        }
    }
}

fn load_local_plugin(root: &Path) -> Result<PluginDefinition> {
    validate_plugin_directory_limits(root)?;
    let manifest_path = root.join(MANIFEST_FILE_NAME);
    let manifest = read_manifest_file(&manifest_path)?;
    let root = root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", root.display()))?;
    let mut plugin = manifest_to_definition(manifest, root)?;
    let prototype_plugin = is_prototype_plugin(&plugin.id);
    let bundled_builtin_slint_plugin = matches!(
        &plugin.renderer,
        PluginRendererDefinition::Slint { root_dir, .. }
            if is_bundled_builtin_slint_plugin(&plugin.id, root_dir)
    );
    if bundled_builtin_slint_plugin {
        plugin.source = PluginSource::Builtin;
    }

    if prototype_plugin {
        plugin.status = PluginStatus::Disabled("prototype widget is disabled".to_string());
        return Ok(plugin);
    }

    let parameters = plugin.parameters.clone();
    if let PluginRendererDefinition::Slint {
        root_dir,
        entry,
        component,
        definition,
    } = &mut plugin.renderer
    {
        match compile_slint_renderer(root_dir, entry, component, &parameters) {
            Ok(compiled) => {
                *definition = Some(compiled);
                plugin.status = PluginStatus::Available;
            }
            Err(error) => {
                plugin.status = PluginStatus::Unavailable(error.to_string());
            }
        }
    }

    Ok(plugin)
}

fn is_bundled_builtin_slint_plugin(plugin_id: &str, root_dir: &Path) -> bool {
    if !BUNDLED_BUILTIN_SLINT_PLUGIN_IDS.contains(&plugin_id) {
        return false;
    }

    bundled_plugin_roots().into_iter().any(|root| {
        root.canonicalize()
            .map(|bundled_plugin_root| root_dir.starts_with(bundled_plugin_root))
            .unwrap_or(false)
    })
}

pub fn manifest_to_definition(
    manifest: PluginManifest,
    root_dir: PathBuf,
) -> Result<PluginDefinition> {
    validate_manifest(&manifest)?;
    let version = Version::parse(&manifest.version).context("version must be valid SemVer")?;
    let host_api_version = VersionReq::parse(&manifest.host_api_version)
        .context("hostApiVersion must be a valid SemVer requirement")?;
    let root_dir = root_dir
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", root_dir.display()))?;
    let entry = path_stays_inside(&root_dir, &root_dir.join(&manifest.renderer.entry))?;
    if entry
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| !extension.eq_ignore_ascii_case("slint"))
        .unwrap_or(true)
    {
        bail!("renderer.entry must point to a .slint file");
    }
    let preview_images = manifest
        .preview_images
        .into_iter()
        .take(MAX_PREVIEW_IMAGES)
        .map(|image| {
            let path = root_dir.join(&image);
            path_stays_inside(&root_dir, &path)
                .with_context(|| format!("invalid previewImages entry {image}"))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(PluginDefinition {
        id: manifest.id,
        name: manifest.name,
        version,
        schema_version: manifest.schema_version,
        host_api_version,
        source: PluginSource::LocalUnsigned,
        renderer: PluginRendererDefinition::Slint {
            root_dir: root_dir.clone(),
            entry,
            component: manifest.renderer.component,
            definition: None,
        },
        default_size: manifest.default_size,
        size_policy: manifest.size_policy,
        min_symbol_limit: manifest.min_symbol_limit,
        symbol_limit: manifest.symbol_limit,
        default_symbols: normalize_default_symbols(manifest.default_symbols),
        preview_images,
        themes: manifest.themes,
        data_requirements: manifest.data_requirements,
        parameters: manifest.parameters,
        status: PluginStatus::Unavailable(SLINT_RENDERER_UNCOMPILED_REASON.to_string()),
    })
}

fn normalize_default_symbols(symbols: Vec<String>) -> Vec<String> {
    symbols
        .iter()
        .filter_map(|symbol| normalize_market_pair_key(symbol))
        .fold(Vec::new(), |mut unique, symbol| {
            if !unique.contains(&symbol) {
                unique.push(symbol);
            }
            unique
        })
}

fn compile_slint_renderer(
    root_dir: &Path,
    entry: &Path,
    component: &str,
    parameters: &[PluginParameter],
) -> Result<ComponentDefinition> {
    let mut compiler = Compiler::default();
    // The interpreter normally leaves image paths untouched and does not expose
    // them to the host. Listing resources keeps their contents unread while
    // making every resolved path available for the sandbox check below.
    compiler.set_embed_resources(EmbedResourcesKind::ListAllResources);
    compiler.set_include_paths(vec![root_dir.to_path_buf()]);
    let loader_root = root_dir.to_path_buf();
    compiler.set_file_loader(move |path| {
        let loader_root = loader_root.clone();
        let path = path.to_path_buf();
        Box::pin(async move {
            Some(
                read_plugin_slint_source(&loader_root, &path)
                    .map_err(|error| io::Error::new(io::ErrorKind::PermissionDenied, error)),
            )
        })
    });

    let entry_source = read_plugin_slint_source(root_dir, entry)?;
    let result = spin_on::spin_on(compiler.build_from_source(entry_source, entry.to_path_buf()));
    if result.has_errors() {
        bail!("Slint compilation failed: {}", diagnostics_text(&result));
    }
    validate_compiled_plugin_paths(root_dir, result.watch_paths(InternalToken))?;

    let definition = result.component(component).ok_or_else(|| {
        anyhow!(
            "renderer.component {} was not exported; available components: {}",
            component,
            result.component_names().collect::<Vec<_>>().join(", ")
        )
    })?;
    validate_slint_contract(&definition, parameters)?;
    Ok(definition)
}

fn read_plugin_slint_source(root_dir: &Path, path: &Path) -> Result<String> {
    let path = path_stays_inside(root_dir, path)?;
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| !extension.eq_ignore_ascii_case("slint"))
        .unwrap_or(true)
    {
        bail!("imported file must be a .slint file: {}", path.display());
    }
    let metadata =
        fs::metadata(&path).with_context(|| format!("failed to stat {}", path.display()))?;
    if metadata.len() > SLINT_FILE_MAX_BYTES {
        bail!(
            "Slint file exceeds {SLINT_FILE_MAX_BYTES} bytes: {}",
            path.display()
        );
    }
    let source =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    validate_slint_file_imports(&path, &source)?;
    Ok(source)
}

fn validate_slint_file_imports(source_path: &Path, source: &str) -> Result<()> {
    let mut diagnostics = BuildDiagnostics::default();
    let syntax = parser::parse(source.to_string(), Some(source_path), &mut diagnostics);
    validate_slint_file_import_nodes(&syntax, source_path)
}

fn validate_slint_file_import_nodes(node: &SyntaxNode, source_path: &Path) -> Result<()> {
    if node.kind() == SyntaxKind::ImportSpecifier {
        for token in node
            .children_with_tokens()
            .filter_map(|item| item.into_token())
        {
            if token.kind() != SyntaxKind::StringLiteral {
                continue;
            }
            let Some(import_path) = i_slint_compiler::literals::unescape_string(token.text())
            else {
                continue;
            };
            let is_slint_import = Path::new(import_path.as_str())
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("slint"));
            if !is_slint_import {
                bail!(
                    "plugin Slint source may only import .slint files; use @image-url for local images: {} in {}",
                    import_path,
                    source_path.display()
                );
            }
        }
    }

    for child in node.children() {
        validate_slint_file_import_nodes(&child, source_path)?;
    }
    Ok(())
}

fn validate_compiled_plugin_paths(root_dir: &Path, paths: &[PathBuf]) -> Result<()> {
    let root_dir = root_dir
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", root_dir.display()))?;

    for path in paths {
        let display_path = path.to_string_lossy();
        if display_path.starts_with("builtin:/") || display_path.starts_with("data:") {
            continue;
        }
        if !path.starts_with(&root_dir) {
            bail!(
                "Slint resource {} escapes plugin root {}",
                path.display(),
                root_dir.display()
            );
        }

        let canonical = path_stays_inside(&root_dir, path)
            .with_context(|| format!("invalid Slint resource {}", path.display()))?;
        let metadata = fs::metadata(&canonical)
            .with_context(|| format!("failed to stat Slint resource {}", canonical.display()))?;
        if !metadata.is_file() {
            bail!("Slint resource is not a file: {}", canonical.display());
        }

        let extension = canonical
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .ok_or_else(|| anyhow!("Slint resource has no extension: {}", canonical.display()))?;
        let max_bytes = match extension.as_str() {
            "slint" => SLINT_FILE_MAX_BYTES,
            "png" | "jpg" | "jpeg" | "svg" => ASSET_MAX_BYTES,
            _ => bail!(
                "Slint resource extension is not allowed: {}",
                canonical.display()
            ),
        };
        if metadata.len() > max_bytes {
            bail!(
                "Slint resource exceeds {max_bytes} bytes: {}",
                canonical.display()
            );
        }
    }

    Ok(())
}

fn validate_slint_contract(
    definition: &ComponentDefinition,
    parameters: &[PluginParameter],
) -> Result<()> {
    let properties = definition.properties().collect::<Vec<_>>();
    for (name, expected_type) in REQUIRED_PROPERTIES {
        let Some((_, actual_type)) = properties.iter().find(|(property, _)| property == name)
        else {
            bail!("Slint component is missing required property {name}");
        };
        if actual_type != expected_type {
            bail!(
                "Slint property {name} has type {:?}, expected {:?}",
                actual_type,
                expected_type
            );
        }
    }

    for parameter in parameters {
        let key = parameter.key();
        let name = format!("config-{key}");
        let Some((_, actual_type)) = properties.iter().find(|(property, _)| property == &name)
        else {
            bail!("Slint component is missing parameter property {name}");
        };
        let expected_type = match parameter {
            PluginParameter::Integer { .. } | PluginParameter::Decimal { .. } => ValueType::Number,
            PluginParameter::Boolean { .. } => ValueType::Bool,
            PluginParameter::Choice { .. } | PluginParameter::String { .. } => ValueType::String,
            PluginParameter::Color { .. } => ValueType::Brush,
        };
        if actual_type != &expected_type {
            bail!(
                "Slint parameter property {name} has type {:?}, expected {:?}",
                actual_type,
                expected_type
            );
        }
    }

    let callbacks = definition.callbacks().collect::<HashSet<_>>();
    for name in REQUIRED_CALLBACKS {
        if !callbacks.contains(*name) {
            bail!("Slint component is missing required callback {name}");
        }
    }

    Ok(())
}

fn diagnostics_text(result: &slint_interpreter::CompilationResult) -> String {
    let diagnostics = result
        .diagnostics()
        .map(|diagnostic| format!("{diagnostic:?}"))
        .collect::<Vec<_>>();
    if diagnostics.is_empty() {
        "no diagnostics".to_string()
    } else {
        diagnostics.join("; ")
    }
}

pub fn read_manifest_file(path: &Path) -> Result<PluginManifest> {
    let metadata =
        fs::metadata(path).with_context(|| format!("failed to read {}", path.display()))?;
    if metadata.len() > MANIFEST_MAX_BYTES {
        bail!("manifest exceeds {MANIFEST_MAX_BYTES} bytes");
    }
    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    parse_manifest(&contents)
}

pub fn path_stays_inside(root: &Path, path: &Path) -> Result<PathBuf> {
    let root = root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", root.display()))?;
    let path = path
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", path.display()))?;
    if !path.starts_with(&root) {
        return Err(anyhow!("{} escapes {}", path.display(), root.display()));
    }
    Ok(path)
}

pub fn validate_plugin_directory_limits(root: &Path) -> Result<()> {
    let root = root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", root.display()))?;
    let mut total_size = 0_u64;
    let mut visited_directories = HashSet::from([root.clone()]);
    validate_plugin_directory_limits_inner(
        &root,
        &root,
        &mut total_size,
        &mut visited_directories,
    )?;
    if total_size > PLUGIN_DIR_MAX_BYTES {
        bail!("plugin directory exceeds {PLUGIN_DIR_MAX_BYTES} bytes");
    }
    Ok(())
}

fn validate_plugin_directory_limits_inner(
    root: &Path,
    current: &Path,
    total_size: &mut u64,
    visited_directories: &mut HashSet<PathBuf>,
) -> Result<()> {
    for entry in
        fs::read_dir(current).with_context(|| format!("failed to list {}", current.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read {}", current.display()))?;
        let path = entry.path();
        let canonical = path_stays_inside(root, &path)?;
        if canonical
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            bail!("path contains ..: {}", canonical.display());
        }
        let metadata = fs::metadata(&canonical)
            .with_context(|| format!("failed to stat {}", canonical.display()))?;
        if metadata.is_dir() {
            if !visited_directories.insert(canonical.clone()) {
                bail!(
                    "plugin directory contains a symlink or junction cycle: {}",
                    path.display()
                );
            }
            validate_plugin_directory_limits_inner(
                root,
                &canonical,
                total_size,
                visited_directories,
            )?;
            continue;
        }
        if !metadata.is_file() {
            bail!("plugin path is not a file: {}", canonical.display());
        }
        let extension = canonical
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .ok_or_else(|| anyhow!("plugin file has no extension: {}", canonical.display()))?;
        if !ALLOWED_EXTENSIONS.contains(&extension.as_str()) {
            bail!(
                "plugin file extension is not allowed: {}",
                canonical.display()
            );
        }
        let file_size = metadata.len();
        if extension == "slint" && file_size > SLINT_FILE_MAX_BYTES {
            bail!(
                "Slint file exceeds {SLINT_FILE_MAX_BYTES} bytes: {}",
                canonical.display()
            );
        }
        if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "svg")
            && file_size > ASSET_MAX_BYTES
        {
            bail!(
                "asset exceeds {ASSET_MAX_BYTES} bytes: {}",
                canonical.display()
            );
        }
        *total_size += file_size;
        if *total_size > PLUGIN_DIR_MAX_BYTES {
            bail!("plugin directory exceeds {PLUGIN_DIR_MAX_BYTES} bytes");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn valid_manifest_json() -> String {
        r#"{
          "schemaVersion": 3,
          "id": "com.example.price-card",
          "name": "Example Price Card",
          "version": "1.0.0",
          "hostApiVersion": ">=0.1.0, <1.0.0",
          "renderer": {
            "kind": "slint",
            "entry": "ui/main.slint",
            "component": "ExamplePriceCard"
          },
          "permissions": {
            "network": false,
            "filesystem": false
          },
          "defaultSize": {
            "width": 220,
            "height": 140
          },
          "symbolLimit": 5,
          "dataRequirements": [
            {
              "capability": "market.price"
            }
          ]
        }"#
        .to_string()
    }

    fn valid_slint_source() -> &'static str {
        r#"
export struct QuoteRow {
    symbol: string,
    price: string,
    change: string,
    positive: bool,
}

export component ExamplePriceCard inherits Window {
    in property <string> widget-id;
    in property <[QuoteRow]> quote-rows;
    in property <string> pairs-heading-text;
    in property <string> source-text;
    in property <string> source-name-text;
    in property <string> updated-text;
    in property <string> empty-text;
    in property <bool> rtl-layout: false;
    in property <bool> pin-to-top;
    in property <bool> layout-locked;
    in property <int> widget-width;
    in property <int> widget-height;
    in property <string> theme-name;
    in property <bool> red-up-enabled;
    in property <int> content-opacity;

    callback drag-move(length, length);
    callback toggle-layout-lock();

    width: root.widget-width * 1px;
    height: root.widget-height * 1px;
    always-on-top: root.pin-to-top;
}
"#
    }

    fn valid_slint_source_with_image(image_path: &str) -> String {
        valid_slint_source().replace(
            "    always-on-top: root.pin-to-top;",
            &format!(
                "    always-on-top: root.pin-to-top;\n    Image {{ source: @image-url(\"{image_path}\"); }}"
            ),
        )
    }

    fn write_reload_test_plugin(
        plugin_dir: &Path,
        plugin_id: &str,
        version: &str,
        revision: &str,
        valid_source: bool,
    ) {
        fs::create_dir_all(plugin_dir.join("ui")).unwrap();
        let manifest = valid_manifest_json()
            .replace("com.example.price-card", plugin_id)
            .replace(
                r#""version": "1.0.0""#,
                &format!(r#""version": "{version}""#),
            );
        let source = if valid_source {
            format!("{}\n// reload revision: {revision}\n", valid_slint_source())
        } else {
            format!(
                "export component ExamplePriceCard inherits Window {{ invalid reload revision {revision}"
            )
        };
        fs::write(plugin_dir.join(MANIFEST_FILE_NAME), manifest).unwrap();
        fs::write(plugin_dir.join("ui").join("main.slint"), source).unwrap();
    }

    fn stable_reload_plan(
        tracker: &mut PluginReloadTracker,
        snapshot: PluginTreeSnapshot,
    ) -> PluginReloadPlan {
        assert!(!tracker.observe(snapshot.clone()));
        assert!(!tracker.observe(snapshot.clone()));
        assert!(tracker.observe(snapshot.clone()));
        tracker.take_pending_or_force(snapshot)
    }

    #[test]
    fn sync_user_plugin_development_guide_creates_and_overwrites_copy() {
        let state_dir = temp_plugin_root("sync-user-plugin-development-guide");
        let user_plugin_root = user_plugin_root(&state_dir);
        fs::create_dir_all(&user_plugin_root).unwrap();
        let guide_path = user_plugin_root.join(USER_PLUGIN_DEVELOPMENT_GUIDE_FILE_NAME);
        fs::write(&guide_path, "stale guide").unwrap();

        let synced_path = sync_user_plugin_development_guide(&state_dir).unwrap();

        assert_eq!(synced_path, guide_path);
        assert_eq!(
            fs::read_to_string(&guide_path).unwrap(),
            USER_PLUGIN_DEVELOPMENT_GUIDE
        );
        let _ = fs::remove_dir_all(state_dir);
    }

    #[test]
    fn user_plugin_development_guide_documents_i18n_and_rtl_contract() {
        assert!(
            USER_PLUGIN_DEVELOPMENT_GUIDE.contains("## Localization And RTL"),
            "plugin guide should document the localization and RTL contract"
        );
        assert!(
            USER_PLUGIN_DEVELOPMENT_GUIDE.contains("in property <bool> rtl-layout: false;"),
            "plugin guide should show the host-supplied RTL property"
        );
        assert!(
            USER_PLUGIN_DEVELOPMENT_GUIDE.contains("in property <string> source-name-text;"),
            "plugin guide should document the short localized source-name property"
        );
        assert!(
            USER_PLUGIN_DEVELOPMENT_GUIDE
                .contains("Visible UI copy should come from host-provided properties"),
            "plugin guide should discourage hardcoded visible English text"
        );
        assert!(
            USER_PLUGIN_DEVELOPMENT_GUIDE
                .contains("Do not compare against localized strings such as `Connecting`"),
            "plugin guide should steer plugins away from localized string comparisons"
        );
        for kind in ["boolean", "choice", "decimal", "color", "string"] {
            assert!(
                USER_PLUGIN_DEVELOPMENT_GUIDE.contains(&format!("`{kind}`")),
                "plugin guide should document the {kind} parameter"
            );
        }
        assert!(USER_PLUGIN_DEVELOPMENT_GUIDE.contains("Host API 0.2.0"));
        assert!(USER_PLUGIN_DEVELOPMENT_GUIDE.contains("Plugin diagnostics / Reload"));
        assert!(USER_PLUGIN_DEVELOPMENT_GUIDE.contains("last successfully compiled definition"));
        assert!(USER_PLUGIN_DEVELOPMENT_GUIDE.contains("increasing generations"));
        assert!(USER_PLUGIN_DEVELOPMENT_GUIDE.contains("permissions.network"));
        assert!(USER_PLUGIN_DEVELOPMENT_GUIDE.contains("permissions.filesystem"));
    }

    #[test]
    fn repo_plugin_development_guide_documents_i18n_and_rtl_contract() {
        assert!(
            REPO_PLUGIN_DEVELOPMENT_GUIDE.contains("## 本地化和 RTL"),
            "repo plugin guide should document localization and RTL expectations"
        );
        assert!(
            REPO_PLUGIN_DEVELOPMENT_GUIDE.contains("in property <bool> rtl-layout: false;"),
            "repo plugin guide should require the host-supplied RTL property"
        );
        assert!(
            REPO_PLUGIN_DEVELOPMENT_GUIDE.contains("in property <string> source-name-text;"),
            "repo plugin guide should document the short localized source-name property"
        );
        assert!(
            REPO_PLUGIN_DEVELOPMENT_GUIDE.contains("可见 UI 文案应来自宿主下发的本地化属性"),
            "repo plugin guide should discourage hardcoded visible English text"
        );
        assert!(
            REPO_PLUGIN_DEVELOPMENT_GUIDE.contains("不要比较 `Connecting`"),
            "repo plugin guide should steer plugins away from localized string comparisons"
        );
        for kind in ["boolean", "choice", "decimal", "color", "string"] {
            assert!(
                REPO_PLUGIN_DEVELOPMENT_GUIDE.contains(&format!("`{kind}`")),
                "repo plugin guide should document the {kind} parameter"
            );
        }
        assert!(REPO_PLUGIN_DEVELOPMENT_GUIDE.contains("Host API 0.2.0"));
        assert!(REPO_PLUGIN_DEVELOPMENT_GUIDE.contains("插件诊断"));
        assert!(REPO_PLUGIN_DEVELOPMENT_GUIDE.contains("last-known-good"));
        assert!(REPO_PLUGIN_DEVELOPMENT_GUIDE.contains("递增 generation"));
        assert!(REPO_PLUGIN_DEVELOPMENT_GUIDE.contains("permissions.network"));
        assert!(REPO_PLUGIN_DEVELOPMENT_GUIDE.contains("permissions.filesystem"));
    }

    #[test]
    fn builtin_quote_board_plugin_uses_quote_board_symbol_limit() {
        let plugins = builtin_plugins();
        let quote_board = plugins
            .iter()
            .find(|plugin| plugin.id == BUILTIN_QUOTE_BOARD_PLUGIN_ID)
            .unwrap();

        assert_eq!(
            quote_board.symbol_limit,
            crypto_hud_shell_state::WidgetKind::QuoteBoard.symbol_limit()
        );
        assert_eq!(quote_board.symbol_limit, 20);
    }

    #[test]
    fn parses_valid_manifest() {
        let manifest = parse_manifest(&valid_manifest_json()).unwrap();

        assert_eq!(manifest.schema_version, 3);
        assert_eq!(manifest.id, "com.example.price-card");
        assert_eq!(manifest.renderer.entry, "ui/main.slint");
        assert_eq!(manifest.size_policy, PluginSizePolicy::Fixed);
        assert_eq!(manifest.min_symbol_limit, 1);
        assert_eq!(manifest.symbol_limit, 5);
        assert!(manifest.default_symbols.is_empty());
    }

    #[test]
    fn rejects_invalid_schema_version() {
        let json = valid_manifest_json().replace(r#""schemaVersion": 3"#, r#""schemaVersion": 2"#);

        assert!(parse_manifest(&json)
            .unwrap_err()
            .to_string()
            .contains("schemaVersion"));
    }

    #[test]
    fn rejects_invalid_semver() {
        let json = valid_manifest_json().replace(r#""version": "1.0.0""#, r#""version": "v1""#);

        assert!(parse_manifest(&json)
            .unwrap_err()
            .to_string()
            .contains("SemVer"));
    }

    #[test]
    fn rejects_permissions_that_are_not_explicitly_false() {
        let json = valid_manifest_json().replace(r#""network": false"#, r#""network": true"#);

        assert!(parse_manifest(&json)
            .unwrap_err()
            .to_string()
            .contains("permissions"));
    }

    #[test]
    fn rejects_path_traversal() {
        let json = valid_manifest_json()
            .replace(r#""entry": "ui/main.slint""#, r#""entry": "../main.slint""#);

        assert!(parse_manifest(&json)
            .unwrap_err()
            .to_string()
            .contains(".."));
    }

    #[test]
    fn rejects_unsupported_capability() {
        let json = valid_manifest_json().replace(
            r#""capability": "market.price""#,
            r#""capability": "filesystem.read""#,
        );

        assert!(parse_manifest(&json)
            .unwrap_err()
            .to_string()
            .contains("unsupported"));
    }

    #[test]
    fn builtins_are_registered() {
        let catalog = PluginCatalog::builtins();

        assert!(catalog.find(BUILTIN_QUOTE_BOARD_PLUGIN_ID).is_some());
        assert!(catalog.find(BUILTIN_MINI_TICKER_PLUGIN_ID).is_some());
        assert!(catalog.errors().is_empty());
    }

    #[test]
    fn runtime_plugin_roots_prefer_the_executable_bundle_over_user_plugins() {
        let state_dir = temp_plugin_root("runtime-plugin-roots");
        let roots = plugin_roots(&state_dir);
        let executable_root = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join(BUNDLED_PLUGIN_DIRECTORY_NAME);

        assert_eq!(roots.first(), Some(&executable_root));
        assert_eq!(roots.last(), Some(&user_plugin_root(&state_dir)));
        let _ = fs::remove_dir_all(state_dir);
    }

    #[test]
    fn builtin_previews_resolve_to_existing_development_assets_in_tests() {
        for path in builtin_preview_images("quote-board") {
            assert!(path.is_file(), "missing preview asset {}", path.display());
        }
    }

    #[test]
    fn plugin_directory_cycle_guard_rejects_a_revisited_directory() {
        let root = temp_plugin_root("directory-cycle-guard");
        let child = root.join("child");
        fs::create_dir_all(&child).unwrap();
        let root = root.canonicalize().unwrap();
        let child = child.canonicalize().unwrap();
        let mut visited = HashSet::from([root.clone(), child]);
        let mut total_size = 0;

        let error =
            validate_plugin_directory_limits_inner(&root, &root, &mut total_size, &mut visited)
                .unwrap_err();

        assert!(error.to_string().contains("cycle"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn duplicate_quote_widgets_are_hidden_from_market() {
        let catalog = PluginCatalog::discover(vec![
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins")
        ]);

        let market_ids = catalog
            .market_plugins()
            .map(|plugin| plugin.id)
            .collect::<Vec<_>>();

        assert!(market_ids
            .iter()
            .any(|id| id == BUILTIN_QUOTE_BOARD_PLUGIN_ID));
        assert!(!market_ids
            .iter()
            .any(|id| id == BUILTIN_MINI_TICKER_PLUGIN_ID));
        assert!(!market_ids
            .iter()
            .any(|id| id == "com.cryptohud.market-board"));
        assert!(market_ids
            .iter()
            .any(|id| id == "com.cryptohud.market-compass"));
        assert!(!market_ids
            .iter()
            .any(|id| id == "com.example.stage3-price-card"));
    }

    #[test]
    fn current_market_widgets_are_marked_builtin() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins");

        let catalog = PluginCatalog::discover(vec![root]);

        for plugin_id in [
            BUILTIN_QUOTE_BOARD_PLUGIN_ID,
            "com.cryptohud.focus-ticker",
            "com.cryptohud.market-compass",
            "com.cryptohud.trust-card",
            "com.cryptohud.status-strip",
        ] {
            let plugin = catalog
                .find(plugin_id)
                .expect("current market widget should be registered");
            assert!(
                plugin.is_available(),
                "{plugin_id} should be available: {:?}",
                plugin.status
            );
            assert_eq!(
                plugin.source,
                PluginSource::Builtin,
                "{plugin_id} should be treated as built-in"
            );
        }
    }

    #[test]
    fn bundled_builtin_slint_plugins_have_localized_market_copy() {
        for plugin_id in BUNDLED_BUILTIN_SLINT_PLUGIN_IDS
            .iter()
            .copied()
            .chain(["com.cryptohud.market-board"])
        {
            for locale in crate::i18n::Locale::ALL {
                assert!(
                    crate::i18n::builtin_plugin_title(locale, plugin_id).is_some(),
                    "{plugin_id} should have a built-in plugin title for {locale:?}"
                );
                assert!(
                    crate::i18n::builtin_plugin_description(locale, plugin_id).is_some(),
                    "{plugin_id} should have a built-in plugin description for {locale:?}"
                );
            }
        }
    }

    #[test]
    fn bundled_builtin_id_outside_repo_plugins_stays_local() {
        let root = temp_plugin_root("bundled-builtin-id-outside-repo-plugins-stays-local");
        let plugin_dir = root.join("com.cryptohud.focus-ticker");
        fs::create_dir_all(plugin_dir.join("ui")).unwrap();
        fs::write(
            plugin_dir.join(MANIFEST_FILE_NAME),
            valid_manifest_json().replace("com.example.price-card", "com.cryptohud.focus-ticker"),
        )
        .unwrap();
        fs::write(
            plugin_dir.join("ui").join("main.slint"),
            valid_slint_source(),
        )
        .unwrap();

        let plugin = load_local_plugin(&plugin_dir).unwrap();

        assert_eq!(plugin.source, PluginSource::LocalUnsigned);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn discovers_valid_local_plugin() {
        let root = temp_plugin_root("discovers-valid-local-plugin");
        let plugin_dir = root.join("com.example.price-card");
        fs::create_dir_all(plugin_dir.join("ui")).unwrap();
        fs::write(plugin_dir.join(MANIFEST_FILE_NAME), valid_manifest_json()).unwrap();
        fs::write(
            plugin_dir.join("ui").join("main.slint"),
            valid_slint_source(),
        )
        .unwrap();

        let catalog = PluginCatalog::discover(vec![root.clone()]);

        let plugin = catalog.find("com.example.price-card").unwrap();
        assert!(plugin.is_available());
        assert!(catalog.errors().is_empty(), "{:?}", catalog.errors());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn discovers_plugin_with_all_extended_parameter_property_types() {
        let root = temp_plugin_root("discovers-extended-parameter-types");
        let plugin_dir = root.join("com.example.price-card");
        fs::create_dir_all(plugin_dir.join("ui")).unwrap();
        let manifest = valid_manifest_json()
            .replace(
                r#""hostApiVersion": ">=0.1.0, <1.0.0""#,
                r#""hostApiVersion": ">=0.2.0, <1.0.0""#,
            )
            .replace(
                r#""dataRequirements": ["#,
                r##""parameters": [
                    { "kind": "boolean", "key": "show-label", "name": "Show label", "default": true },
                    { "kind": "choice", "key": "density", "name": "Density", "default": "compact", "options": [
                        { "value": "compact", "name": "Compact" },
                        { "value": "comfortable", "name": "Comfortable" }
                    ] },
                    { "kind": "decimal", "key": "line-width", "name": "Line width", "default": 1.5, "minimum": 0.5, "maximum": 4.0, "step": 0.25 },
                    { "kind": "color", "key": "accent", "name": "Accent", "default": "#3366ff" },
                    { "kind": "string", "key": "caption", "name": "Caption", "default": "Market", "maxLength": 24 }
                ],
                "dataRequirements": ["##,
            );
        let source = valid_slint_source().replace(
            "    in property <int> content-opacity;",
            r#"    in property <int> content-opacity;
    in property <bool> config-show-label;
    in property <string> config-density;
    in property <float> config-line-width;
    in property <color> config-accent;
    in property <string> config-caption;"#,
        );
        fs::write(plugin_dir.join(MANIFEST_FILE_NAME), manifest).unwrap();
        fs::write(plugin_dir.join("ui").join("main.slint"), source).unwrap();

        let catalog = PluginCatalog::discover(vec![root.clone()]);

        let definition = catalog.find("com.example.price-card").unwrap();
        assert!(definition.is_available(), "{:?}", definition.status);
        assert_eq!(definition.parameters.len(), 5);
        assert!(catalog.errors().is_empty(), "{:?}", catalog.errors());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_tree_snapshot_tracks_user_plugin_changes() {
        let state_dir = temp_plugin_root("plugin-tree-fingerprint");
        let before = plugin_tree_snapshot(&state_dir);
        let plugin_dir = user_plugin_root(&state_dir).join("com.example.changed");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(plugin_dir.join(MANIFEST_FILE_NAME), "{}").unwrap();

        let after = plugin_tree_snapshot(&state_dir);

        assert_ne!(before, after);
        let _ = fs::remove_dir_all(state_dir);
    }

    #[test]
    fn plugin_tree_snapshot_hashes_same_length_file_replacements() {
        let root = temp_plugin_root("plugin-tree-content-hash");
        let plugin_dir = root.join("com.example.changed");
        fs::create_dir_all(&plugin_dir).unwrap();
        let manifest_path = plugin_dir.join(MANIFEST_FILE_NAME);
        fs::write(&manifest_path, "{}").unwrap();
        let before = scan_plugin_roots(std::slice::from_ref(&root));

        fs::write(&manifest_path, "[]").unwrap();
        let after = scan_plugin_roots(std::slice::from_ref(&root));

        assert_ne!(before, after, "content hash must detect equal-length saves");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_reload_tracker_discards_stale_generations_after_rapid_saves() {
        let state_dir = temp_plugin_root("plugin-reload-generations");
        let initial = plugin_tree_snapshot(&state_dir);
        let mut tracker = PluginReloadTracker::new(initial);
        let plugin_dir = user_plugin_root(&state_dir).join("com.example.changed");
        fs::create_dir_all(&plugin_dir).unwrap();
        let manifest_path = plugin_dir.join(MANIFEST_FILE_NAME);
        fs::write(&manifest_path, "{}").unwrap();

        let first_snapshot = plugin_tree_snapshot(&state_dir);
        let first = stable_reload_plan(&mut tracker, first_snapshot);
        fs::write(&manifest_path, "[]").unwrap();

        assert!(!first.is_current(&state_dir));
        tracker.finish(first.generation, false);
        let second_snapshot = plugin_tree_snapshot(&state_dir);
        let second = stable_reload_plan(&mut tracker, second_snapshot);
        assert!(second.generation > first.generation);
        assert!(second.is_current(&state_dir));
        tracker.finish(second.generation, true);

        let _ = fs::remove_dir_all(state_dir);
    }

    #[test]
    fn pending_reload_is_not_rebased_onto_an_unstable_newer_snapshot() {
        let state_dir = temp_plugin_root("plugin-reload-pending-generation");
        let initial = plugin_tree_snapshot(&state_dir);
        let mut tracker = PluginReloadTracker::new(initial);
        let plugin_dir = user_plugin_root(&state_dir).join("com.example.changed");
        fs::create_dir_all(&plugin_dir).unwrap();
        let manifest_path = plugin_dir.join(MANIFEST_FILE_NAME);
        fs::write(&manifest_path, "{}").unwrap();
        let first_snapshot = plugin_tree_snapshot(&state_dir);

        assert!(!tracker.observe(first_snapshot.clone()));
        assert!(!tracker.observe(first_snapshot.clone()));
        assert!(tracker.observe(first_snapshot));

        fs::write(&manifest_path, "[]").unwrap();
        let newer_snapshot = plugin_tree_snapshot(&state_dir);
        let pending = tracker.take_pending_or_force(newer_snapshot.clone());

        assert_eq!(pending.generation, 1);
        assert!(!pending.is_current(&state_dir));
        tracker.finish(pending.generation, false);
        assert!(!tracker.observe(newer_snapshot.clone()));
        assert!(!tracker.observe(newer_snapshot.clone()));
        assert!(tracker.observe(newer_snapshot.clone()));
        let replacement = tracker.take_pending_or_force(newer_snapshot);
        assert!(replacement.generation > pending.generation);
        assert!(replacement.is_current(&state_dir));

        let _ = fs::remove_dir_all(state_dir);
    }

    #[test]
    fn plugin_reload_tracker_retries_an_uncommitted_snapshot() {
        let initial = PluginTreeSnapshot::default();
        let mut tracker = PluginReloadTracker::new(initial);
        let mut changed = PluginTreeSnapshot::default();
        changed.scan_errors.push(PluginCatalogError {
            path: PathBuf::from("changed"),
            message: "changed".to_string(),
        });

        let first = stable_reload_plan(&mut tracker, changed.clone());
        tracker.finish(first.generation, false);

        assert!(!tracker.observe(changed.clone()));
        assert!(tracker.observe(changed.clone()));
        let retry = tracker.take_pending_or_force(changed.clone());
        assert!(retry.generation > first.generation);
        tracker.finish(retry.generation, true);
        assert!(!tracker.observe(changed));
    }

    #[test]
    fn incremental_reload_retains_last_good_then_recovers_deletes_and_reappears() {
        let root = temp_plugin_root("incremental-reload-lifecycle");
        let plugin_a_dir = root.join("com.example.plugin-a");
        let plugin_b_dir = root.join("com.example.plugin-b");
        let plugin_a_id = "com.example.plugin-a";
        let plugin_b_id = "com.example.plugin-b";
        write_reload_test_plugin(&plugin_a_dir, plugin_a_id, "1.0.0", "a-v1", true);
        write_reload_test_plugin(&plugin_b_dir, plugin_b_id, "1.0.0", "b-v1", true);
        let initial_snapshot = scan_plugin_roots(std::slice::from_ref(&root));
        let mut tracker = PluginReloadTracker::new(initial_snapshot);
        let catalog = PluginCatalog::discover(vec![root.clone()]);
        let plugin_b_revision = catalog
            .effective_revisions
            .borrow()
            .get(plugin_b_id)
            .cloned();

        write_reload_test_plugin(&plugin_a_dir, plugin_a_id, "1.1.0", "a-broken", false);
        let failed_plan =
            stable_reload_plan(&mut tracker, scan_plugin_roots(std::slice::from_ref(&root)));
        assert_eq!(failed_plan.changed_sources.len(), 1);
        let failed = catalog.reload_incremental(&failed_plan);
        tracker.finish(failed_plan.generation, true);

        assert!(failed.changed_plugin_ids.is_empty());
        assert_eq!(
            failed.retained_plugin_ids,
            BTreeSet::from([plugin_a_id.to_string()])
        );
        let retained = failed.catalog.find(plugin_a_id).unwrap();
        assert!(retained.is_available());
        assert_eq!(retained.version, Version::new(1, 0, 0));
        let canonical_plugin_a_dir = plugin_a_dir.canonicalize().unwrap();
        assert!(failed
            .catalog
            .errors()
            .iter()
            .any(|error| error.path.starts_with(&canonical_plugin_a_dir)));
        assert_eq!(
            failed
                .catalog
                .effective_revisions
                .borrow()
                .get(plugin_b_id)
                .cloned(),
            plugin_b_revision
        );

        write_reload_test_plugin(&plugin_a_dir, plugin_a_id, "2.0.0", "a-v2", true);
        let recovered_plan =
            stable_reload_plan(&mut tracker, scan_plugin_roots(std::slice::from_ref(&root)));
        let recovered = failed.catalog.reload_incremental(&recovered_plan);
        tracker.finish(recovered_plan.generation, true);
        assert_eq!(
            recovered.changed_plugin_ids,
            BTreeSet::from([plugin_a_id.to_string()])
        );
        assert!(recovered.retained_plugin_ids.is_empty());
        assert_eq!(
            recovered.catalog.find(plugin_a_id).unwrap().version,
            Version::new(2, 0, 0)
        );
        assert!(recovered.catalog.errors().is_empty());
        assert_eq!(
            recovered
                .catalog
                .effective_revisions
                .borrow()
                .get(plugin_b_id)
                .cloned(),
            plugin_b_revision
        );

        fs::remove_dir_all(&plugin_a_dir).unwrap();
        let removed_plan =
            stable_reload_plan(&mut tracker, scan_plugin_roots(std::slice::from_ref(&root)));
        let removed = recovered.catalog.reload_incremental(&removed_plan);
        tracker.finish(removed_plan.generation, true);
        assert_eq!(
            removed.changed_plugin_ids,
            BTreeSet::from([plugin_a_id.to_string()])
        );
        assert!(removed.catalog.find(plugin_a_id).is_none());

        write_reload_test_plugin(&plugin_a_dir, plugin_a_id, "3.0.0", "a-v3", true);
        let reappeared_plan =
            stable_reload_plan(&mut tracker, scan_plugin_roots(std::slice::from_ref(&root)));
        let reappeared = removed.catalog.reload_incremental(&reappeared_plan);
        tracker.finish(reappeared_plan.generation, true);
        assert_eq!(
            reappeared.changed_plugin_ids,
            BTreeSet::from([plugin_a_id.to_string()])
        );
        assert_eq!(
            reappeared.catalog.find(plugin_a_id).unwrap().version,
            Version::new(3, 0, 0)
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn incremental_reload_keeps_previous_id_when_manifest_changes_to_a_duplicate() {
        let root = temp_plugin_root("incremental-reload-duplicate-id");
        let first_dir = root.join("a-first");
        let second_dir = root.join("b-second");
        let first_id = "com.example.first";
        let second_id = "com.example.second";
        write_reload_test_plugin(&first_dir, first_id, "1.0.0", "first", true);
        write_reload_test_plugin(&second_dir, second_id, "1.0.0", "second", true);
        let initial_snapshot = scan_plugin_roots(std::slice::from_ref(&root));
        let mut tracker = PluginReloadTracker::new(initial_snapshot);
        let catalog = PluginCatalog::discover(vec![root.clone()]);

        write_reload_test_plugin(&second_dir, first_id, "2.0.0", "duplicate", true);
        let plan = stable_reload_plan(&mut tracker, scan_plugin_roots(std::slice::from_ref(&root)));
        let reload = catalog.reload_incremental(&plan);

        assert!(reload.changed_plugin_ids.is_empty());
        assert_eq!(
            reload.retained_plugin_ids,
            BTreeSet::from([second_id.to_string()])
        );
        assert_eq!(
            reload.catalog.find(first_id).unwrap().version,
            Version::new(1, 0, 0)
        );
        assert_eq!(
            reload.catalog.find(second_id).unwrap().version,
            Version::new(1, 0, 0)
        );
        assert!(reload
            .catalog
            .errors()
            .iter()
            .any(|error| error.message.contains("duplicate plugin id")));

        tracker.finish(plan.generation, true);
        fs::remove_dir_all(&first_dir).unwrap();
        let resolved_plan =
            stable_reload_plan(&mut tracker, scan_plugin_roots(std::slice::from_ref(&root)));
        assert_eq!(resolved_plan.changed_sources.len(), 1);
        let resolved = reload.catalog.reload_incremental(&resolved_plan);
        tracker.finish(resolved_plan.generation, true);

        assert_eq!(resolved.compiled_source_count, 0);
        assert_eq!(
            resolved.changed_plugin_ids,
            BTreeSet::from([first_id.to_string(), second_id.to_string()])
        );
        assert_eq!(
            resolved.catalog.find(first_id).unwrap().version,
            Version::new(2, 0, 0)
        );
        assert!(resolved.catalog.find(second_id).is_none());
        assert!(resolved
            .catalog
            .errors()
            .iter()
            .all(|error| !error.message.contains("duplicate plugin id")));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn incremental_reload_preserves_candidates_under_an_incomplete_root_scan() {
        let root = temp_plugin_root("incremental-reload-incomplete-root");
        let plugin_dir = root.join("com.example.plugin-a");
        let plugin_id = "com.example.plugin-a";
        write_reload_test_plugin(&plugin_dir, plugin_id, "1.0.0", "a-v1", true);
        let initial_snapshot = scan_plugin_roots(std::slice::from_ref(&root));
        let mut tracker = PluginReloadTracker::new(initial_snapshot);
        let catalog = PluginCatalog::discover(vec![root.clone()]);
        let mut incomplete = PluginTreeSnapshot::default();
        incomplete.incomplete_root_ranks.insert(0);
        incomplete.scan_errors.push(PluginCatalogError {
            path: root.clone(),
            message: "temporary access failure".to_string(),
        });

        let plan = stable_reload_plan(&mut tracker, incomplete);
        let reload = catalog.reload_incremental(&plan);
        tracker.finish(plan.generation, true);

        assert!(reload.changed_plugin_ids.is_empty());
        assert_eq!(reload.compiled_source_count, 0);
        assert_eq!(
            reload.retained_plugin_ids,
            BTreeSet::from([plugin_id.to_string()])
        );
        assert_eq!(
            reload.catalog.find(plugin_id).unwrap().version,
            Version::new(1, 0, 0)
        );
        assert!(reload
            .catalog
            .errors()
            .iter()
            .any(|error| error.message.contains("temporary access failure")));

        fs::remove_dir_all(&plugin_dir).unwrap();
        let confirmed_plan =
            stable_reload_plan(&mut tracker, scan_plugin_roots(std::slice::from_ref(&root)));
        let confirmed = reload.catalog.reload_incremental(&confirmed_plan);
        tracker.finish(confirmed_plan.generation, true);
        assert_eq!(
            confirmed.changed_plugin_ids,
            BTreeSet::from([plugin_id.to_string()])
        );
        assert!(confirmed.catalog.find(plugin_id).is_none());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn incremental_reload_compiles_complete_candidates_despite_an_unrelated_root_error() {
        let root = temp_plugin_root("incremental-reload-partial-root-scan");
        let plugin_dir = root.join("com.example.plugin-a");
        let plugin_id = "com.example.plugin-a";
        write_reload_test_plugin(&plugin_dir, plugin_id, "1.0.0", "a-v1", true);
        let initial_snapshot = scan_plugin_roots(std::slice::from_ref(&root));
        let mut tracker = PluginReloadTracker::new(initial_snapshot);
        let catalog = PluginCatalog::discover(vec![root.clone()]);

        write_reload_test_plugin(&plugin_dir, plugin_id, "2.0.0", "a-v2", true);
        let mut partial = scan_plugin_roots(std::slice::from_ref(&root));
        partial.incomplete_root_ranks.insert(0);
        partial.scan_errors.push(PluginCatalogError {
            path: root.join("unrelated-entry"),
            message: "temporary access failure".to_string(),
        });

        let startup_catalog = PluginCatalog::from_snapshot(partial.clone(), None, 0);
        assert_eq!(
            startup_catalog.find(plugin_id).unwrap().version,
            Version::new(2, 0, 0)
        );
        let plan = stable_reload_plan(&mut tracker, partial);
        let reload = catalog.reload_incremental(&plan);
        tracker.finish(plan.generation, true);

        assert_eq!(reload.compiled_source_count, 1);
        assert_eq!(
            reload.changed_plugin_ids,
            BTreeSet::from([plugin_id.to_string()])
        );
        assert_eq!(
            reload.catalog.find(plugin_id).unwrap().version,
            Version::new(2, 0, 0)
        );
        assert!(reload
            .catalog
            .errors()
            .iter()
            .any(|error| error.message.contains("temporary access failure")));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recovered_source_scan_retries_even_when_its_fingerprint_is_unchanged() {
        let root = temp_plugin_root("incremental-reload-recovered-source-scan");
        let plugin_dir = root.join("com.example.plugin-a");
        let plugin_id = "com.example.plugin-a";
        write_reload_test_plugin(&plugin_dir, plugin_id, "1.0.0", "a-v1", true);
        let initial_snapshot = scan_plugin_roots(std::slice::from_ref(&root));
        let mut tracker = PluginReloadTracker::new(initial_snapshot);
        let catalog = PluginCatalog::discover(vec![root.clone()]);

        write_reload_test_plugin(&plugin_dir, plugin_id, "2.0.0", "a-v2", true);
        let recovered_snapshot = scan_plugin_roots(std::slice::from_ref(&root));
        let source = recovered_snapshot.candidates.keys().next().unwrap().clone();
        let mut incomplete = recovered_snapshot.clone();
        incomplete.incomplete_sources.insert(source);
        incomplete.scan_errors.push(PluginCatalogError {
            path: plugin_dir.clone(),
            message: "temporary fingerprint failure".to_string(),
        });

        let incomplete_plan = stable_reload_plan(&mut tracker, incomplete);
        let retained = catalog.reload_incremental(&incomplete_plan);
        tracker.finish(incomplete_plan.generation, true);
        assert!(retained.changed_plugin_ids.is_empty());
        assert_eq!(
            retained.catalog.find(plugin_id).unwrap().version,
            Version::new(1, 0, 0)
        );

        let recovered_plan = stable_reload_plan(&mut tracker, recovered_snapshot);
        assert_eq!(recovered_plan.changed_sources.len(), 1);
        let recovered = retained.catalog.reload_incremental(&recovered_plan);
        tracker.finish(recovered_plan.generation, true);
        assert_eq!(recovered.compiled_source_count, 1);
        assert_eq!(
            recovered.catalog.find(plugin_id).unwrap().version,
            Version::new(2, 0, 0)
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn non_file_manifest_keeps_last_good_until_a_valid_file_returns() {
        let root = temp_plugin_root("incremental-reload-manifest-directory");
        let plugin_dir = root.join("com.example.plugin-a");
        let plugin_id = "com.example.plugin-a";
        write_reload_test_plugin(&plugin_dir, plugin_id, "1.0.0", "a-v1", true);
        let initial_snapshot = scan_plugin_roots(std::slice::from_ref(&root));
        let mut tracker = PluginReloadTracker::new(initial_snapshot);
        let catalog = PluginCatalog::discover(vec![root.clone()]);
        let manifest_path = plugin_dir.join(MANIFEST_FILE_NAME);

        fs::remove_file(&manifest_path).unwrap();
        fs::create_dir(&manifest_path).unwrap();
        let incomplete = scan_plugin_roots(std::slice::from_ref(&root));
        assert!(incomplete.incomplete_root_ranks.contains(&0));
        let plan = stable_reload_plan(&mut tracker, incomplete);
        let retained = catalog.reload_incremental(&plan);
        tracker.finish(plan.generation, true);

        assert!(retained.changed_plugin_ids.is_empty());
        assert_eq!(
            retained.retained_plugin_ids,
            BTreeSet::from([plugin_id.to_string()])
        );
        assert_eq!(
            retained.catalog.find(plugin_id).unwrap().version,
            Version::new(1, 0, 0)
        );
        assert!(retained
            .catalog
            .errors()
            .iter()
            .any(|error| error.message.contains("not a regular file")));

        fs::remove_dir(&manifest_path).unwrap();
        write_reload_test_plugin(&plugin_dir, plugin_id, "2.0.0", "a-v2", true);
        let recovered_plan =
            stable_reload_plan(&mut tracker, scan_plugin_roots(std::slice::from_ref(&root)));
        let recovered = retained.catalog.reload_incremental(&recovered_plan);
        tracker.finish(recovered_plan.generation, true);
        assert_eq!(
            recovered.changed_plugin_ids,
            BTreeSet::from([plugin_id.to_string()])
        );
        assert_eq!(
            recovered.catalog.find(plugin_id).unwrap().version,
            Version::new(2, 0, 0)
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn failed_duplicate_id_edit_never_replaces_the_last_good_definition() {
        let root = temp_plugin_root("incremental-reload-failed-duplicate-id");
        let first_dir = root.join("a-first");
        let second_dir = root.join("b-second");
        let first_id = "com.example.first";
        let second_id = "com.example.second";
        write_reload_test_plugin(&first_dir, first_id, "1.0.0", "first", true);
        write_reload_test_plugin(&second_dir, second_id, "1.0.0", "second", true);
        let initial_snapshot = scan_plugin_roots(std::slice::from_ref(&root));
        let mut tracker = PluginReloadTracker::new(initial_snapshot);
        let catalog = PluginCatalog::discover(vec![root.clone()]);

        write_reload_test_plugin(&second_dir, first_id, "2.0.0", "broken", false);
        let failed_plan =
            stable_reload_plan(&mut tracker, scan_plugin_roots(std::slice::from_ref(&root)));
        let failed = catalog.reload_incremental(&failed_plan);
        tracker.finish(failed_plan.generation, true);
        assert!(failed.changed_plugin_ids.is_empty());
        assert_eq!(
            failed.catalog.find(second_id).unwrap().version,
            Version::new(1, 0, 0)
        );

        fs::remove_dir_all(&first_dir).unwrap();
        let resolved_plan =
            stable_reload_plan(&mut tracker, scan_plugin_roots(std::slice::from_ref(&root)));
        let resolved = failed.catalog.reload_incremental(&resolved_plan);
        tracker.finish(resolved_plan.generation, true);

        assert_eq!(
            resolved.changed_plugin_ids,
            BTreeSet::from([first_id.to_string()])
        );
        assert!(resolved.catalog.find(first_id).is_none());
        assert_eq!(
            resolved.catalog.find(second_id).unwrap().version,
            Version::new(1, 0, 0)
        );
        assert!(resolved
            .catalog
            .errors()
            .iter()
            .any(|error| error.message.contains("Slint compilation failed")));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cancellable_plugin_scan_stops_before_touching_roots() {
        let cancelled = AtomicBool::new(true);
        let root = temp_plugin_root("cancelled-plugin-scan");

        assert!(
            scan_plugin_roots_with_cancel(std::slice::from_ref(&root), Some(&cancelled)).is_none()
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_diagnostics_redact_local_absolute_roots() {
        let state_dir = temp_plugin_root("redacted-plugin-diagnostics");
        let plugin_dir = user_plugin_root(&state_dir).join("com.example.invalid");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(plugin_dir.join(MANIFEST_FILE_NAME), "{}").unwrap();
        let catalog = PluginCatalog::load(&state_dir);

        let diagnostics = catalog.diagnostic_messages(&state_dir);

        assert!(diagnostics
            .iter()
            .any(|message| message.contains("<user-plugins>")));
        assert!(diagnostics
            .iter()
            .all(|message| !message.contains(&state_dir.to_string_lossy().to_string())));
        let _ = fs::remove_dir_all(state_dir);
    }

    #[test]
    fn plugin_diagnostics_redact_debug_escaped_compiler_paths() {
        let state_dir = temp_plugin_root("redacted-plugin-compiler-diagnostics");
        let plugin_dir = user_plugin_root(&state_dir).join("com.example.invalid");
        write_reload_test_plugin(&plugin_dir, "com.example.invalid", "1.0.0", "broken", false);
        let catalog = PluginCatalog::load(&state_dir);

        let diagnostics = catalog.diagnostic_messages(&state_dir).join("\n");
        let canonical_root = user_plugin_root(&state_dir).canonicalize().unwrap();
        let canonical_text = canonical_root.to_string_lossy();
        let debug_escaped = canonical_text.escape_debug().to_string();

        assert!(diagnostics.contains("<user-plugins>"));
        assert!(!diagnostics.contains(canonical_text.as_ref()));
        assert!(!diagnostics.contains(&debug_escaped));
        #[cfg(windows)]
        assert!(!diagnostics.contains(r"?\\<user-plugins>"));
        let _ = fs::remove_dir_all(state_dir);
    }

    #[test]
    fn catalog_snapshot_can_be_replaced_in_place() {
        let catalog = PluginCatalog::builtins();
        assert!(!catalog.plugins().is_empty());

        catalog.replace_with(PluginCatalog::from_plugins_for_tests(Vec::new()));

        assert!(catalog.plugins().is_empty());
        assert!(catalog.find(BUILTIN_QUOTE_BOARD_PLUGIN_ID).is_none());
    }

    #[test]
    fn discovers_plugin_with_nested_local_image_resource() {
        let root = temp_plugin_root("discovers-plugin-with-nested-local-image-resource");
        let plugin_dir = root.join("com.example.price-card");
        let asset_dir = plugin_dir.join("ui").join("assets");
        fs::create_dir_all(&asset_dir).unwrap();
        fs::write(plugin_dir.join(MANIFEST_FILE_NAME), valid_manifest_json()).unwrap();
        fs::write(
            plugin_dir.join("ui").join("main.slint"),
            valid_slint_source_with_image("assets/icon.svg"),
        )
        .unwrap();
        fs::write(
            asset_dir.join("icon.svg"),
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"/>"#,
        )
        .unwrap();

        let catalog = PluginCatalog::discover(vec![root.clone()]);

        let plugin = catalog.find("com.example.price-card").unwrap();
        assert!(plugin.is_available(), "{:?}", plugin.status);
        assert!(catalog.errors().is_empty(), "{:?}", catalog.errors());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn external_slint_image_resource_marks_plugin_unavailable() {
        let root = temp_plugin_root("external-slint-image-resource");
        let plugin_dir = root.join("com.example.price-card");
        fs::create_dir_all(plugin_dir.join("ui")).unwrap();
        fs::write(root.join("outside.png"), b"not an image").unwrap();
        fs::write(plugin_dir.join(MANIFEST_FILE_NAME), valid_manifest_json()).unwrap();
        fs::write(
            plugin_dir.join("ui").join("main.slint"),
            valid_slint_source_with_image("../../outside.png"),
        )
        .unwrap();

        let catalog = PluginCatalog::discover(vec![root.clone()]);

        let plugin = catalog.find("com.example.price-card").unwrap();
        assert!(!plugin.is_available());
        assert_eq!(catalog.errors().len(), 1);
        assert!(catalog.errors()[0].message.contains("escapes plugin root"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn external_slint_file_import_marks_plugin_unavailable() {
        let root = temp_plugin_root("external-slint-file-import");
        let plugin_dir = root.join("com.example.price-card");
        fs::create_dir_all(plugin_dir.join("ui")).unwrap();
        fs::write(root.join("outside.ttf"), b"not a font").unwrap();
        fs::write(plugin_dir.join(MANIFEST_FILE_NAME), valid_manifest_json()).unwrap();
        fs::write(
            plugin_dir.join("ui").join("main.slint"),
            format!("import \"../../outside.ttf\";\n{}", valid_slint_source()),
        )
        .unwrap();

        let catalog = PluginCatalog::discover(vec![root.clone()]);

        let plugin = catalog.find("com.example.price-card").unwrap();
        assert!(!plugin.is_available());
        assert_eq!(catalog.errors().len(), 1);
        assert!(catalog.errors()[0]
            .message
            .contains("may only import .slint files"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn slint_contract_failure_marks_plugin_unavailable() {
        let root = temp_plugin_root("slint-contract-failure-marks-plugin-unavailable");
        let plugin_dir = root.join("com.example.price-card");
        fs::create_dir_all(plugin_dir.join("ui")).unwrap();
        fs::write(plugin_dir.join(MANIFEST_FILE_NAME), valid_manifest_json()).unwrap();
        fs::write(
            plugin_dir.join("ui").join("main.slint"),
            "export component ExamplePriceCard inherits Window {}",
        )
        .unwrap();

        let catalog = PluginCatalog::discover(vec![root.clone()]);

        let plugin = catalog.find("com.example.price-card").unwrap();
        assert!(!plugin.is_available());
        assert_eq!(catalog.errors().len(), 1);
        assert!(catalog.errors()[0].message.contains("widget-id"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn invalid_local_plugin_is_collected_as_error() {
        let root = temp_plugin_root("invalid-local-plugin-is-collected-as-error");
        let plugin_dir = root.join("com.example.bad");
        fs::create_dir_all(plugin_dir.join("ui")).unwrap();
        let json = valid_manifest_json().replace(r#""schemaVersion": 3"#, r#""schemaVersion": 1"#);
        fs::write(plugin_dir.join(MANIFEST_FILE_NAME), json).unwrap();
        fs::write(
            plugin_dir.join("ui").join("main.slint"),
            valid_slint_source(),
        )
        .unwrap();

        let catalog = PluginCatalog::discover(vec![root.clone()]);

        assert!(catalog.find("com.example.bad").is_none());
        assert_eq!(catalog.errors().len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_oversized_manifest_file() {
        let root = temp_plugin_root("rejects-oversized-manifest-file");
        let plugin_dir = root.join("com.example.too-large");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join(MANIFEST_FILE_NAME),
            " ".repeat(MANIFEST_MAX_BYTES as usize + 1),
        )
        .unwrap();

        let catalog = PluginCatalog::discover(vec![root.clone()]);

        assert_eq!(catalog.errors().len(), 1);
        assert!(catalog.errors()[0].message.contains("manifest exceeds"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn discovers_repo_local_plugins() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins");

        let catalog = PluginCatalog::discover(vec![root]);

        for plugin_id in [
            "com.cryptohud.focus-ticker",
            "com.cryptohud.market-compass",
            "com.cryptohud.trust-card",
            "com.cryptohud.status-strip",
        ] {
            let plugin = catalog
                .find(plugin_id)
                .expect("plugin should be discovered");
            assert!(
                plugin.is_available(),
                "{plugin_id} should be available: {:?}",
                plugin.status
            );
        }
        for plugin_id in [
            "com.cryptohud.market-board",
            "com.example.stage3-price-card",
        ] {
            let plugin = catalog
                .find(plugin_id)
                .expect("prototype plugin should still be discoverable");
            assert!(
                matches!(plugin.status, PluginStatus::Disabled(_)),
                "{plugin_id} should be disabled"
            );
        }
        assert!(catalog.errors().is_empty(), "{:?}", catalog.errors());
    }

    #[test]
    fn status_strip_manifest_declares_symbol_grid_size_policy() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins");
        let catalog = PluginCatalog::discover(vec![root]);
        let plugin = catalog.find("com.cryptohud.status-strip").unwrap();

        assert_eq!(
            plugin.size_policy,
            PluginSizePolicy::SymbolGrid {
                cell_size: PluginSize {
                    width: 122,
                    height: 84
                },
                content_padding: PluginSize {
                    width: 8,
                    height: 8
                },
                columns: Some(5),
                rows: None
            }
        );
    }

    #[test]
    fn market_board_manifest_declares_default_symbols() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins");
        let catalog = PluginCatalog::discover(vec![root]);
        let plugin = catalog.find("com.cryptohud.market-board").unwrap();

        assert_eq!(
            plugin.default_symbols,
            vec![
                "binance:spot:BTC/USDT",
                "binance:spot:ETH/USDT",
                "binance:spot:SOL/USDT",
            ]
        );
    }

    #[test]
    fn market_compass_manifest_declares_preview_images() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins");
        let catalog = PluginCatalog::discover(vec![root]);
        let plugin = catalog.find("com.cryptohud.market-compass").unwrap();

        assert_eq!(
            plugin.preview_images.len(),
            2,
            "market compass should expose light and dark thumbnails"
        );
        for image_path in &plugin.preview_images {
            assert!(
                image_path.exists(),
                "market compass preview image should exist: {}",
                image_path.display()
            );
            let image = slint::Image::load_from_path(image_path).unwrap_or_else(|error| {
                panic!(
                    "market compass preview image should load: {}: {error}",
                    image_path.display()
                )
            });
            let pixels = image.to_rgba8().unwrap_or_else(|| {
                panic!(
                    "market compass preview image should expose RGBA pixels: {}",
                    image_path.display()
                )
            });
            assert_eq!(
                (pixels.width(), pixels.height()),
                (480, 480),
                "market compass preview should match the natural widget canvas"
            );
            if image_path.file_name().and_then(|name| name.to_str()) == Some("preview-dark.png") {
                let width = pixels.width() as usize;
                let data = pixels.as_slice();
                let bright_cyan_pixels = (258..349)
                    .flat_map(|y| (98..383).map(move |x| data[y * width + x]))
                    .filter(|pixel| {
                        pixel.r < 100 && pixel.g > 170 && pixel.b > 130 && pixel.a > 180
                    })
                    .count();
                assert!(
                    bright_cyan_pixels < 2_000,
                    "dark preview chart area should not contain a bright cyan panel"
                );
            }
        }
    }

    #[test]
    fn market_compass_declares_configurable_switch_interval() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins");
        let catalog = PluginCatalog::discover(vec![root]);
        let plugin = catalog.find("com.cryptohud.market-compass").unwrap();

        assert!(matches!(
            &plugin.parameters[0],
            PluginParameter::Integer {
                key,
                default: 5,
                minimum: 1,
                maximum: 60,
                unit,
                ..
            } if key == "switch-interval-seconds" && unit == "s"
        ));
        let source = repo_plugin_ui_source("com.cryptohud.market-compass");
        assert!(source.contains("in property <int> config-switch-interval-seconds: 5;"));
        assert!(source.contains("interval: root.config-switch-interval-seconds * 1s;"));
    }

    #[test]
    fn repo_plugins_accept_host_supplied_scale_and_root_drag_layer() {
        for plugin_id in HOST_SCALE_REPO_PLUGIN_IDS {
            let source = repo_plugin_ui_source(plugin_id);

            assert!(
                source.contains("in property <float> widget-scale: 1.0;"),
                "{plugin_id} should accept the host supplied widget scale"
            );
            assert!(
                source.contains("property <float> content-scale: root.widget-scale;"),
                "{plugin_id} should not infer content scale from window dimensions"
            );
            assert!(
                source.find("drag_area := TouchArea").unwrap()
                    < source
                        .find("card := Rectangle")
                        .or_else(|| source.find("canvas := Rectangle"))
                        .unwrap(),
                "{plugin_id} should keep dragging in root window coordinates"
            );
        }
    }

    #[test]
    fn repo_plugins_accept_host_supplied_rtl_layout() {
        for plugin_id in HOST_SCALE_REPO_PLUGIN_IDS {
            let source = repo_plugin_ui_source(plugin_id);

            assert!(
                source.contains("in property <bool> rtl-layout: false;"),
                "{plugin_id} should accept the host supplied RTL layout flag"
            );
        }
    }

    #[test]
    fn repo_plugin_visible_text_literals_are_limited_to_non_localized_tokens() {
        let allowed = [" · "];

        for plugin_id in HOST_SCALE_REPO_PLUGIN_IDS {
            let source = repo_plugin_ui_source(plugin_id);
            for (line_index, line) in source.lines().enumerate() {
                if !is_user_facing_text_line(line) {
                    continue;
                }
                for literal in quoted_literals(line) {
                    assert!(
                        allowed.contains(&literal.as_str()),
                        "unexpected visible text literal in {plugin_id}/ui/main.slint line {}: {:?}",
                        line_index + 1,
                        literal
                    );
                }
            }
        }
    }

    #[test]
    fn available_repo_plugins_use_direct_scaled_layout() {
        for plugin_id in DIRECT_SCALE_REPO_PLUGIN_IDS {
            let source = repo_plugin_ui_source(plugin_id);

            assert!(
                source.contains("function s(value: length) -> length")
                    && source.contains("return value * root.content-scale;"),
                "{plugin_id} should use a shared helper for direct scaled layout"
            );
            assert!(
                !source.contains("transform-scale-x: root.content-scale;")
                    && !source.contains("transform-scale-y: root.content-scale;"),
                "{plugin_id} should avoid transform scaling because tiny live windows clip it"
            );
        }
    }

    #[test]
    fn trust_card_chart_omits_horizontal_dotted_grid() {
        let source = repo_plugin_ui_source("com.cryptohud.trust-card");

        assert!(
            !source.contains("for dot[i] in 38 : Rectangle"),
            "trust card chart should not render horizontal dotted guide lines"
        );
        assert!(source.contains("commands: root.chart-line-path;"));
        assert!(source.contains("commands: root.chart-fill-path;"));
    }

    #[test]
    fn market_compass_loading_price_size_uses_state_not_localized_text() {
        let source = repo_plugin_ui_source("com.cryptohud.market-compass");

        assert!(
            !source.contains("quote.price == \"Connecting\"")
                && !source.contains("quote.price == \"连接中\""),
            "market compass loading layout should not compare localized runtime text"
        );
        assert!(
            source.contains(
                "font-size: active_panel.selected-chart-ready ? root.s(60px) : root.s(36px);"
            ),
            "market compass price size should follow chart readiness instead of localized labels"
        );
    }

    #[test]
    fn circular_repo_plugins_use_direct_scaled_layout() {
        for plugin_id in CIRCULAR_REPO_PLUGIN_IDS {
            let source = repo_plugin_ui_source(plugin_id);

            assert!(
                source.contains("width: root.widget-width * 1px;"),
                "{plugin_id} should bind root width to layout width"
            );
            assert!(
                source.contains("height: root.widget-height * 1px;"),
                "{plugin_id} should bind root height to layout height"
            );
            assert!(
                source.contains("property <float> content-scale: root.widget-scale;"),
                "{plugin_id} should use the host supplied content scale"
            );
            assert!(
                source.contains("function s(value: length) -> length"),
                "{plugin_id} should expose a direct scale helper"
            );
            assert!(
                source.contains("return value * root.content-scale;"),
                "{plugin_id} should scale dimensions from the host supplied scale"
            );
            assert!(
                source.contains("root.widget-width * 1px - root.s(480px)"),
                "{plugin_id} should center horizontally in the layout bounds"
            );
            assert!(
                source.contains("root.widget-height * 1px - root.s(480px)"),
                "{plugin_id} should center vertically in the layout bounds"
            );
            assert!(
                source.contains("width: root.s(480px);"),
                "{plugin_id} should scale the visual card width directly"
            );
            assert!(
                source.contains("height: root.s(480px);"),
                "{plugin_id} should scale the visual card height directly"
            );
            assert!(
                !source.contains("transform-scale-x: root.content-scale;")
                    && !source.contains("transform-scale-y: root.content-scale;"),
                "{plugin_id} should not rely on transform scaling for widget sizing"
            );
            assert!(
                source.contains("font-size: root.s("),
                "{plugin_id} should scale visible text with the card"
            );
            assert!(
                source.contains("opacity: root.content-opacity / 100;")
                    && source.contains("source: @image-url(\"opaque-circle.png\");"),
                "{plugin_id} should apply opacity to an opaque circular background while keeping the square outside transparent"
            );
        }
    }

    #[test]
    fn market_compass_orders_pairs_counterclockwise_for_clockwise_rotation() {
        let source = repo_plugin_ui_source("com.cryptohud.market-compass");

        assert!(
            source.contains("return root.bounded-rotation-step;"),
            "market compass active pair should advance with the clockwise rotation step"
        );
        assert!(
            source.contains("property <int> bounded-rotation-step: root.orbit-count <= 0 ? 0 : Math.mod(root.rotation-step, root.orbit-count);")
                && source.contains(
                    "slot-angle: -90deg + (root.rotation-step - index) * 1turn / root.orbit-denominator;"
                ),
            "market compass pairs should be laid out counterclockwise so the next pair enters 12 o'clock during clockwise rotation"
        );
        assert!(
            source.contains("root.rotation-step += 1;")
                && !source.contains("root.rotation-step = 0;"),
            "market compass rotation should remain monotonic so the last-to-first transition continues clockwise"
        );
        assert!(
            !source.contains("slot-angle: -90deg + (index + root.rotation-step)"),
            "market compass should not lay out pairs clockwise"
        );
    }

    #[test]
    fn market_compass_nodes_follow_the_rail_without_self_rotation() {
        let source = repo_plugin_ui_source("com.cryptohud.market-compass");

        assert!(
            source.contains("animate slot-angle { duration: 620ms; easing: ease-in-out; }")
                && !source.contains("animate x { duration: 620ms;")
                && !source.contains("animate y { duration: 620ms;"),
            "market compass should animate its polar angle instead of interpolating x/y across a chord"
        );
        assert!(
            source.contains(
                "x: root.orbit-center-x + root.orbit-radius * Math.cos(coin_node.slot-angle)"
            ) && source.contains(
                "y: root.orbit-center-y + root.orbit-radius * Math.sin(coin_node.slot-angle)"
            ),
            "market compass node centers should be derived from one shared circular rail"
        );
        assert!(
            !source.contains("property <angle> rolling-angle:")
                && !source.contains("rim_glint := Rectangle")
                && !source.contains("quote-icon-rotation-sheets")
                && !source.contains("source-clip-x: coin_node.icon-frame-index"),
            "market compass coin shells and logos should stay upright while their nodes orbit"
        );
    }

    #[test]
    fn market_compass_dark_theme_removes_the_misaligned_inner_ring() {
        let source = repo_plugin_ui_source("com.cryptohud.market-compass");

        assert!(
            source.contains("dark_inner_ring_mask := Rectangle")
                && source.contains("width: parent.width * 0.594;")
                && source.contains("border-width: parent.width * 0.032;")
                && source.contains("border-color: #05090d;")
                && source.contains("visible: !root.light-theme;"),
            "market compass should cover only the dark node asset's extra inner cyan ring"
        );
    }

    #[test]
    fn market_compass_price_uses_raw_quote_price() {
        let source = repo_plugin_ui_source("com.cryptohud.market-compass");

        assert!(
            source.contains("text: quote.price;"),
            "market compass should display the host-formatted price without adding currency symbols"
        );
        assert!(
            !source.contains("\"$\" + quote.price"),
            "market compass should not prefix prices with a dollar sign"
        );
        assert!(
            !source.contains("price_backdrop := Rectangle"),
            "market compass should not draw an obvious capsule behind the main price"
        );
        assert!(
            source.contains("price_shadow := Text") && source.contains("price_glow := Text"),
            "market compass should improve price contrast through layered text, not a separate backing shape"
        );
        assert!(
            source.contains(
                "property <color> price-text-color: root.light-theme ? #0b302e : #ffffff;"
            ),
            "market compass price should use dark ink on the porcelain light theme and white on the dark dial"
        );
        assert!(
            source.contains("color: root.price-shadow-color;")
                && source.contains("color: root.price-glow-color;")
                && source.contains("color: root.price-text-color;"),
            "market compass price layers should stay readable through theme-specific text colors"
        );
        assert!(
            source
                .matches("font-size: active_panel.selected-chart-ready ? root.s(60px) : root.s(36px);")
                .count()
                == 3,
            "market compass price layers should use readable size for live prices and compact size for loading text"
        );
    }

    #[test]
    fn market_compass_pair_heading_stays_readable_on_dark_dial() {
        let source = repo_plugin_ui_source("com.cryptohud.market-compass");

        assert!(
            source.contains("pair_shadow := Text"),
            "market compass pair heading should have a subtle contrast layer"
        );
        assert!(
            source.contains(
                "property <color> pair-heading-color: root.light-theme ? #245f5a : #c7d8eb;"
            ),
            "market compass pair heading should use dark teal on the porcelain light theme and a light dial color on dark theme"
        );
        assert!(
            !source.contains(
                "color: root.muted-text-color;\n                font-size: root.s(25px);"
            ),
            "market compass pair heading should not inherit low-contrast theme-muted color"
        );
    }

    #[test]
    fn market_compass_light_theme_uses_porcelain_assets() {
        let source = repo_plugin_ui_source("com.cryptohud.market-compass");

        for asset in [
            "opaque-circle-light.png",
            "reference-light.png",
            "coin-node-light.png",
            "coin-node-light-selected.png",
        ] {
            assert!(
                source.contains(&format!("source: @image-url(\"{asset}\");")),
                "market compass light theme should include dedicated porcelain asset {asset}"
            );
        }
        assert!(
            !source.contains("opacity: root.light-theme ? 0.64 : 1;"),
            "market compass light theme should not be a translucent dark reference image"
        );
    }

    #[test]
    fn runtime_applies_widget_layout_as_logical_window_size() {
        let source = include_str!("runtime_bridge.rs");

        assert!(
            source.contains("LogicalSize::new(layout.width as f32, layout.height as f32)"),
            "widget windows should use logical layout size so display scaling does not crop content"
        );
        assert!(
            !source.contains("PhysicalSize::new(layout.width as u32, layout.height as u32)"),
            "widget layout dimensions should not be applied as physical pixels"
        );
    }

    #[test]
    fn disabled_prototype_plugins_are_not_compiled_eagerly() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins");
        let catalog = PluginCatalog::discover(vec![root]);
        let plugin = catalog
            .find("com.cryptohud.market-board")
            .expect("plugin should be discovered");

        assert!(matches!(plugin.status, PluginStatus::Disabled(_)));
        let PluginRendererDefinition::Slint {
            definition: None, ..
        } = &plugin.renderer
        else {
            panic!("disabled prototype renderer should not be compiled");
        };
    }

    #[test]
    fn circular_repo_plugin_reference_images_have_transparent_edges() {
        for plugin_id in CIRCULAR_REPO_PLUGIN_IDS {
            let ui_dir = repo_plugin_path(plugin_id).join("ui");
            assert_round_asset_has_transparent_antialiased_edge(
                plugin_id,
                &ui_dir.join("reference.png"),
            );
            assert_round_asset_has_transparent_antialiased_edge(
                plugin_id,
                &ui_dir.join("reference-light.png"),
            );
            assert_round_asset_has_transparent_antialiased_edge(
                plugin_id,
                &ui_dir.join("opaque-circle.png"),
            );
            assert_round_asset_has_transparent_antialiased_edge(
                plugin_id,
                &ui_dir.join("opaque-circle-light.png"),
            );
        }
    }

    const CIRCULAR_REPO_PLUGIN_IDS: &[&str] = &["com.cryptohud.market-compass"];

    fn assert_round_asset_has_transparent_antialiased_edge(plugin_id: &str, image_path: &Path) {
        let asset_name = image_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("asset");
        let image = slint::Image::load_from_path(image_path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", image_path.display()));
        let pixels = image
            .to_rgba8()
            .unwrap_or_else(|| panic!("failed to read RGBA pixels from {}", image_path.display()));
        let width = pixels.width() as usize;
        let height = pixels.height() as usize;
        let data = pixels.as_slice();

        assert_eq!((width, height), (480, 480), "{plugin_id} {asset_name} size");
        for x in 0..width {
            assert_eq!(data[x].a, 0, "{plugin_id} {asset_name} top edge pixel {x}");
            assert_eq!(
                data[(height - 1) * width + x].a,
                0,
                "{plugin_id} {asset_name} bottom edge pixel {x}"
            );
        }
        for y in 0..height {
            assert_eq!(
                data[y * width].a,
                0,
                "{plugin_id} {asset_name} left edge pixel {y}"
            );
            assert_eq!(
                data[y * width + width - 1].a,
                0,
                "{plugin_id} {asset_name} right edge pixel {y}"
            );
        }

        let center_x = width / 2;
        let center_y = height / 2;
        let left_ramp = (0..center_x)
            .filter(|x| {
                let alpha = data[center_y * width + x].a;
                alpha > 0 && alpha < 255
            })
            .count();
        let right_ramp = (center_x..width)
            .filter(|x| {
                let alpha = data[center_y * width + x].a;
                alpha > 0 && alpha < 255
            })
            .count();
        let top_ramp = (0..center_y)
            .filter(|y| {
                let alpha = data[y * width + center_x].a;
                alpha > 0 && alpha < 255
            })
            .count();
        let bottom_ramp = (center_y..height)
            .filter(|y| {
                let alpha = data[y * width + center_x].a;
                alpha > 0 && alpha < 255
            })
            .count();

        for (side, ramp) in [
            ("left", left_ramp),
            ("right", right_ramp),
            ("top", top_ramp),
            ("bottom", bottom_ramp),
        ] {
            assert!(
                ramp >= 5,
                "{plugin_id} {asset_name} {side} circular edge should be antialiased, found {ramp} ramp pixels"
            );
        }
    }

    fn repo_plugin_ui_source(plugin_id: &str) -> String {
        let path = repo_plugin_path(plugin_id).join("ui").join("main.slint");
        fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
    }

    fn repo_plugin_path(plugin_id: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("plugins")
            .join(plugin_id)
    }

    fn is_user_facing_text_line(line: &str) -> bool {
        line.contains("text:")
            || line.contains("placeholder-text:")
            || line.contains("title:")
            || line.contains("tooltip:")
    }

    fn quoted_literals(line: &str) -> Vec<String> {
        let mut literals = Vec::new();
        let mut chars = line.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch != '"' {
                continue;
            }
            let mut literal = String::new();
            let mut escaped = false;
            for value in chars.by_ref() {
                if escaped {
                    literal.push(value);
                    escaped = false;
                } else if value == '\\' {
                    escaped = true;
                } else if value == '"' {
                    break;
                } else {
                    literal.push(value);
                }
            }
            literals.push(literal);
        }
        literals
    }

    fn temp_plugin_root(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("crypto-hud-{label}-{suffix}"));
        fs::create_dir_all(&root).unwrap();
        root
    }
}
