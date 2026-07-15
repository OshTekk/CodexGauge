use codexbar::core::ProviderId;

/// This desktop product runs as a notification-area application.
///
/// The existing main-window surfaces stay compiled for upstream parity, but
/// product entry points must not reveal them while this policy is active.
pub const TRAY_ONLY: bool = true;

/// Whether a product entry point may reveal a surface hosted by the main
/// Tauri window.
pub const fn allows_main_window_surfaces() -> bool {
    !TRAY_ONLY
}

/// Whether the optional floating bar may be restored, opened, or changed.
pub const fn allows_float_bar() -> bool {
    !TRAY_ONLY
}

/// Providers exposed by the CodexGauge desktop product.
///
/// The shared provider registry remains unchanged; desktop surfaces apply this
/// policy at their user-facing boundaries.
pub const VISIBLE_PROVIDERS: &[ProviderId] = &[ProviderId::Codex];

pub fn is_visible_provider(provider: ProviderId) -> bool {
    VISIBLE_PROVIDERS.contains(&provider)
}

pub fn is_visible_provider_cli_name(provider_id: &str) -> bool {
    ProviderId::from_cli_name(provider_id).is_some_and(is_visible_provider)
}

pub fn is_visible_provider_scoped_key(key: &str) -> bool {
    is_visible_provider_cli_name(key.split_once(':').map_or(key, |(provider, _)| provider))
}

pub fn visible_provider_ids() -> impl Iterator<Item = ProviderId> {
    VISIBLE_PROVIDERS.iter().copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_policy_only_accepts_codex() {
        for provider in ProviderId::all() {
            assert_eq!(
                is_visible_provider(*provider),
                *provider == ProviderId::Codex,
                "unexpected desktop visibility for {}",
                provider.cli_name()
            );
        }

        assert!(is_visible_provider_cli_name(ProviderId::Codex.cli_name()));
        assert!(!is_visible_provider_cli_name(ProviderId::Claude.cli_name()));
        assert!(!is_visible_provider_cli_name("unknown"));
        assert!(is_visible_provider_scoped_key("codex:weekly"));
        assert!(!is_visible_provider_scoped_key("claude:weekly"));
    }

    #[test]
    fn tray_only_policy_disables_main_surfaces_and_float_bar() {
        let policy =
            std::hint::black_box((TRAY_ONLY, allows_main_window_surfaces(), allows_float_bar()));

        assert_eq!(policy, (true, false, false));
    }
}
