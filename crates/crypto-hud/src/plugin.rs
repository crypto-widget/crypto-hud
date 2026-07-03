use std::{
    collections::HashSet,
    fmt, fs, io,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
pub use crypto_hud_runtime::{
    parse_manifest, validate_manifest, PluginDataRequirement, PluginManifest, PluginSize,
    MAX_SYMBOL_LIMIT, MIN_SYMBOL_LIMIT,
};
use semver::Version;
use slint_interpreter::{Compiler, ComponentDefinition, ValueType};

pub use crypto_hud_shell_state::{BUILTIN_MINI_TICKER_PLUGIN_ID, BUILTIN_QUOTE_BOARD_PLUGIN_ID};

pub const MANIFEST_FILE_NAME: &str = "widget.json";
pub const MANIFEST_MAX_BYTES: u64 = 64 * 1024;
pub const SLINT_FILE_MAX_BYTES: u64 = 256 * 1024;
pub const ASSET_MAX_BYTES: u64 = 1024 * 1024;
pub const PLUGIN_DIR_MAX_BYTES: u64 = 5 * 1024 * 1024;
const HIDDEN_MARKET_PLUGIN_IDS: &[&str] = &[
    BUILTIN_MINI_TICKER_PLUGIN_ID,
    "com.cryptohud.market-board",
    "com.cryptohud.market-compass",
    "com.cryptohud.orbit-pulse",
    "com.example.stage3-price-card",
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
        Self::discover(plugin_roots(state_dir))
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

#[derive(Debug, Clone)]
pub struct PluginDefinition {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub source: PluginSource,
    pub renderer: PluginRendererDefinition,
    pub default_size: PluginSize,
    pub min_symbol_limit: usize,
    pub symbol_limit: usize,
    pub data_requirements: Vec<PluginDataRequirement>,
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
                height: 232,
            },
            min_symbol_limit: MIN_SYMBOL_LIMIT,
            symbol_limit: MAX_SYMBOL_LIMIT,
            data_requirements: vec![PluginDataRequirement {
                capability: "market.price".to_string(),
            }],
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
            min_symbol_limit: MIN_SYMBOL_LIMIT,
            symbol_limit: MIN_SYMBOL_LIMIT,
            data_requirements: vec![PluginDataRequirement {
                capability: "market.price".to_string(),
            }],
            status: PluginStatus::Available,
        },
    ]
}

