# Localization Guide

Crypto HUD keeps localization in code so the native Slint shell, widget
runtime, notifications, and settings models all use the same locale contract.

## Supported Locales

Keep this list in the same order as `LanguagePreference::ALL`,
`Locale::ALL`, the language combo box, and persisted preference indices.

| Locale | Preference | Audience |
| --- | --- | --- |
| `en` | `En` | Default English UI |
| `zh-CN` | `ZhHans` | Simplified Chinese communities |
| `zh-TW` | `ZhHant` | Traditional Chinese for Taiwan, Hong Kong, Macau, and overseas Chinese users |
| `es-419` | `Es419` | Latin American Spanish |
| `pt-BR` | `PtBr` | Brazilian Portuguese |
| `vi` | `Vi` | Vietnamese |
| `id` | `Id` | Indonesian |
| `tr` | `Tr` | Turkish |
| `ko` | `Ko` | Korean |
| `ja` | `Ja` | Japanese |
| `ru` | `Ru` | Russian |
| `ar` | `Ar` | Arabic with RTL layout |

Do not reorder existing preferences. The numeric indices are persisted in user
settings and covered by shell-state tests.

## Where Text Lives

- Rust UI strings live in `crates/crypto-hud/src/i18n.rs`.
- Persisted language preferences live in
  `crates/crypto-hud-shell-state/src/lib.rs`.
- Runtime text labels are passed through `crates/crypto-hud/src/runtime_bridge.rs`.
- Settings window text is applied in `crates/crypto-hud/src/settings_window.rs`.
- Built-in plugin-market titles and descriptions live in
  `builtin_plugin_title` and `builtin_plugin_description`.
- Core/runtime alert evaluation should return structured data only; notification titles and bodies are formatted in the app layer with the active locale.
- Slint files should not contain translatable fallback text. Keep visible
  literals there limited to logos, ticker examples, internal window titles,
  decorative preview text, or single-character controls.

When adding a new `UiText` field, update every locale explicitly. The test
`non_english_ui_text_constants_explicitly_maintain_every_field` is intended to
fail when a locale silently falls back to English.

## English-Identical Terms

Non-English primary UI text should not accidentally stay in English. The test
`primary_static_ui_copy_is_localized_for_every_non_english_locale` compares key
navigation, action, status, and settings strings against English.
The broader test `non_english_ui_text_does_not_match_english_unless_intentional`
scans every `UiText` field, case-insensitively, so lowercased English leftovers
are caught too.

When a term is intentionally identical to English because it is a local
borrowing or same-spelled word, add it to
`ALLOWED_IDENTICAL_PRIMARY_UI_FIELDS` with a short reason. The companion test
`allowed_identical_primary_ui_fields_are_still_needed` keeps that allowlist from
turning into stale noise after a term is translated later.

Product names, literal proxy examples, and intentionally empty runtime fragments
belong in `ALLOWED_IDENTICAL_TECHNICAL_UI_FIELDS` with a reason. The test
`allowed_identical_technical_ui_fields_are_still_needed` keeps those technical
exceptions honest too.

## Locale Detection

`LanguagePreference::from_locale_tag` is the shared parsing source for system
locale detection and persisted language preference compatibility. It accepts
common BCP 47 and POSIX-style tags, normalizes underscores, strips suffixes
such as `.UTF-8` and `@calendar=gregorian`, and then maps to a supported
language preference. The app-layer `locale_from_tag` must reuse that parser and
only convert the resulting preference to a UI `Locale`.
Saved settings write canonical BCP 47-style tags such as `zh-CN`, `es-419`,
and `pt-BR`, while deserialization must continue to accept legacy snake_case
values such as `zh_hans`, `es_419`, and `pt_br`.

Chinese script subtags must win over region subtags:

