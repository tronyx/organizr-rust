//! The scheduler: runs the jobs of the application at the times of their cron
//! schedules, inside the server.
//!
//! PHP Organizr needs an entry in the cron table of the system that calls `cron.php`
//! each minute. Here a task of the server wakes at each full minute and starts the
//! jobs whose schedule names that minute. A job reads the settings when the minute
//! comes, so a change of a schedule or of a switch counts from the next minute on.
//!
//! A job that still runs is not started a second time, and a job is stopped when it
//! runs longer than its time limit.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use jiff::ZonedDateTime;
use organizr_api::AppState;
use organizr_plugins::health_checks;
use organizr_settings::Settings;

use crate::cron::Schedule;

/// How long a job may run.
const JOB_TIME_LIMIT: Duration = Duration::from_mins(10);

/// Whether a cron schedule names the minute of the given time.
pub fn is_due(schedule: &str, now: &ZonedDateTime) -> Result<bool, String> {
    let schedule: Schedule = schedule.parse()?;
    Ok(schedule.names(now))
}

/// One job of the scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Job {
    /// The run of the health checks plugin.
    HealthChecks,
}

impl Job {
    /// Every job.
    pub const ALL: [Self; 1] = [Self::HealthChecks];

    /// The name of the job in the log.
    pub const fn name(self) -> &'static str {
        match self {
            Self::HealthChecks => "health checks",
        }
    }

    /// The schedule of the job, when the settings switch the job on.
    pub fn schedule(self, settings: &Settings) -> Option<&str> {
        match self {
            Self::HealthChecks => (settings.health_checks_cron_run_enabled
                && health_checks::is_ready(settings))
            .then_some(settings.health_checks_cron_run_schedule.as_str()),
        }
    }

    async fn run(self, state: &AppState) {
        match self {
            Self::HealthChecks => {
                let Some(http) = state.outbound() else {
                    tracing::warn!("the health checks cannot run: no requests can be sent");
                    return;
                };
                let settings = state.settings();
                health_checks::run(http, &settings).await;
            }
        }
    }
}

/// The jobs whose schedule names the minute of `now`. A schedule that cannot be read
/// is logged and starts nothing.
pub fn due_jobs(settings: &Settings, now: &ZonedDateTime) -> Vec<Job> {
    Job::ALL
        .into_iter()
        .filter(|job| {
            let Some(schedule) = job.schedule(settings) else {
                return false;
            };
            match is_due(schedule, now) {
                Ok(due) => due,
                Err(error) => {
                    tracing::warn!(job = job.name(), schedule, %error, "the schedule of a job is not valid");
                    false
                }
            }
        })
        .collect()
}

/// The time until the next full minute.
fn until_next_minute(now: &ZonedDateTime) -> Duration {
    let into_minute = u64::try_from(now.second()).unwrap_or(0) * 1_000_000_000
        + u64::try_from(now.subsec_nanosecond()).unwrap_or(0);
    Duration::from_nanos(60_000_000_000_u64.saturating_sub(into_minute))
        .max(Duration::from_millis(1))
}

