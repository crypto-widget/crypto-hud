use std::{
    collections::HashSet,
    fmt, fs, io,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use crypto_hud_core::{default_market_symbols, normalize_market_pair_key};
pub use crypto_hud_runtime::{
    parse_manifest, validate_manifest, PluginDataRequirement, PluginManifest, PluginParameter,
    PluginSize, PluginSizePolicy, PluginTheme, PluginThemeRole, MAX_PREVIEW_IMAGES,
    MIN_SYMBOL_LIMIT,
};
use i_slint_compiler::{
    diagnostics::BuildDiagnostics,
    parser::{self, SyntaxKind, SyntaxNode},
    EmbedResourcesKind,
};
use i_slint_core::InternalToken;
use semver::Version;
use slint_interpreter::{Compiler, ComponentDefinition, ValueType};

pub use crypto_hud_shell_state::{BUILTIN_MINI_TICKER_PLUGIN_ID, BUILTIN_QUOTE_BOARD_PLUGIN_ID};

pub const MANIFEST_FILE_NAME: &str = "widget.json";
pub const USER_PLUGIN_DEVELOPMENT_GUIDE_FILE_NAME: &str = "CUSTOM_UI_PLUGIN_DEVELOPMENT.md";
pub const MANIFEST_MAX_BYTES: u64 = 64 * 1024;
pub const SLINT_FILE_MAX_BYTES: u64 = 256 * 1024;
pub const ASSET_MAX_BYTES: u64 = 1024 * 1024;
pub const PLUGIN_DIR_MAX_BYTES: u64 = 5 * 1024 * 1024;
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
pub struct PluginCatalog {
    plugins: Vec<PluginDefinition>,
    errors: Vec<PluginCatalogError>,
}

impl PluginCatalog {
    pub fn builtins() -> Self {
        Self {
            plugins: builtin_plugins(),
            errors: Vec::new(),
        }
    }

    pub fn load(state_dir: &Path) -> Self {
        let bundled_roots = bundled_plugin_roots();
        let bundled_root_available = bundled_roots.iter().any(|root| root.is_dir());
        let expected_bundled_root = bundled_roots
            .iter()
            .find(|root| root.is_dir())
            .or_else(|| bundled_roots.first())
            .cloned();
        let mut catalog = Self::discover(plugin_roots(state_dir));
        if !bundled_root_available {
            if let Some(root) = expected_bundled_root.as_ref() {
                catalog.push_error(root.clone(), "bundled plugin directory is missing");
            }
        }
        for plugin_id in BUNDLED_BUILTIN_SLINT_PLUGIN_IDS {
            let bundled_plugin_loaded = catalog.plugins.iter().any(|plugin| {
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
                catalog.push_error(path, "required bundled plugin is missing or invalid");
            }
        }
        catalog
    }

    pub fn discover(plugin_roots: Vec<PathBuf>) -> Self {
        let mut catalog = Self::builtins();
        let mut seen_ids = catalog
            .plugins
            .iter()
            .map(|plugin| plugin.id.clone())
            .collect::<HashSet<_>>();

        for root in plugin_roots {
            discover_root(&root, &mut catalog, &mut seen_ids);
        }

        catalog
    }

    pub fn plugins(&self) -> &[PluginDefinition] {
        &self.plugins
    }

    pub fn market_plugins(&self) -> impl Iterator<Item = &PluginDefinition> {
        self.plugins
            .iter()
            .filter(|plugin| is_market_plugin_visible(&plugin.id))
    }

    pub fn errors(&self) -> &[PluginCatalogError] {
        &self.errors
    }

    pub fn find(&self, plugin_id: &str) -> Option<&PluginDefinition> {
        self.plugins.iter().find(|plugin| plugin.id == plugin_id)
    }

    #[cfg(test)]
    pub fn from_plugins_for_tests(plugins: Vec<PluginDefinition>) -> Self {
        Self {
            plugins,
            errors: Vec::new(),
        }
    }

    fn push_error(&mut self, path: PathBuf, error: impl fmt::Display) {
        self.errors.push(PluginCatalogError {
            path,
            message: error.to_string(),
        });
    }
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

fn discover_root(root: &Path, catalog: &mut PluginCatalog, seen_ids: &mut HashSet<String>) {
    if !root.exists() {
        return;
    }

    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            catalog.push_error(root.to_path_buf(), error);
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                catalog.push_error(root.to_path_buf(), error);
                continue;
            }
        };
        let path = entry.path();
        let Ok(metadata) = entry.metadata() else {
            catalog.push_error(path, "failed to read plugin candidate metadata");
            continue;
        };
        if !metadata.is_dir() {
            continue;
        }

        let manifest_path = path.join(MANIFEST_FILE_NAME);
        if !manifest_path.exists() {
            continue;
        }

        match load_local_plugin(&path) {
            Ok(plugin) => {
                if seen_ids.contains(&plugin.id) {
                    catalog.push_error(manifest_path, format!("duplicate plugin id {}", plugin.id));
                    continue;
                }
                if let PluginStatus::Unavailable(message) = &plugin.status {
                    catalog.push_error(manifest_path.clone(), message);
                }
                seen_ids.insert(plugin.id.clone());
                catalog.plugins.push(plugin);
            }
            Err(error) => catalog.push_error(manifest_path, error),
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
        let PluginParameter::Integer { key, .. } = parameter;
        let name = format!("config-{key}");
        let Some((_, actual_type)) = properties.iter().find(|(property, _)| property == &name)
        else {
            bail!("Slint component is missing parameter property {name}");
        };
        if actual_type != &ValueType::Number {
            bail!("Slint parameter property {name} must be an integer or float");
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
            .map(|plugin| plugin.id.as_str())
            .collect::<Vec<_>>();

        assert!(market_ids.contains(&BUILTIN_QUOTE_BOARD_PLUGIN_ID));
        assert!(!market_ids.contains(&BUILTIN_MINI_TICKER_PLUGIN_ID));
        assert!(!market_ids.contains(&"com.cryptohud.market-board"));
        assert!(market_ids.contains(&"com.cryptohud.market-compass"));
        assert!(!market_ids.contains(&"com.example.stage3-price-card"));
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
