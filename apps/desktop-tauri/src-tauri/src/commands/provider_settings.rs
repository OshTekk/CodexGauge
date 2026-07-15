use super::*;

fn ensure_visible_desktop_provider(
    provider_id: &str,
) -> Result<codexbar::core::ProviderId, String> {
    let provider = codexbar::core::ProviderId::from_cli_name(provider_id)
        .ok_or_else(|| format!("Provider '{provider_id}' is not available in desktop product"))?;
    if crate::product_policy::is_visible_provider(provider) {
        Ok(provider)
    } else {
        Err(format!(
            "Provider '{provider_id}' is not available in desktop product"
        ))
    }
}

fn ensure_visible_desktop_providers(provider_ids: &[String]) -> Result<(), String> {
    for provider_id in provider_ids {
        ensure_visible_desktop_provider(provider_id)?;
    }
    Ok(())
}

// ── Provider summaries + ordering ─────────────────────────────────────

/// Lightweight provider entry returned to the UI after a reorder.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSummary {
    pub id: String,
    pub display_name: String,
    pub enabled: bool,
    pub order: u32,
}

/// Build `ProviderSummary` list honouring the persisted `provider_order`.
pub(crate) fn build_provider_summaries(settings: &Settings) -> Vec<ProviderSummary> {
    settings
        .provider_display_order()
        .into_iter()
        .filter(|provider| crate::product_policy::is_visible_provider(*provider))
        .enumerate()
        .map(|(idx, provider)| ProviderSummary {
            id: provider.cli_name().to_string(),
            display_name: provider.display_name().to_string(),
            enabled: settings.is_provider_enabled(provider),
            order: idx as u32,
        })
        .collect()
}

pub(crate) fn merge_visible_provider_order(
    settings: &Settings,
    requested_ids: &[String],
) -> Vec<String> {
    let requested = requested_ids
        .iter()
        .filter_map(|provider_id| ProviderId::from_cli_name(provider_id))
        .filter(|provider| crate::product_policy::is_visible_provider(*provider))
        .collect::<Vec<_>>();
    let mut requested = requested.into_iter();

    settings
        .provider_display_order()
        .into_iter()
        .map(|provider| {
            if crate::product_policy::is_visible_provider(provider) {
                requested.next().unwrap_or(provider)
            } else {
                provider
            }
        })
        .map(|provider| provider.cli_name().to_string())
        .collect()
}

#[tauri::command]
pub fn reorder_providers(
    app: tauri::AppHandle,
    ids: Vec<String>,
) -> Result<Vec<ProviderSummary>, String> {
    ensure_visible_desktop_providers(&ids)?;
    let mut settings = Settings::load();
    settings.provider_order = merge_visible_provider_order(&settings, &ids);
    settings.save().map_err(|e| e.to_string())?;
    crate::tray_bridge::refresh_tray_presentation(&app);
    // Notify open surfaces (tray flyout, pop-out window) so their provider grid
    // and cards re-render in the new order immediately after a drag-reorder.
    crate::events::emit_settings_changed(&app);
    Ok(build_provider_summaries(&settings))
}

// ── Per-provider cookie source + region ───────────────────────────────

/// Map a CLI-name string to a `ProviderId` whose cookie source is exposed in
/// the UI. Returns `None` for providers without a user-facing cookie source.
fn cookie_source_provider(provider_id: &str) -> Option<codexbar::core::ProviderId> {
    use codexbar::core::ProviderId;
    Some(match provider_id {
        "codex" => ProviderId::Codex,
        "claude" => ProviderId::Claude,
        "cursor" => ProviderId::Cursor,
        "opencode" => ProviderId::OpenCode,
        "factory" => ProviderId::Factory,
        "alibaba" => ProviderId::Alibaba,
        "kimi" | "kimik2" => ProviderId::Kimi,
        "minimax" => ProviderId::MiniMax,
        "augment" => ProviderId::Augment,
        "amp" => ProviderId::Amp,
        "ollama" => ProviderId::Ollama,
        "mistral" => ProviderId::Mistral,
        "qoder" => ProviderId::Qoder,
        "sakana" => ProviderId::Sakana,
        _ => return None,
    })
}

