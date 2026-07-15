use codexbar::core::ProviderId;

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
}