- `zh-Hans-HK` maps to `zh-CN`.
- `zh-Hant-CN` maps to `zh-TW`.
- `zh-HK`, `zh-MO`, `zh-TW`, and `yue-*` map to `zh-TW`.
- Generic `zh` and `cmn-Hans-*` map to `zh-CN`.
- Latin American and US Spanish tags such as `es-419`, `es-MX`, `es-AR`,
  `es-CO`, and `es-US` map to `es-419`. Do not map `es-ES` to `es-419`;
  this product does not maintain European Spanish copy. Do not infer `es`
  without a market region as Latin American Spanish.
- Brazilian Portuguese tags such as `pt-BR` and `pt_BR.UTF-8` map to `pt-BR`.
  Do not map `pt-PT` to `pt-BR`; this product does not maintain generic
  Portuguese copy. Do not infer `pt` without a Brazil region as Brazilian
  Portuguese.

## RTL Rules

Arabic is the only RTL locale today. Whenever Arabic text contains LTR tokens,
wrap the token with `ltr_isolate_for_locale(locale, value)` before formatting.

High-risk LTR tokens include:

- Product names such as `Crypto HUD`.
- Keyboard shortcuts such as `Alt+C`.
- Symbols and pairs such as `BTC`, `BTC/USDT`, and `BTCUSDT`.
- Versions, filenames, plugin ids, provider names, URLs, paths, dimensions, and
  counts.
- Percentages and formatted prices in alert or market-status text.

Static Arabic `AR_TEXT` literals that contain high-risk ASCII fragments should
use `\u{2066}` and `\u{2069}` directly. The test
`arabic_ui_text_constant_keeps_ltr_literals_isolated` scans for common misses.

For Slint UI, pass `rtl-layout` from Rust and mirror alignment or positions in
the component rather than reversing strings.

## Plugin UI Rules

Repo-bundled Slint plugins must accept the host-provided `rtl-layout` property
and should align localized labels with it. Keep market symbols, prices, and
percent changes stable for scanning unless a plugin intentionally mirrors the
whole layout.

Plugin Slint files should render host-provided localized properties such as
`pairs-heading-text`, `source-text`, `source-name-text`, `updated-text`, and
`empty-text`. Do not add visible English fallback strings in plugin Slint, and
do not compare layout state against localized text such as `Connecting`; use
explicit readiness/state properties instead.

Local or third-party plugin marketplace text should come from the manifest, not
the built-in translation table. Only `PluginSource::Builtin` may use
`builtin_plugin_title` and `builtin_plugin_description`.
Manifest-provided display names, including local plugin names and custom theme
names, remain unchanged in every locale; show the author's string exactly as
written.
User-entered display names such as custom widget names follow the same rule.

Known plugin status reasons from the host, such as Slint compilation failures
or missing required properties/callbacks, should be formatted in the locale
layer. Keep dynamic technical details intact, and isolate them for RTL locales.
Host-defined plugin capabilities such as `market.price` and `market.candles`
should also be displayed through locale-aware short labels; keep unknown
capability ids as technical tokens.

## Terminology

Use product terms consistently. These terms are intentionally compact because
Crypto HUD is a dense desktop utility, not a marketing page.

| English concept | Usage note |
| --- | --- |
| Widget | Desktop floating component. Keep distinct from plugin. |
| Pair | Market pair or symbol shown by a provider. |
| Quote asset | The second asset in a pair, for example `USDT` in `BTC/USDT`. |
| Alert | Local price/change notification rule. |
| Source | Market data provider or feed state. |
| Fallback | Automatic alternate source when a provider fails. |
| Widget Library | The settings tab that lists built-in and local widgets. |
| Custom widget | Local Slint plugin/widget supplied by the user. |

For Traditional Chinese, prefer crypto-community terms such as `交易對`,
`小工具`, and `資料源`; do not copy Simplified Chinese terms blindly.
For Japanese and Korean, keep wording precise and restrained. For Brazilian
Portuguese, use Brazil-market wording instead of generic European Portuguese.

## Market Review Notes