pub(crate) fn provider_cookie_source_lookup(
    settings: &Settings,
    provider_id: &str,
) -> Option<String> {
    cookie_source_provider(provider_id).map(|id| settings.cookie_source(id).to_string())
}

pub(crate) fn provider_cookie_source_set(
    settings: &mut Settings,
    provider_id: &str,
    source: String,
) -> Result<(), String> {
    let id = cookie_source_provider(provider_id)
        .ok_or_else(|| format!("Provider '{provider_id}' does not expose a cookie source"))?;
    settings.set_cookie_source(id, source);
    Ok(())
}

#[tauri::command]
pub fn set_provider_cookie_source(provider_id: String, source: String) -> Result<(), String> {
    ensure_visible_desktop_provider(&provider_id)?;
    let source = source.trim();
    if source.is_empty()
        || !cookie_source_options_for(&provider_id, Language::English)
            .iter()
            .any(|option| option.value == source)
    {
        return Err(format!(
            "Invalid cookie source '{source}' for provider '{provider_id}'"
        ));
    }
    let mut settings = Settings::load();
    provider_cookie_source_set(&mut settings, &provider_id, source.to_string())?;
    settings.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_provider_cookie_source(provider_id: String) -> Result<Option<String>, String> {
    ensure_visible_desktop_provider(&provider_id)?;
    Ok(provider_cookie_source_lookup(
        &Settings::load(),
        &provider_id,
    ))
}

fn region_provider(provider_id: &str) -> Option<codexbar::core::ProviderId> {
    use codexbar::core::ProviderId;
    Some(match provider_id {
        "alibaba" => ProviderId::Alibaba,
        "zai" => ProviderId::Zai,
        "minimax" => ProviderId::MiniMax,
        _ => return None,
    })
}

pub(crate) fn provider_region_lookup(settings: &Settings, provider_id: &str) -> Option<String> {
    region_provider(provider_id).map(|id| {
        if id == codexbar::core::ProviderId::MiniMax {
            codexbar::providers::MiniMaxProvider::region_from_settings(Some(
                settings.api_region(id),
            ))
            .settings_value()
            .to_string()
        } else {
            settings.api_region(id).to_string()
        }
    })
}

pub(crate) fn provider_region_set(
    settings: &mut Settings,
    provider_id: &str,
    region: String,
) -> Result<(), String> {
    let id = region_provider(provider_id)
        .ok_or_else(|| format!("Provider '{provider_id}' does not have a region picker"))?;
    settings.set_api_region(id, region);
    Ok(())
}

#[tauri::command]
pub fn set_provider_region(provider_id: String, region: String) -> Result<(), String> {
    ensure_visible_desktop_provider(&provider_id)?;
    let region = region.trim();
    if region.is_empty()
        || !region_options_for(&provider_id)
            .iter()
            .any(|option| option.value == region)
    {
        return Err(format!(
            "Invalid region '{region}' for provider '{provider_id}'"
        ));
    }
    let mut settings = Settings::load();
    provider_region_set(&mut settings, &provider_id, region.to_string())?;
    settings.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_provider_region(provider_id: String) -> Result<Option<String>, String> {
    ensure_visible_desktop_provider(&provider_id)?;
    Ok(provider_region_lookup(&Settings::load(), &provider_id))
}

fn workspace_provider(provider_id: &str) -> Option<codexbar::core::ProviderId> {
    use codexbar::core::ProviderId;
    Some(match provider_id {
        "openaiapi" => ProviderId::OpenAIApi,
        "litellm" => ProviderId::LiteLLM,
        "devin" => ProviderId::Devin,
        "opencodego" => ProviderId::OpenCodeGo,
        "zed" => ProviderId::Zed,
        _ => return None,
    })
}

#[tauri::command]
pub fn set_provider_workspace_id(provider_id: String, workspace_id: String) -> Result<(), String> {
    ensure_visible_desktop_provider(&provider_id)?;
    let id = workspace_provider(&provider_id).ok_or_else(|| {
        format!("Provider '{provider_id}' does not expose a workspace/project id")
    })?;
    let workspace_id = codexbar::settings::validate_provider_workspace_value(id, &workspace_id)?;
    let mut settings = Settings::load();
    prevent_litellm_key_retargeting(id, settings.workspace_id(id), &workspace_id)?;
    settings.set_workspace_id(id, workspace_id);
    settings.save().map_err(|e| e.to_string())
}

fn prevent_litellm_key_retargeting(
    id: codexbar::core::ProviderId,
    current_workspace_id: &str,
    next_workspace_id: &str,
) -> Result<(), String> {
    litellm_workspace_change_allowed(
        id,
        current_workspace_id,
        next_workspace_id,
        ApiKeys::load().has_key(id.cli_name()),
    )
}

fn litellm_workspace_change_allowed(
    id: codexbar::core::ProviderId,
    current_workspace_id: &str,
    next_workspace_id: &str,
    has_saved_api_key: bool,
) -> Result<(), String> {
    if id != codexbar::core::ProviderId::LiteLLM || next_workspace_id.trim().is_empty() {
        return Ok(());
    }

    let current = current_workspace_id.trim().trim_end_matches('/');
    let next = next_workspace_id.trim().trim_end_matches('/');
    if current.eq_ignore_ascii_case(next) || !has_saved_api_key {
        return Ok(());
    }

    Err(
        "Remove the saved LiteLLM API key before changing the LiteLLM base URL, then save the key again for the new endpoint."
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_provider_guard_only_accepts_codex() {
        for provider in ProviderId::all() {
            let result = ensure_visible_desktop_provider(provider.cli_name());
            assert_eq!(
                result.is_ok(),
                *provider == ProviderId::Codex,
                "unexpected command access for {}",
                provider.cli_name()
            );
        }

        assert!(ensure_visible_desktop_provider("unknown").is_err());
        assert!(
            ensure_visible_desktop_providers(&[
                ProviderId::Codex.cli_name().to_string(),
                ProviderId::Claude.cli_name().to_string(),
            ])
            .is_err()
        );
    }

    #[test]
    fn codex_cookie_source_catalog_remains_available() {
        assert_eq!(
            ensure_visible_desktop_provider(ProviderId::Codex.cli_name()),
            Ok(ProviderId::Codex)
        );
        assert!(
            !cookie_source_options_for(ProviderId::Codex.cli_name(), Language::English).is_empty()
        );
    }

    #[test]
    fn hidden_provider_commands_fail_closed() {
        let unavailable = |error: String| {
            assert!(
                error.contains("not available in desktop product"),
                "unexpected error: {error}"
            );
        };

        unavailable(set_provider_cookie_source("claude".into(), "auto".into()).unwrap_err());
        unavailable(get_provider_cookie_source("claude".into()).unwrap_err());
        unavailable(set_provider_region("alibaba".into(), "global".into()).unwrap_err());
        unavailable(get_provider_region("alibaba".into()).unwrap_err());
        unavailable(
            set_provider_workspace_id("litellm".into(), "https://example.com".into()).unwrap_err(),
        );
        unavailable(get_provider_workspace_id("litellm".into()).unwrap_err());
        unavailable(get_provider_gateway_url("wayfinder".into()).unwrap_err());
        unavailable(
            set_provider_gateway_url("wayfinder".into(), "https://example.com".into()).unwrap_err(),
        );
        unavailable(get_provider_cookie_source_options("claude".into()).unwrap_err());
        unavailable(get_provider_region_options("alibaba".into()).unwrap_err());
    }

    #[test]
    fn maps_opencode_go_workspace_provider() {
        assert_eq!(
            workspace_provider("opencodego"),
            Some(ProviderId::OpenCodeGo)
        );
    }

    #[test]
    fn litellm_endpoint_change_requires_reentering_saved_key() {
        assert!(
            litellm_workspace_change_allowed(
                ProviderId::LiteLLM,
                "https://old.example.com",
                "https://new.example.com",
                true,
            )
            .is_err()
        );
        assert!(
            litellm_workspace_change_allowed(
                ProviderId::LiteLLM,
                "https://old.example.com",
                "https://old.example.com/",
                true,
            )
            .is_ok()
        );
        assert!(
            litellm_workspace_change_allowed(
                ProviderId::LiteLLM,
                "https://old.example.com",
                "",
                true,
            )
            .is_ok()
        );
        assert!(
            litellm_workspace_change_allowed(
                ProviderId::LiteLLM,
                "https://old.example.com",
                "https://new.example.com",
                false,
            )
            .is_ok()
        );
    }
}

#[tauri::command]
pub fn get_provider_workspace_id(provider_id: String) -> Result<Option<String>, String> {
    ensure_visible_desktop_provider(&provider_id)?;
    let Some(id) = workspace_provider(&provider_id) else {
        return Ok(None);
    };
    let value = Settings::load().workspace_id(id).trim().to_string();
    Ok((!value.is_empty()).then_some(value))
}

fn gateway_provider(provider_id: &str) -> Option<codexbar::core::ProviderId> {
    (provider_id == "wayfinder").then_some(codexbar::core::ProviderId::Wayfinder)
}

#[tauri::command]
pub fn get_provider_gateway_url(provider_id: String) -> Result<Option<String>, String> {
    ensure_visible_desktop_provider(&provider_id)?;
    let Some(id) = gateway_provider(&provider_id) else {
        return Ok(None);
    };
    Ok(Some(Settings::load().gateway_url(id).to_string()))
}

#[tauri::command]
pub fn set_provider_gateway_url(provider_id: String, gateway_url: String) -> Result<(), String> {
    ensure_visible_desktop_provider(&provider_id)?;
    let id = gateway_provider(&provider_id)
        .ok_or_else(|| format!("Provider '{provider_id}' does not expose a gateway URL"))?;
    let gateway_url = gateway_url.trim();
    codexbar::providers::wayfinder::parse_gateway_url(gateway_url)
        .map_err(|error| error.to_string())?;

    let mut settings = Settings::load();
    settings.set_gateway_url(id, gateway_url.to_string());
    settings.save().map_err(|error| error.to_string())
}

// ── Phase 6c — cookie source & region option catalogs ────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CookieSourceOption {
    pub value: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegionOption {
    pub value: String,
    pub label: String,
}

fn cookie_option(
    lang: Language,
    value: &str,
    auto_desc: impl Into<String>,
    manual_desc: impl Into<String>,
    off_desc: Option<&str>,
) -> CookieSourceOption {
    let (label, description) = match value {
        "auto" => (
            locale::get_text(lang, locale::LocaleKey::Automatic),
            auto_desc.into(),
        ),
        "manual" => (
            locale::get_text(lang, locale::LocaleKey::CookieSourceManual),
            manual_desc.into(),
        ),
        "off" => (
            locale::get_text(lang, locale::LocaleKey::ProviderDisabled),
            off_desc.unwrap_or("").to_string(),
        ),
        other => (other.to_string(), String::new()),
    };
    CookieSourceOption {
        value: value.to_string(),
        label,
        description: if description.is_empty() {
            None
        } else {
            Some(description)
        },
    }
}

/// Returns the catalog of cookie source options for a given provider,
/// mirroring the `egui` ComboBox choices in `preferences.rs`.
/// Empty vec means the provider does not expose a cookie-source picker.
pub fn cookie_source_options_for(provider_id: &str, lang: Language) -> Vec<CookieSourceOption> {
    match provider_id {
        "codex" => vec![
            cookie_option(
                lang,
                "auto",
                locale::get_text(lang, locale::LocaleKey::ProviderCodexAutoImportHelp),
                "Paste a Cookie header from a chatgpt.com request.",
                Some("Disable OpenAI dashboard cookie usage."),
            ),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from a chatgpt.com request.",
                None,
            ),
            cookie_option(
                lang,
                "off",
                "",
                "",
                Some("Disable OpenAI dashboard cookie usage."),
            ),
        ],
        "claude" => vec![
            cookie_option(
                lang,
                "auto",
                locale::get_text(lang, locale::LocaleKey::ProviderClaudeCookiesHelp),
                "",
                None,
            ),
            cookie_option(
                lang,
                "manual",
                "",
                locale::get_text(lang, locale::LocaleKey::ProviderClaudeCookiesHelp),
                None,
            ),
        ],
        "cursor" => vec![
            cookie_option(
                lang,
                "auto",
                locale::get_text(lang, locale::LocaleKey::ProviderCursorCookieSourceHelp),
                "",
                None,
            ),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from a cursor.com request.",
                None,
            ),
        ],
        "opencode" => vec![
            cookie_option(
                lang,
                "auto",
                "Automatic imports browser cookies from opencode.ai.",
                "",
                None,
            ),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from the billing page.",
                None,
            ),
        ],
        "factory" => vec![
            cookie_option(
                lang,
                "auto",
                "Automatic imports browser cookies and WorkOS sessions.",
                "",
                None,
            ),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from Factory.",
                None,
            ),
        ],
        "alibaba" => vec![
            cookie_option(
                lang,
                "auto",
                "Automatic imports browser cookies from Model Studio / Bailian.",
                "",
                None,
            ),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from Model Studio or Bailian.",
                None,
            ),
        ],
        "kimi" | "kimik2" => vec![
            cookie_option(lang, "auto", "Automatic imports browser cookies.", "", None),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a cookie header or the kimi-auth token value.",
                None,
            ),
            cookie_option(lang, "off", "", "", Some("Kimi cookies are disabled.")),
        ],
        "minimax" => vec![
            cookie_option(
                lang,
                "auto",
                "Automatic imports browser cookies and Coding Plan tokens.",
                "",
                None,
            ),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from the Coding Plan page.",
                None,
            ),
        ],
        "augment" => vec![
            cookie_option(lang, "auto", "Automatic imports browser cookies.", "", None),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from the Augment dashboard.",
                None,
            ),
        ],
        "amp" => vec![
            cookie_option(lang, "auto", "Automatic imports browser cookies.", "", None),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from Amp settings.",
                None,
            ),
        ],
        "ollama" => vec![
            cookie_option(lang, "auto", "Automatic imports browser cookies.", "", None),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from Ollama settings.",
                None,
            ),
        ],
        "mistral" => vec![
            cookie_option(
                lang,
                "auto",
                "Automatic imports browser cookies from Mistral Admin.",
                "",
                None,
            ),
            cookie_option(
                lang,
                "manual",
                "",
                "Paste a Cookie header from admin.mistral.ai.",
                None,
            ),
        ],
        _ => Vec::new(),
    }
}