/// Runs the scheduler until the task is dropped.
pub async fn run(state: AppState) {
    let running: Vec<Arc<AtomicBool>> = Job::ALL.iter().map(|_| Arc::default()).collect();
    loop {
        tokio::time::sleep(until_next_minute(&ZonedDateTime::now())).await;
        let now = ZonedDateTime::now();
        for job in due_jobs(&state.settings(), &now) {
            let Some(busy) = Job::ALL
                .iter()
                .position(|known| *known == job)
                .and_then(|place| running.get(place))
                .cloned()
            else {
                continue;
            };
            if busy.swap(true, Ordering::SeqCst) {
                tracing::warn!(
                    job = job.name(),
                    "the job still runs; this start is left out"
                );
                continue;
            }
            let state = state.clone();
            tokio::spawn(async move {
                tracing::debug!(job = job.name(), "a scheduled job starts");
                if tokio::time::timeout(JOB_TIME_LIMIT, job.run(&state))
                    .await
                    .is_err()
                {
                    tracing::warn!(job = job.name(), "the job ran too long and was stopped");
                }
                busy.store(false, Ordering::SeqCst);
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use jiff::ZonedDateTime;
    use organizr_testkit::installed_settings;
    use serde_json::json;

    use super::{Job, due_jobs, is_due, until_next_minute};

    fn at(time: &str) -> ZonedDateTime {
        format!("{time}[UTC]").parse().expect("the time is valid")
    }

    #[test]
    fn schedule_names_minutes_as_a_cron_table_does() {
        let every_five = "*/5 * * * *";
        assert_eq!(
            is_due(every_five, &at("2026-11-20T10:10:00+00:00")),
            Ok(true)
        );
        // The seconds of the time do not count.
        assert_eq!(
            is_due(every_five, &at("2026-11-20T10:10:59.5+00:00")),
            Ok(true)
        );
        assert_eq!(
            is_due(every_five, &at("2026-11-20T10:11:00+00:00")),
            Ok(false)
        );
        assert_eq!(
            is_due("30 4 * * *", &at("2026-11-20T04:30:10+00:00")),
            Ok(true)
        );
        assert_eq!(
            is_due("30 4 * * *", &at("2026-11-20T05:30:00+00:00")),
            Ok(false)
        );
        // 20 November 2026 is a Friday.
        assert_eq!(
            is_due("0 12 * * 5", &at("2026-11-20T12:00:00+00:00")),
            Ok(true)
        );
        assert_eq!(
            is_due("0 12 * * 1", &at("2026-11-20T12:00:00+00:00")),
            Ok(false)
        );
        assert_eq!(
            is_due("  * * * * *  ", &at("2026-11-20T12:00:00+00:00")),
            Ok(true)
        );
    }

    #[test]
    fn schedule_that_is_no_cron_schedule_is_an_error() {
        for wrong in ["", "often", "61 * * * *", "* * *"] {
            assert!(
                is_due(wrong, &at("2026-11-20T12:00:00+00:00")).is_err(),
                "{wrong}"
            );
        }
    }

    #[test]
    fn job_is_due_only_when_it_is_switched_on_and_ready() {
        let now = at("2026-11-20T10:10:00+00:00");
        let item = json!({"0": [
            {"label": "UUID", "value": "made-up-id"},
            {"label": "Enabled", "value": true},
        ]});
        let ready = vec![
            ("HEALTHCHECKS-enabled", json!(true)),
            ("HEALTHCHECKS-cron-run-enabled", json!(true)),
            ("HEALTHCHECKS-all-items", item),
        ];
        assert_eq!(
            due_jobs(&installed_settings(&ready), &now),
            [Job::HealthChecks]
        );
        assert_eq!(
            due_jobs(
                &installed_settings(&ready),
                &at("2026-11-20T10:11:00+00:00")
            ),
            []
        );

        for changed in [
            ("HEALTHCHECKS-enabled", json!(false)),
            ("HEALTHCHECKS-cron-run-enabled", json!(false)),
            ("HEALTHCHECKS-all-items", json!("")),
            ("HEALTHCHECKS-cron-run-schedule", json!("not a schedule")),
        ] {
            let mut settings = ready.clone();
            settings.push(changed.clone());
            assert_eq!(
                due_jobs(&installed_settings(&settings), &now),
                [],
                "{changed:?}"
            );
        }
    }

    #[test]
    fn scheduler_sleeps_to_the_next_full_minute() {
        assert_eq!(
            until_next_minute(&at("2026-11-20T10:10:15+00:00")),
            Duration::from_secs(45)
        );
        assert_eq!(
            until_next_minute(&at("2026-11-20T10:10:59.75+00:00")),
            Duration::from_millis(250)
        );
        assert_eq!(
            until_next_minute(&at("2026-11-20T10:10:00+00:00")),
            Duration::from_mins(1)
        );
    }
}
