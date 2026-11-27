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
//!
//! The timer and the clock can differ by a few milliseconds, so the task can wake just
//! before the minute it waited for. It then sleeps the rest of the way, and the jobs of
//! one minute start only once.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use jiff::ZonedDateTime;
use organizr_api::AppState;
use organizr_api::cron::Schedule;
use organizr_plugins::health_checks;
use organizr_settings::Settings;

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
    /// A backup of the install.
    Backup,
    /// The version check (D-23).
    UpdateCheck,
}

impl Job {
    /// Every job.
    pub const ALL: [Self; 3] = [Self::HealthChecks, Self::Backup, Self::UpdateCheck];

    /// The name of the job in the log.
    pub const fn name(self) -> &'static str {
        match self {
            Self::HealthChecks => "health checks",
            Self::Backup => "backup",
            Self::UpdateCheck => "version check",
        }
    }

    /// The schedule of the job, when the settings switch the job on.
    pub fn schedule(self, settings: &Settings) -> Option<&str> {
        match self {
            Self::HealthChecks => (settings.health_checks_cron_run_enabled
                && health_checks::is_ready(settings))
            .then_some(settings.health_checks_cron_run_schedule.as_str()),
            Self::Backup => settings
                .auto_backup_cron_enabled
                .then_some(settings.auto_backup_cron_schedule.as_str()),
            // The schedule of the self-update of PHP is the schedule of the check.
            Self::UpdateCheck => settings
                .auto_update_cron_enabled
                .then_some(settings.auto_update_cron_schedule.as_str()),
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
            Self::Backup => {
                if let Err(error) = organizr_api::create_backup(state).await {
                    tracing::error!(error, "the scheduled backup could not be made");
                }
            }
            Self::UpdateCheck => {
                // The check logs what it found, and why it failed.
                let _ = organizr_api::check_for_update(state).await;
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

/// The second (Unix time) at which the next full minute after `now` begins.
fn next_minute(now: &ZonedDateTime) -> i64 {
    (now.timestamp().as_second().div_euclid(60) + 1) * 60
}

/// Whether the scheduler, woken at `now` for the minute that begins at `target`, starts
/// the jobs of that minute. It does not when it woke before the minute, or when the
/// jobs of the minute of `now` started already (`last`, in minutes of Unix time).
fn is_minute_to_run(now: &ZonedDateTime, target: i64, last: Option<i64>) -> bool {
    let second = now.timestamp().as_second();
    second >= target && last != Some(second.div_euclid(60))
}

/// The mark that a job runs. Dropping it frees the job again, also when the job
/// panics, so that a failed run does not keep the job from starting until a restart.
struct Busy(Arc<AtomicBool>);

impl Busy {
    /// Marks the job as running, or gives nothing when it runs already.
    fn take(flag: &Arc<AtomicBool>) -> Option<Self> {
        (!flag.swap(true, Ordering::SeqCst)).then(|| Self(Arc::clone(flag)))
    }
}

impl Drop for Busy {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Runs the scheduler until the task is dropped.
pub async fn run(state: AppState) {
    let running: Vec<Arc<AtomicBool>> = Job::ALL.iter().map(|_| Arc::default()).collect();
    // `GET /test/cron` says from this whether the scheduler runs.
    state.scheduler_woke(ZonedDateTime::now().timestamp().as_second());
    let mut last_minute = None;
    loop {
        let before = ZonedDateTime::now();
        let target = next_minute(&before);
        tokio::time::sleep(until_next_minute(&before)).await;
        let now = ZonedDateTime::now();
        state.scheduler_woke(now.timestamp().as_second());
        if !is_minute_to_run(&now, target, last_minute) {
            continue;
        }
        last_minute = Some(now.timestamp().as_second().div_euclid(60));
        for job in due_jobs(&state.settings(), &now) {
            let Some(busy) = Job::ALL
                .iter()
                .position(|known| *known == job)
                .and_then(|place| running.get(place))
                .cloned()
            else {
                continue;
            };
            let Some(busy) = Busy::take(&busy) else {
                tracing::warn!(
                    job = job.name(),
                    "the job still runs; this start is left out"
                );
                continue;
            };
            let state = state.clone();
            tokio::spawn(async move {
                // Frees the job when the task ends, in any way.
                let _busy = busy;
                tracing::debug!(job = job.name(), "a scheduled job starts");
                if tokio::time::timeout(JOB_TIME_LIMIT, job.run(&state))
                    .await
                    .is_err()
                {
                    tracing::warn!(job = job.name(), "the job ran too long and was stopped");
                }
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

    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::{Busy, Job, due_jobs, is_due, is_minute_to_run, next_minute, until_next_minute};

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
    fn backup_is_due_at_its_schedule_when_it_is_switched_on() {
        // 22 November 2026 is a Sunday. The default schedule is `@weekly`.
        let sunday = at("2026-11-22T00:00:00+00:00");
        let on = [("autoBackupCronEnabled", json!(true))];
        assert_eq!(due_jobs(&installed_settings(&on), &sunday), [Job::Backup]);
        assert_eq!(
            due_jobs(&installed_settings(&on), &at("2026-11-23T00:00:00+00:00")),
            []
        );
        assert_eq!(due_jobs(&installed_settings(&[]), &sunday), []);

        let nightly = [
            ("autoBackupCronEnabled", json!(true)),
            ("autoBackupCronSchedule", json!("30 3 * * *")),
        ];
        assert_eq!(
            due_jobs(
                &installed_settings(&nightly),
                &at("2026-11-23T03:30:00+00:00")
            ),
            [Job::Backup]
        );
        assert_eq!(due_jobs(&installed_settings(&nightly), &sunday), []);
    }

    #[test]
    fn version_check_is_due_at_the_schedule_of_the_auto_update() {
        // 22 November 2026 is a Sunday. The default schedule is `@weekly`.
        let sunday = at("2026-11-22T00:00:00+00:00");
        let on = [("autoUpdateCronEnabled", json!(true))];
        assert_eq!(
            due_jobs(&installed_settings(&on), &sunday),
            [Job::UpdateCheck]
        );
        assert_eq!(
            due_jobs(&installed_settings(&on), &at("2026-11-23T00:00:00+00:00")),
            []
        );
        assert_eq!(due_jobs(&installed_settings(&[]), &sunday), []);
        let daily = [
            ("autoUpdateCronEnabled", json!(true)),
            ("autoUpdateCronSchedule", json!("15 4 * * *")),
        ];
        assert_eq!(
            due_jobs(
                &installed_settings(&daily),
                &at("2026-11-23T04:15:00+00:00")
            ),
            [Job::UpdateCheck]
        );
    }

    #[tokio::test]
    async fn job_that_panics_still_frees_its_place() {
        let flag = Arc::new(AtomicBool::new(false));
        let busy = Busy::take(&flag).expect("the job is free");
        // A second start finds the job busy.
        assert!(Busy::take(&flag).is_none());
        let task = tokio::spawn(async move {
            let _busy = busy;
            panic!("the job fails");
        });
        assert!(task.await.is_err());
        assert!(!flag.load(Ordering::SeqCst));
        assert!(Busy::take(&flag).is_some());
    }

    #[test]
    fn scheduler_that_wakes_before_the_minute_waits_for_it() {
        let target = next_minute(&at("2026-11-20T10:09:15+00:00"));
        assert_eq!(
            target,
            at("2026-11-20T10:10:00+00:00").timestamp().as_second()
        );
        // A timer can end a few milliseconds before the clock shows the minute.
        assert!(!is_minute_to_run(
            &at("2026-11-20T10:09:59.995+00:00"),
            target,
            None
        ));
        assert!(is_minute_to_run(
            &at("2026-11-20T10:10:00.001+00:00"),
            target,
            None
        ));
        assert!(is_minute_to_run(
            &at("2026-11-20T10:10:01+00:00"),
            target,
            None
        ));
    }

    #[test]
    fn scheduler_starts_the_jobs_of_a_minute_once() {
        let target = next_minute(&at("2026-11-20T10:09:15+00:00"));
        let ten_past = Some(at("2026-11-20T10:10:00+00:00").timestamp().as_second() / 60);
        // The clock went back after the jobs of 10:10 started.
        assert!(!is_minute_to_run(
            &at("2026-11-20T10:10:30+00:00"),
            target,
            ten_past
        ));
        assert!(is_minute_to_run(
            &at("2026-11-20T10:11:00+00:00"),
            target,
            ten_past
        ));
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