/// Returns the API region options for a given provider.
/// Empty vec means the provider has no region picker.
pub fn region_options_for(provider_id: &str) -> Vec<RegionOption> {
    match provider_id {
        "alibaba" => codexbar::providers::AlibabaRegion::ALL
            .iter()
            .map(|region| RegionOption {
                value: region.settings_value().to_string(),
                label: region.display_name().to_string(),
            })
            .collect(),
        "zai" => vec![
            RegionOption {
                value: "global".to_string(),
                label: "Global".to_string(),
            },
            RegionOption {
                value: "china".to_string(),
                label: "China Mainland (BigModel)".to_string(),
            },
        ],
        "minimax" => vec![
            RegionOption {
                value: "global".to_string(),
                label: codexbar::providers::MiniMaxRegion::Global
                    .display_name()
                    .to_string(),
            },
            RegionOption {
                value: "cn".to_string(),
                label: codexbar::providers::MiniMaxRegion::ChinaMainland
                    .display_name()
                    .to_string(),
            },
        ],
        _ => Vec::new(),
    }
}

#[tauri::command]
pub fn get_provider_cookie_source_options(
    provider_id: String,
) -> Result<Vec<CookieSourceOption>, String> {
    ensure_visible_desktop_provider(&provider_id)?;
    let lang = Settings::load().ui_language;
    Ok(cookie_source_options_for(&provider_id, lang))
}

#[tauri::command]
pub fn get_provider_region_options(provider_id: String) -> Result<Vec<RegionOption>, String> {
    ensure_visible_desktop_provider(&provider_id)?;
    Ok(region_options_for(&provider_id))
}
