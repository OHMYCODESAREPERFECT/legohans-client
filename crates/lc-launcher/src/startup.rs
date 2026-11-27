//! The startup sequence: splash, update check, account loading, then Home.

/// Startup stages in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Brand splash while core data loads.
    Splash,
    /// Update check.
    UpdateCheck,
    /// Account loading.
    Accounts,
    /// Finished; show Home.
    Done,
}

/// The outcome of one stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepState {
    /// Not reached yet.
    Pending,
    /// Finished successfully.
    Done(String),
    /// Intentionally not performed.
    Skipped(String),
    /// Failed, but startup continues.
    Failed(String),
}

/// The real work done at each stage.
pub trait StartupWork {
    /// Load core data.
    fn init(&mut self) -> StepState;
    /// Check for updates (`enabled` is the user's setting).
    fn update_check(&mut self, enabled: bool) -> StepState;
    /// Load accounts.
    fn load_accounts(&mut self) -> StepState;
}

/// Drives the stages over time. Work runs when a stage is entered, and each stage stays
/// visible for a minimum time so the transitions are readable.
#[derive(Debug)]
pub struct Startup {
    stage: Stage,
    elapsed: f32,
    entered: bool,
    check_updates: bool,
    /// Outcome of init, update check and account loading.
    pub steps: [StepState; 3],
    splash_secs: f32,
    step_secs: f32,
}

impl Startup {
    /// Create the sequence. With `show_splash == false` the stages run without delay.
    pub fn new(show_splash: bool, check_updates: bool) -> Self {
        let (splash_secs, step_secs) = if show_splash { (1.4, 0.7) } else { (0.0, 0.0) };
        Self {
            stage: Stage::Splash,
            elapsed: 0.0,
            entered: false,
            check_updates,
            steps: [StepState::Pending, StepState::Pending, StepState::Pending],
            splash_secs,
            step_secs,
        }
    }

    /// The current stage.
    pub fn stage(&self) -> Stage {
        self.stage
    }

    /// Whether Home can be shown.
    pub fn is_done(&self) -> bool {
        self.stage == Stage::Done
    }

    /// Overall progress from 0 to 1.
    pub fn progress(&self) -> f32 {
        let index = match self.stage {
            Stage::Splash => 0.0,
            Stage::UpdateCheck => 1.0,
            Stage::Accounts => 2.0,
            Stage::Done => return 1.0,
        };
        let min = self.min_secs();
        let within = if min > 0.0 {
            (self.elapsed / min).clamp(0.0, 1.0)
        } else {
            1.0
        };
        ((index + within) / 3.0).clamp(0.0, 1.0)
    }

    fn min_secs(&self) -> f32 {
        match self.stage {
            Stage::Splash => self.splash_secs,
            _ => self.step_secs,
        }
    }

    /// Advance by `dt` seconds, running stage work as stages are entered.
    pub fn tick(&mut self, dt: f32, work: &mut impl StartupWork) {
        let mut dt = dt.max(0.0);
        loop {
            if self.stage == Stage::Done {
                return;
            }
            if !self.entered {
                let (slot, result) = match self.stage {
                    Stage::Splash => (0, work.init()),
                    Stage::UpdateCheck => (1, work.update_check(self.check_updates)),
                    Stage::Accounts => (2, work.load_accounts()),
                    Stage::Done => return,
                };
                self.steps[slot] = result;
                self.entered = true;
            }
            self.elapsed += dt;
            dt = 0.0;
            if self.elapsed < self.min_secs() {
                return;
            }
            self.elapsed = 0.0;
            self.entered = false;
            self.stage = match self.stage {
                Stage::Splash => Stage::UpdateCheck,
                Stage::UpdateCheck => Stage::Accounts,
                Stage::Accounts | Stage::Done => Stage::Done,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recorder {
        calls: Vec<String>,
    }

    impl StartupWork for Recorder {
        fn init(&mut self) -> StepState {
            self.calls.push("init".into());
            StepState::Done("ok".into())
        }
        fn update_check(&mut self, enabled: bool) -> StepState {
            self.calls.push(format!("update:{enabled}"));
            if enabled {
                StepState::Skipped("none".into())
            } else {
                StepState::Skipped("disabled".into())
            }
        }
        fn load_accounts(&mut self) -> StepState {
            self.calls.push("accounts".into());
            StepState::Done("2 accounts".into())
        }
    }

    #[test]
    fn stages_run_in_order_with_minimum_durations() {
        let mut s = Startup::new(true, true);
        let mut work = Recorder::default();
        s.tick(0.0, &mut work);
        assert_eq!(s.stage(), Stage::Splash);
        assert_eq!(work.calls, ["init"]);

        s.tick(1.0, &mut work);
        assert_eq!(s.stage(), Stage::Splash);
        s.tick(0.5, &mut work);
        assert_eq!(s.stage(), Stage::UpdateCheck);
        assert_eq!(work.calls, ["init", "update:true"]);

        s.tick(0.8, &mut work);
        assert_eq!(s.stage(), Stage::Accounts);
        s.tick(0.8, &mut work);
        assert!(s.is_done());
        assert_eq!(work.calls, ["init", "update:true", "accounts"]);
        assert_eq!(s.steps[2], StepState::Done("2 accounts".into()));
        assert_eq!(s.progress(), 1.0);
    }

    #[test]
    fn work_runs_exactly_once_per_stage() {
        let mut s = Startup::new(true, true);
        let mut work = Recorder::default();
        for _ in 0..200 {
            s.tick(0.05, &mut work);
        }
        assert_eq!(work.calls.len(), 3);
    }

    #[test]
    fn without_splash_everything_completes_in_one_tick() {
        let mut s = Startup::new(false, false);
        let mut work = Recorder::default();
        s.tick(0.0, &mut work);
        assert!(s.is_done());
        assert_eq!(work.calls, ["init", "update:false", "accounts"]);
        assert_eq!(s.steps[1], StepState::Skipped("disabled".into()));
    }

    #[test]
    fn progress_is_monotonic() {
        let mut s = Startup::new(true, true);
        let mut work = Recorder::default();
        let mut last = 0.0;
        for _ in 0..100 {
            s.tick(0.05, &mut work);
            let p = s.progress();
            assert!(p >= last);
            last = p;
        }
        assert_eq!(last, 1.0);
    }
}