pub fn plugin_roots(state_dir: &Path) -> Vec<PathBuf> {
    vec![
        state_dir.join("plugins"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins"),
    ]
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

    if let PluginRendererDefinition::Slint {
        root_dir,
        entry,
        component,
        definition,
    } = &mut plugin.renderer
    {
        match compile_slint_renderer(root_dir, entry, component) {
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
    Ok(PluginDefinition {
        id: manifest.id,
        name: manifest.name,
        version,
        source: PluginSource::LocalUnsigned,
        renderer: PluginRendererDefinition::Slint {
            root_dir,
            entry,
            component: manifest.renderer.component,
            definition: None,
        },
        default_size: manifest.default_size,
        min_symbol_limit: manifest.min_symbol_limit,
        symbol_limit: manifest.symbol_limit,
        data_requirements: manifest.data_requirements,
        status: PluginStatus::Unavailable("Slint renderer has not been compiled".to_string()),
    })
}

fn compile_slint_renderer(
    root_dir: &Path,
    entry: &Path,
    component: &str,
) -> Result<ComponentDefinition> {
    let mut compiler = Compiler::default();
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

    let result = spin_on::spin_on(compiler.build_from_path(entry));
    if result.has_errors() {
        bail!("Slint compilation failed: {}", diagnostics_text(&result));
    }

    let definition = result.component(component).ok_or_else(|| {
        anyhow!(
            "renderer.component {} was not exported; available components: {}",
            component,
            result.component_names().collect::<Vec<_>>().join(", ")
        )
    })?;
    validate_slint_contract(&definition)?;
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
    fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))
}

fn validate_slint_contract(definition: &ComponentDefinition) -> Result<()> {
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
    validate_plugin_directory_limits_inner(&root, &root, &mut total_size)?;
    if total_size > PLUGIN_DIR_MAX_BYTES {
        bail!("plugin directory exceeds {PLUGIN_DIR_MAX_BYTES} bytes");
    }
    Ok(())
}

fn validate_plugin_directory_limits_inner(
    root: &Path,
    current: &Path,
    total_size: &mut u64,
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
            validate_plugin_directory_limits_inner(root, &canonical, total_size)?;
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
    in property <string> updated-text;
    in property <string> empty-text;
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

    #[test]
    fn parses_valid_manifest() {
        let manifest = parse_manifest(&valid_manifest_json()).unwrap();

        assert_eq!(manifest.schema_version, 3);
        assert_eq!(manifest.id, "com.example.price-card");
        assert_eq!(manifest.renderer.entry, "ui/main.slint");
        assert_eq!(manifest.min_symbol_limit, 1);
        assert_eq!(manifest.symbol_limit, 5);
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
        assert!(!market_ids.contains(&"com.cryptohud.market-compass"));
        assert!(!market_ids.contains(&"com.cryptohud.orbit-pulse"));
        assert!(!market_ids.contains(&"com.example.stage3-price-card"));
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
            "com.example.stage3-price-card",
            "com.cryptohud.focus-ticker",
            "com.cryptohud.market-board",
            "com.cryptohud.trust-card",
            "com.cryptohud.orbit-pulse",
            "com.cryptohud.market-compass",
            "com.cryptohud.status-strip",
        ] {
            let plugin = catalog
                .find(plugin_id)
                .expect("plugin should be discovered");
            assert!(plugin.is_available(), "{plugin_id} should be available");
        }
        assert!(catalog.errors().is_empty(), "{:?}", catalog.errors());
    }

    #[test]
    fn circular_repo_plugins_scale_from_runtime_window_size() {
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
                source.contains("((root.widget-width * 1px) / 480px)"),
                "{plugin_id} should scale from layout width"
            );
            assert!(
                source.contains("((root.widget-height * 1px) / 480px)"),
                "{plugin_id} should scale from layout height"
            );
            assert!(
                source.contains("root.widget-width * 1px - 480px * root.content-scale"),
                "{plugin_id} should center horizontally in the layout bounds"
            );
            assert!(
                source.contains("root.widget-height * 1px - 480px * root.content-scale"),
                "{plugin_id} should center vertically in the layout bounds"
            );
            assert!(
                source.contains("width: 480px * root.content-scale;"),
                "{plugin_id} should scale card width directly"
            );
            assert!(
                source.contains("height: 480px * root.content-scale;"),
                "{plugin_id} should scale card height directly"
            );
            assert!(
                !source.contains("transform-scale"),
                "{plugin_id} should not rely on transform scaling for window resizing"
            );
        }
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
    fn circular_repo_plugin_size_properties_are_settable() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins");
        let catalog = PluginCatalog::discover(vec![root]);
        let plugin = catalog
            .find("com.cryptohud.market-compass")
            .expect("plugin should be discovered");
        let PluginRendererDefinition::Slint {
            definition: Some(definition),
            ..
        } = &plugin.renderer
        else {
            panic!("market compass should use an available Slint renderer");
        };
        let instance = definition
            .create()
            .expect("dynamic Slint widget should instantiate");

        instance
            .set_property("widget-width", slint_interpreter::Value::Number(600.0))
            .expect("widget-width should be settable");
        instance
            .set_property("widget-height", slint_interpreter::Value::Number(600.0))
            .expect("widget-height should be settable");

        assert_eq!(
            instance.get_property("widget-width"),
            Ok(slint_interpreter::Value::Number(600.0))
        );
        assert_eq!(
            instance.get_property("widget-height"),
            Ok(slint_interpreter::Value::Number(600.0))
        );
    }

    #[test]
    fn circular_repo_plugin_reference_images_have_transparent_edges() {
        for plugin_id in CIRCULAR_REPO_PLUGIN_IDS {
            let image_path = repo_plugin_path(plugin_id).join("ui").join("reference.png");
            let image = slint::Image::load_from_path(&image_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", image_path.display()));
            let pixels = image.to_rgba8().unwrap_or_else(|| {
                panic!("failed to read RGBA pixels from {}", image_path.display())
            });
            let width = pixels.width() as usize;
            let height = pixels.height() as usize;
            let data = pixels.as_slice();

            assert_eq!((width, height), (480, 480), "{plugin_id} reference size");
            for x in 0..width {
                assert_eq!(data[x].a, 0, "{plugin_id} top edge pixel {x}");
                assert_eq!(
                    data[(height - 1) * width + x].a,
                    0,
                    "{plugin_id} bottom edge pixel {x}"
                );
            }
            for y in 0..height {
                assert_eq!(data[y * width].a, 0, "{plugin_id} left edge pixel {y}");
                assert_eq!(
                    data[y * width + width - 1].a,
                    0,
                    "{plugin_id} right edge pixel {y}"
                );
            }
        }
    }

    const CIRCULAR_REPO_PLUGIN_IDS: &[&str] =
        &["com.cryptohud.orbit-pulse", "com.cryptohud.market-compass"];

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
