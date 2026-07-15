use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use codexbar::settings::Settings;

const AUTO_REFRESH_POLL_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RefreshPlan {
    run_initial_refresh: bool,
    periodic_interval: Option<Duration>,
}

pub fn install(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let initial_plan = refresh_plan(Settings::load().refresh_interval_secs);
        if initial_plan.run_initial_refresh {
            let _ = crate::commands::do_refresh_providers_if_stale(&app).await;
        }

        let mut schedule = initial_plan
            .periodic_interval
            .map(|interval| (interval, Instant::now() + interval));
        loop {
            let interval = refresh_interval(Settings::load().refresh_interval_secs);
            match interval {
                None => schedule = None,
                Some(interval) => {
                    let now = Instant::now();
                    let scheduled_at = scheduled_at_for_interval(&mut schedule, interval, now);
                    if now >= scheduled_at {
                        let _ = crate::commands::do_refresh_providers_if_stale(&app).await;
                        schedule = Some((
                            interval,
                            next_fixed_tick(scheduled_at, Instant::now(), interval),
                        ));
                    }
                }
            }
            tokio::time::sleep(AUTO_REFRESH_POLL_INTERVAL).await;
        }
    });
}

fn scheduled_at_for_interval(
    schedule: &mut Option<(Duration, Instant)>,
    interval: Duration,
    now: Instant,
) -> Instant {
    match *schedule {
        Some((scheduled_interval, scheduled_at)) if scheduled_interval == interval => scheduled_at,
        _ => {
            let scheduled_at = now + interval;
            *schedule = Some((interval, scheduled_at));
            scheduled_at
        }
    }
}

fn refresh_plan(seconds: u64) -> RefreshPlan {
    RefreshPlan {
        run_initial_refresh: true,
        periodic_interval: refresh_interval(seconds),
    }
}

fn next_fixed_tick(
    previous_scheduled_at: Instant,
    completed_at: Instant,
    interval: Duration,
) -> Instant {
    let mut scheduled_at = previous_scheduled_at + interval;
    while scheduled_at <= completed_at {
        scheduled_at += interval;
    }
    scheduled_at
}

fn powertoys_local_usage_provider_ids(settings: &Settings) -> Vec<String> {
    if !settings.powertoys_status_pipe_enabled {
        return Vec::new();
    }

    crate::product_policy::visible_provider_ids()
        .filter(|provider| settings.is_provider_enabled(*provider))
        .map(|provider| provider.cli_name().to_string())
        .collect()
}

pub(crate) fn schedule_refresh_enrichment(settings: &Settings) {
    let provider_ids = powertoys_local_usage_provider_ids(settings);
    if provider_ids.is_empty() {
        return;
    }
    static ENRICHMENT: OnceLock<Arc<tokio::sync::Mutex<()>>> = OnceLock::new();
    let Ok(guard) = Arc::clone(ENRICHMENT.get_or_init(|| Arc::new(tokio::sync::Mutex::new(()))))
        .try_lock_owned()
    else {
        return;
    };
    tauri::async_runtime::spawn(async move {
        let _guard = guard;
        crate::commands::refresh_provider_local_usage_cache(provider_ids).await;
    });
}

fn refresh_interval(seconds: u64) -> Option<Duration> {
    (seconds > 0).then(|| Duration::from_secs(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_refresh_setting_disables_repetition_but_not_initial_refresh() {
        assert_eq!(
            refresh_plan(0),
            RefreshPlan {
                run_initial_refresh: true,
                periodic_interval: None,
            }
        );
    }

    #[test]
    fn periodic_setting_still_starts_with_background_refresh() {
        assert_eq!(
            refresh_plan(300),
            RefreshPlan {
                run_initial_refresh: true,
                periodic_interval: Some(Duration::from_secs(300)),
            }
        );
    }

    #[test]
    fn enabling_periodic_refresh_persists_the_first_deadline() {
        let now = Instant::now();
        let interval = Duration::from_secs(300);
        let mut schedule = None;

        let scheduled_at = scheduled_at_for_interval(&mut schedule, interval, now);

        assert_eq!(scheduled_at, now + interval);
        assert_eq!(schedule, Some((interval, scheduled_at)));
    }

    #[test]
    fn changing_periodic_refresh_replaces_the_old_deadline() {
        let now = Instant::now();
        let old_interval = Duration::from_secs(300);
        let new_interval = Duration::from_secs(600);
        let mut schedule = Some((old_interval, now + old_interval));

        let scheduled_at = scheduled_at_for_interval(&mut schedule, new_interval, now);

        assert_eq!(scheduled_at, now + new_interval);
        assert_eq!(schedule, Some((new_interval, scheduled_at)));
    }

    #[test]
    fn fixed_cadence_advances_from_the_scheduled_tick() {
        let start = Instant::now();
        let interval = Duration::from_secs(100);
        let first_tick = start + interval;

        assert_eq!(
            next_fixed_tick(first_tick, first_tick + Duration::from_secs(60), interval),
            start + Duration::from_secs(200)
        );
        assert_eq!(
            next_fixed_tick(first_tick, first_tick + Duration::from_secs(260), interval),
            start + Duration::from_secs(400)
        );
    }

    #[test]
    fn powertoys_local_usage_refresh_only_includes_visible_enabled_providers() {
        let mut settings = Settings::default();
        assert!(powertoys_local_usage_provider_ids(&settings).is_empty());

        settings.powertoys_status_pipe_enabled = true;
        settings.enabled_providers = [
            codexbar::core::ProviderId::Codex.cli_name().to_string(),
            codexbar::core::ProviderId::Claude.cli_name().to_string(),
            codexbar::core::ProviderId::Cursor.cli_name().to_string(),
        ]
        .into_iter()
        .collect();

        assert_eq!(
            powertoys_local_usage_provider_ids(&settings),
            vec![codexbar::core::ProviderId::Codex.cli_name().to_string()]
        );
    }
}
