use codexbar::agent_sessions::{
    AgentSession, AgentSessionDiscovery, AgentSessionDiscoveryMode, AgentSessionDiscoveryResult,
    AgentSessionProvider, SessionFocusResult, focus_session,
};
use codexbar::core::ProviderId;
use codexbar::settings::Settings;

fn agent_session_is_visible(session: &AgentSession) -> bool {
    let provider = match session.provider {
        AgentSessionProvider::Codex => ProviderId::Codex,
        AgentSessionProvider::Claude => ProviderId::Claude,
    };
    crate::product_policy::is_visible_provider(provider)
}

fn filter_visible_agent_sessions(
    mut result: AgentSessionDiscoveryResult,
) -> AgentSessionDiscoveryResult {
    if let AgentSessionDiscoveryResult::Hosts(hosts) = &mut result {
        for host in hosts {
            host.sessions.retain(agent_session_is_visible);
        }
    }
    result
}

#[tauri::command]
pub async fn list_agent_sessions() -> AgentSessionDiscoveryResult {
    let settings = Settings::load();
    let mode = if settings.agent_sessions_enabled {
        AgentSessionDiscoveryMode::Enabled {
            ssh_hosts: settings.agent_session_ssh_hosts,
        }
    } else {
        AgentSessionDiscoveryMode::Disabled
    };
    filter_visible_agent_sessions(AgentSessionDiscovery::default().scan(mode).await)
}

#[tauri::command]
pub fn focus_agent_session(session: AgentSession) -> SessionFocusResult {
    if !agent_session_is_visible(&session) {
        return SessionFocusResult::unsupported(
            "This session provider is not available in the desktop product",
        );
    }
    focus_session(&session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use codexbar::agent_sessions::{
        AgentSessionActivity, AgentSessionFocusTarget, AgentSessionHostResult, AgentSessionSource,
        AgentSessionState, AgentSessionWorkspace,
    };

    fn session(provider: AgentSessionProvider) -> AgentSession {
        AgentSession {
            id: format!("{provider:?}"),
            provider,
            source: AgentSessionSource::Unknown,
            state: AgentSessionState::Idle,
            pid: None,
            transcript_path: None,
            host: "local".to_string(),
            workspace: AgentSessionWorkspace {
                cwd: None,
                project_name: None,
            },
            activity: AgentSessionActivity {
                started_at: None,
                last_activity_at: None,
            },
            focus_target: AgentSessionFocusTarget::None,
        }
    }

    #[test]
    fn agent_session_snapshot_only_keeps_codex() {
        let result = filter_visible_agent_sessions(AgentSessionDiscoveryResult::Hosts(vec![
            AgentSessionHostResult::success(
                "local",
                vec![
                    session(AgentSessionProvider::Claude),
                    session(AgentSessionProvider::Codex),
                ],
            ),
        ]));

        let AgentSessionDiscoveryResult::Hosts(hosts) = result else {
            panic!("expected hosts result");
        };
        assert_eq!(hosts[0].sessions.len(), 1);
        assert_eq!(hosts[0].sessions[0].provider, AgentSessionProvider::Codex);
    }
}