Automated tests catch missing fields, English fallbacks, RTL token isolation,
and plugin contract regressions. They do not prove market-appropriate wording.
Before shipping copy changes, do a focused human review for these locales:

- `zh-TW`: review crypto-community terminology separately from Simplified
  Chinese, especially `交易對`, `資料源`, alerts, and widget/plugin wording.
- `es-419`: avoid Spain-specific wording; prefer neutral Latin American
  financial and trading terms.
- `pt-BR`: use Brazilian Portuguese conventions, not generic or European
  Portuguese phrasing.
- `vi`, `id`, and `tr`: check retail-trading wording, source/fallback language,
  and whether borrowed English crypto terms are clearer than literal
  translations.
- `ko` and `ja`: keep UI copy precise, restrained, and consistent with local
  exchange/product conventions; review alerts and risk-adjacent text carefully.
- `ru`: review sanctions/compliance-sensitive wording before release. UI copy
  should describe viewing public market data only and must not imply trading,
  custody, routing, or availability promises.
- `ar`: test RTL layout manually in addition to string checks. Inspect mixed
  Arabic/LTR rows, proxy/path/status messages, plugin-market cards, and desktop
  widget footers.

## Verification Checklist

For localization changes, run at least:

```powershell
cargo +1.96.0 fmt --all
cargo +1.96.0 check -p crypto-hud
cargo +1.96.0 test -p crypto-hud locale_tags_resolve_to_supported_locales
cargo +1.96.0 test -p crypto-hud-core -p crypto-hud-runtime -p crypto-hud-shell-state -p crypto-hud
```

For a focused i18n pass before the full suite, run:

