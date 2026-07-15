use std::collections::HashMap;

use codexbar::settings::{ApiKeys, ManualCookies, Settings};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeDiagnostics {
    pub app_version: String,
    pub platform: String,
    pub enabled_providers: Vec<String>,
    pub provider_cookie_sources: HashMap<String, String>,
    pub has_manual_cookies: Vec<String>,
    pub has_api_keys: Vec<String>,
    pub hide_personal_info: bool,
    pub refresh_interval_secs: u64,
}

fn safe_diagnostics_from(
    settings: Settings,
    cookies: ManualCookies,
    api_keys: ApiKeys,
) -> SafeDiagnostics {
    let mut enabled_providers = settings
        .get_enabled_provider_ids()
        .into_iter()
        .filter(|id| crate::product_policy::is_visible_provider(*id))
        .map(|id| id.cli_name().to_string())
        .collect::<Vec<_>>();
    enabled_providers.sort();

    let provider_cookie_sources = crate::product_policy::visible_provider_ids()
        .map(|id| {
            (
                id.cli_name().to_string(),
                settings.cookie_source(id).to_string(),
            )
        })
        .collect::<HashMap<_, _>>();

    let mut has_manual_cookies = cookies
        .get_all_for_display()
        .into_iter()
        .filter(|entry| crate::product_policy::is_visible_provider_cli_name(&entry.provider_id))
        .map(|entry| entry.provider_id)
        .collect::<Vec<_>>();
    has_manual_cookies.sort();

    let mut has_api_keys = api_keys
        .get_all_for_display()
        .into_iter()
        .filter(|entry| crate::product_policy::is_visible_provider_cli_name(&entry.provider_id))
        .map(|entry| entry.provider_id)
        .collect::<Vec<_>>();
    has_api_keys.sort();

    SafeDiagnostics {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        enabled_providers,
        provider_cookie_sources,
        has_manual_cookies,
        has_api_keys,
        hide_personal_info: settings.hide_personal_info,
        refresh_interval_secs: settings.refresh_interval_secs,
    }
}

#[tauri::command]
pub fn get_safe_diagnostics() -> SafeDiagnostics {
    safe_diagnostics_from(Settings::load(), ManualCookies::load(), ApiKeys::load())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_diagnostics_contains_no_secret_values() {
        let mut settings = Settings::default();
        settings
            .enabled_providers
            .insert(codexbar::core::ProviderId::Claude.cli_name().to_string());
        let mut cookies = ManualCookies::default();
        cookies.set(
            codexbar::core::ProviderId::Codex.cli_name(),
            "session=secret-cookie-value",
        );
        cookies.set(
            codexbar::core::ProviderId::Claude.cli_name(),
            "session=hidden-cookie-value",
        );
        let mut keys = ApiKeys::default();
        keys.set("openrouter", "sk-secret-api-key", None);

        let payload = safe_diagnostics_from(settings, cookies, keys);
        let json = serde_json::to_string(&payload).expect("serialize diagnostics");

        assert_eq!(
            payload.enabled_providers,
            vec![codexbar::core::ProviderId::Codex.cli_name().to_string()]
        );
        assert_eq!(payload.provider_cookie_sources.len(), 1);
        assert!(
            payload
                .provider_cookie_sources
                .contains_key(codexbar::core::ProviderId::Codex.cli_name())
        );
        assert_eq!(
            payload.has_manual_cookies,
            vec![codexbar::core::ProviderId::Codex.cli_name().to_string()]
        );
        assert!(payload.has_api_keys.is_empty());
        assert!(json.contains(codexbar::core::ProviderId::Codex.cli_name()));
        assert!(!json.contains(codexbar::core::ProviderId::Claude.cli_name()));
        assert!(!json.contains("openrouter"));
        assert!(!json.contains("secret-cookie-value"));
        assert!(!json.contains("hidden-cookie-value"));
        assert!(!json.contains("sk-secret-api-key"));
        assert!(!json.contains("session="));
    }
}
