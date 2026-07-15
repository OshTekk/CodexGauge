use std::collections::HashSet;

use crate::commands::ProviderCatalogEntry;
use codexbar::locale::{self, LocaleKey};
use codexbar::settings::Language;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrayMenuEntry {
    pub(crate) id: Option<String>,
    pub(crate) label: String,
    pub(crate) children: Vec<Self>,
    pub(crate) is_separator: bool,
    pub(crate) disabled: bool,
    /// When `Some`, this entry renders as a check/checkbox item.
    /// `true` = checked (enabled), `false` = unchecked (disabled).
    pub(crate) checked: Option<bool>,
}

impl TrayMenuEntry {
    fn item(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            label: label.into(),
            children: Vec::new(),
            is_separator: false,
            disabled: false,
            checked: None,
        }
    }

    fn path_segment(&self) -> Option<String> {
        if self.is_separator {
            return None;
        }

        Some(
            self.id
                .clone()
                .unwrap_or_else(|| self.label.to_ascii_lowercase().replace(' ', "_")),
        )
    }
}

#[cfg(test)]
pub(crate) fn build_tray_menu(
    providers: &[ProviderCatalogEntry],
    status_labels: &[(String, String)],
    enabled_providers: &HashSet<String>,
) -> Vec<TrayMenuEntry> {
    build_tray_menu_with(
        providers,
        status_labels,
        enabled_providers,
        false,
        Language::English,
    )
}

pub(crate) fn build_tray_menu_with(
    _providers: &[ProviderCatalogEntry],
    _status_labels: &[(String, String)],
    _enabled_providers: &HashSet<String>,
    _float_bar_enabled: bool,
    lang: Language,
) -> Vec<TrayMenuEntry> {
    let text = |key| locale::get_text(lang, key);

    vec![
        TrayMenuEntry::item("refresh", text(LocaleKey::TrayRefreshAll)),
        TrayMenuEntry::item("settings", text(LocaleKey::TraySettings)),
        TrayMenuEntry::item("quit", text(LocaleKey::MenuQuit)),
    ]
}

pub(crate) fn proof_menu_items(entries: &[TrayMenuEntry], menu_path: &str) -> Option<Vec<String>> {
    proof_menu_entries(entries, menu_path).map(|visible_entries| {
        visible_entries
            .iter()
            .filter(|entry| !entry.is_separator)
            .map(|entry| entry.label.clone())
            .collect()
    })
}

pub(crate) fn proof_menu_context_for_item(
    entries: &[TrayMenuEntry],
    item_id: &str,
) -> Option<(String, Vec<String>)> {
    proof_menu_context_for_item_inner(entries, item_id, "tray")
}

fn proof_menu_context_for_item_inner(
    entries: &[TrayMenuEntry],
    item_id: &str,
    menu_path: &str,
) -> Option<(String, Vec<String>)> {
    for entry in entries {
        if entry.is_separator {
            continue;
        }

        if entry.id.as_deref() == Some(item_id) {
            return proof_menu_items(entries, menu_path)
                .map(|items| (menu_path.to_string(), items));
        }

        if entry.children.is_empty() {
            continue;
        }

        let next_path = format!("{menu_path}/{}", entry.path_segment()?);
        if let Some(context) =
            proof_menu_context_for_item_inner(&entry.children, item_id, &next_path)
        {
            return Some(context);
        }
    }

    None
}

fn proof_menu_entries<'a>(
    entries: &'a [TrayMenuEntry],
    menu_path: &str,
) -> Option<&'a [TrayMenuEntry]> {
    let mut segments = menu_path.split('/');
    if segments.next()? != "tray" {
        return None;
    }

    let mut current = entries;
    for segment in segments {
        let submenu = current.iter().find(|entry| {
            !entry.is_separator
                && !entry.children.is_empty()
                && entry.path_segment().as_deref() == Some(segment)
        })?;
        current = &submenu.children;
    }

    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_provider_catalog() -> Vec<ProviderCatalogEntry> {
        vec![
            ProviderCatalogEntry {
                id: "codex".into(),
                display_name: "Codex".into(),
                cookie_domain: None,
            },
            ProviderCatalogEntry {
                id: "claude".into(),
                display_name: "Claude".into(),
                cookie_domain: None,
            },
        ]
    }

    fn both_enabled() -> HashSet<String> {
        ["codex".to_string(), "claude".to_string()]
            .into_iter()
            .collect()
    }

    #[test]
    fn tray_menu_contains_only_refresh_settings_and_quit() {
        let menu = build_tray_menu_with(
            &sample_provider_catalog(),
            &[("codex".into(), "Codex 92%".into())],
            &both_enabled(),
            true,
            Language::English,
        );

        assert_eq!(
            menu.iter()
                .map(|entry| entry.id.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("refresh"), Some("settings"), Some("quit")]
        );
        assert!(menu.iter().all(|entry| {
            !entry.is_separator
                && !entry.disabled
                && entry.checked.is_none()
                && entry.children.is_empty()
        }));
        assert_eq!(
            proof_menu_items(&menu, "tray").unwrap(),
            vec!["Refresh All", "Settings...", "Quit"]
        );
    }

    #[test]
    fn proof_menu_context_for_leaf_item_returns_parent_menu() {
        let (menu_path, items) = proof_menu_context_for_item(
            &build_tray_menu(&sample_provider_catalog(), &[], &both_enabled()),
            "settings",
        )
        .unwrap();

        assert_eq!(menu_path, "tray");
        assert_eq!(items, vec!["Refresh All", "Settings...", "Quit"]);
    }

    #[test]
    fn tray_menu_static_labels_follow_language() {
        let menu = build_tray_menu_with(
            &sample_provider_catalog(),
            &[("codex".into(), "Codex 92%".into())],
            &both_enabled(),
            true,
            Language::Japanese,
        );
        let items = proof_menu_items(&menu, "tray").unwrap();

        assert_eq!(items, vec!["すべて更新", "設定...", "終了"]);
    }
}