```powershell
cargo +1.96.0 test -p crypto-hud non_english_ui_text_constants_explicitly_maintain_every_field
cargo +1.96.0 test -p crypto-hud non_english_ui_text_does_not_match_english_unless_intentional
cargo +1.96.0 test -p crypto-hud readmes_advertise_the_exact_supported_locale_tags
cargo +1.96.0 test -p crypto-hud-shell-state enum_serialization_uses_stable_config_tags
cargo +1.96.0 test -p crypto-hud-shell-state language_preference_deserializes_common_locale_tags
cargo +1.96.0 test -p crypto-hud-shell-state language_preference_error_lists_canonical_and_legacy_config_tags
cargo +1.96.0 test -p crypto-hud locale_tags_resolve_to_supported_locales
cargo +1.96.0 test -p crypto-hud locale_tag_resolution_reuses_language_preference_parser
cargo +1.96.0 test -p crypto-hud supported_locale_lists_stay_in_sync_with_language_preferences
cargo +1.96.0 test -p crypto-hud locale_all_is_available_outside_test_only_code
cargo +1.96.0 test -p crypto-hud arabic_ui_text_constant_keeps_ltr_literals_isolated
cargo +1.96.0 test -p crypto-hud dynamic_option_sets_are_localized_for_every_non_english_locale
cargo +1.96.0 test -p crypto-hud key_settings_help_copy_is_localized_for_every_non_english_locale
cargo +1.96.0 test -p crypto-hud settings_and_market_copy_follow_locale
cargo +1.96.0 test -p crypto-hud slint_user_facing_text_literals_are_limited_to_non_localized_tokens
cargo +1.96.0 test -p crypto-hud refresh_tray_text_sets_every_localized_tray_label
cargo +1.96.0 test -p crypto-hud settings_window_shared_text_controls_follow_rtl_layout
cargo +1.96.0 test -p crypto-hud plugin_market_text_rows_follow_rtl_layout
cargo +1.96.0 test -p crypto-hud my_widgets_list_text_rows_follow_rtl_layout
cargo +1.96.0 test -p crypto-hud selected_widget_detail_text_rows_follow_rtl_layout
cargo +1.96.0 test -p crypto-hud network_proxy_text_rows_follow_rtl_layout
cargo +1.96.0 test -p crypto-hud system_app_info_labels_follow_rtl_layout
cargo +1.96.0 test -p crypto-hud symbol_picker_text_rows_follow_rtl_layout
cargo +1.96.0 test -p crypto-hud settings_refresh_preserves_open_symbol_picker
cargo +1.96.0 test -p crypto-hud app_signature_version_keeps_prefix_inside_ltr_isolate
cargo +1.96.0 test -p crypto-hud primary_alert_input_accepts_rtl_isolated_symbol_display
cargo +1.96.0 test -p crypto-hud update_notification_body_is_localized_for_every_non_english_locale
cargo +1.96.0 test -p crypto-hud arabic_update_notification_isolates_ltr_release_values
cargo +1.96.0 test -p crypto-hud alert_notification_copy_is_localized_for_every_non_english_locale
cargo +1.96.0 test -p crypto-hud arabic_alert_notification_isolates_ltr_market_values
cargo +1.96.0 test -p crypto-hud alert_24h_terms_are_locale_appropriate
cargo +1.96.0 test -p crypto-hud runtime_text_bridge_uses_selected_locale_labels
cargo +1.96.0 test -p crypto-hud runtime_refresh_tracks_locale_sensitive_widget_labels
cargo +1.96.0 test -p crypto-hud initial_widget_apply_sets_locale_sensitive_widget_labels
cargo +1.96.0 test -p crypto-hud network_proxy_empty_address_detail_is_localized_for_every_non_english_locale
cargo +1.96.0 test -p crypto-hud status_failure_message_is_localized_for_every_non_english_locale
cargo +1.96.0 test -p crypto-hud icon_cache_cleared_status_is_localized_for_every_non_english_locale
cargo +1.96.0 test -p crypto-hud builtin_plugin_market_copy_is_localized_for_every_non_english_locale
cargo +1.96.0 test -p crypto-hud bundled_builtin_slint_plugins_have_localized_market_copy
cargo +1.96.0 test -p crypto-hud plugin_unavailable_id_is_localized_for_every_non_english_locale
cargo +1.96.0 test -p crypto-hud plugin_market_dynamic_descriptions_are_localized_for_every_non_english_locale
cargo +1.96.0 test -p crypto-hud dynamic_short_labels_are_localized_for_every_non_english_locale
cargo +1.96.0 test -p crypto-hud known_plugin_status_reasons_are_localized_for_every_non_english_locale
cargo +1.96.0 test -p crypto-hud symbol_picker_control_copy_is_localized_for_every_non_english_locale
cargo +1.96.0 test -p crypto-hud dynamic_plugin_and_symbol_copy_follow_locale
cargo +1.96.0 test -p crypto-hud plugin_market_item_preserves_local_plugin_metadata_in_every_locale
cargo +1.96.0 test -p crypto-hud custom_widget_display_names_preserve_user_text_in_every_locale
cargo +1.96.0 test -p crypto-hud custom_widget_name_input_preserves_user_text_in_every_locale
cargo +1.96.0 test -p crypto-hud local_plugin_market_titles_preserve_manifest_names_in_every_locale
cargo +1.96.0 test -p crypto-hud custom_plugin_theme_labels_preserve_manifest_names_in_every_locale
cargo +1.96.0 test -p crypto-hud plugin_development_guides_document_manifest_name_boundary
cargo +1.96.0 test -p crypto-hud repo_plugins_accept_host_supplied_rtl_layout
cargo +1.96.0 test -p crypto-hud repo_plugin_visible_text_literals_are_limited_to_non_localized_tokens
cargo +1.96.0 test -p crypto-hud localization_guide_lists_every_supported_locale
```

Also manually inspect Arabic and long-string locales when layout changes touch:

- Settings tabs and form controls.
- Symbol picker and selected chips.
- Widget runtime rows and source/status footer.
- Notification titles and bodies.
- Plugin-market cards and local plugin metadata.
