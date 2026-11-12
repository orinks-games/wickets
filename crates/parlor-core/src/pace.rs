//! How fast a game's timed play runs, and the clock that runs it.
//!
//! Every timed game shares three rules:
//!
//! - Its countdowns hold while speech is still playing, at every pace and
//!   in every mode (Hearken's [`SpeechHold`]). The hold never marks or
//!   separates a score.
//! - The player picks a [`Pace`] for the game in Settings: relaxed, normal
//!   or quick. Quick is the original timing, normal a little slower and
//!   relaxed much slower.
//! - Stars, unlocks and progress count at every pace. Normal and quick
//!   share their best scores; relaxed runs keep their own boards
//!   ([`Pace::board`], [`crate::Challenges::submit_at`]).
//!
//! A game adds it in a few lines:
//!
//! ```ignore
//! // Settings: the game's first row.
//! fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
//!     vec![pace::row(ID), /* ... */]
//! }
//! // The play screen: one clock for the run, made when the run starts.
//! self.clock = RunClock::start(ctx, ID);
//! // Its update: every countdown advances by the clock's time.
//! let dt = self.clock.game_dt(ctx, dt);
//! // Its scores: relaxed runs on their own boards.
//! Records::record(ctx, ID, &self.clock.pace().board("course 3"), points);
//! Challenges::submit_at(ctx, &challenge, self.clock.pace(), points);
//! ```

use hearken::screens::SettingsRow;
use hearken::{Ctx, SpeechHold, SpeechHoldState};
use serde::{Deserialize, Serialize};

use crate::menus::settings::setting;
use crate::{Parlor, Records};

/// How fast a game's timed play runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pace {
    /// Much slower, with its own score boards.
    Relaxed,
    /// A little slower than quick: the default.
    #[default]
    Normal,
    /// The original timing.
    Quick,
}

impl Pace {
    /// In the order the Settings row steps through them.
    pub const ALL: [Pace; 3] = [Pace::Relaxed, Pace::Normal, Pace::Quick];

    /// "relaxed", "normal" or "quick", as the Settings row says it.
    pub fn word(self) -> &'static str {
        match self {
            Pace::Relaxed => "relaxed",
            Pace::Normal => "normal",
            Pace::Quick => "quick",
        }
    }

    /// Game time for each second of play: 1.0 is the original timing.
    pub fn scale(self) -> f64 {
        match self {
            Pace::Relaxed => 0.5,
            Pace::Normal => 0.85,
            Pace::Quick => 1.0,
        }
    }

    /// Whether this pace keeps its own score boards: relaxed only.
    pub fn separate(self) -> bool {
        self == Pace::Relaxed
    }

    /// The score board a run at this pace records on: `"{base}-relaxed"`
    /// for relaxed, `base` for normal and quick, which share it.
    pub fn board(self, base: &str) -> String {
        if self.separate() {
            format!("{base}-relaxed")
        } else {
            base.to_string()
        }
    }

    /// What a best on this pace's board is called in a result or record
    /// line: "relaxed best" for relaxed, else "best".
    pub fn best_word(self) -> &'static str {
        if self.separate() {
            "relaxed best"
        } else {
            "best"
        }
    }

    /// The pace `delta` steps from this one, wrapping round.
    pub fn step(self, delta: i32) -> Pace {
        let at = Pace::ALL.iter().position(|&p| p == self).unwrap_or(1) as i32;
        Pace::ALL[(at + delta).rem_euclid(Pace::ALL.len() as i32) as usize]
    }

    /// What the Settings row says after a change.
    fn meaning(self) -> &'static str {
        match self {
            Pace::Relaxed => "Much slower timing. Relaxed scores are saved separately.",
            Pace::Normal => "A little slower than quick.",
            Pace::Quick => "The original timing.",
        }
    }
}

/// The records key game `id`'s pace is saved under.
pub fn key(id: &str) -> String {
    format!("{id}/settings/pace")
}

/// Game `id`'s saved pace, if one is saved, from records already read.
pub fn stored(ctx: &Ctx<Parlor>, id: &str) -> Option<Pace> {
    Records::peek::<Option<Pace>>(ctx, &key(id))
}

/// Game `id`'s pace: the saved one, else [`Pace::Normal`].
pub fn get(ctx: &Ctx<Parlor>, id: &str) -> Pace {
    get_or(ctx, id, Pace::default())
}

/// Game `id`'s pace: the saved one, else `default`.
pub fn get_or(ctx: &Ctx<Parlor>, id: &str, default: Pace) -> Pace {
    stored(ctx, id).unwrap_or(default)
}

/// Saves game `id`'s pace.
pub fn set(ctx: &mut Ctx<Parlor>, id: &str, pace: Pace) {
    Records::store(ctx, &key(id), &Some(pace));
}

/// "Pace: relaxed".
pub fn label(pace: Pace) -> String {
    format!("Pace: {}", pace.word())
}

/// The game's Pace row in Settings, defaulting to normal.
pub fn row(id: &'static str) -> SettingsRow<Parlor> {
    row_with(id, move |ctx| get(ctx, id))
}

/// The game's Pace row, reading the current pace with `current`: for a
/// game with its own default or an older setting to carry over.
pub fn row_with(
    id: &'static str,
    current: impl Fn(&Ctx<Parlor>) -> Pace + 'static,
) -> SettingsRow<Parlor> {
    let current = std::rc::Rc::new(current);
    let read = std::rc::Rc::clone(&current);
    setting(
        &format!("{}_pace", id.replace('-', "_")),
        move |ctx| label(read(ctx)),
        move |ctx, delta| {
            let pace = current(ctx).step(delta);
            set(ctx, id, pace);
            Some(format!(
                "{}. {} Applies from the next run.",
                label(pace),
                pace.meaning()
            ))
        },
        "How fast timed play runs. Relaxed is the slowest and quick the fastest. A change applies from the next run. Stars and unlocks count at every pace. Relaxed scores are saved separately.",
    )
}

/// Where a [`RunClock`] stood on its last frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockState {
    /// Game time ran, at the pace's scale.
    Running,
    /// Speech was playing, so game time stood still.
    Held,
    /// The game let time run through speech for this frame, such as under
    /// a held key (see [`RunClock::bypass`]).
    Bypassed,
}

impl ClockState {
    /// The state's name, for logs and tests.
    pub fn name(self) -> &'static str {
        match self {
            ClockState::Running => "running",
            ClockState::Held => "held",
            ClockState::Bypassed => "bypassed",
        }
    }
}

/// One run's game clock: the run's pace, fixed when the run starts, and a
/// [`SpeechHold`]. Every countdown in the run advances by
/// [`RunClock::game_dt`] instead of the frame's time.
#[derive(Clone, Debug)]
pub struct RunClock {
    pace: Pace,
    hold: SpeechHold,
    bypass: bool,
    state: ClockState,
}

impl Default for RunClock {
    fn default() -> Self {
        RunClock::new(Pace::default())
    }
}

impl RunClock {
    /// A clock at `pace`.
    pub fn new(pace: Pace) -> RunClock {
        RunClock {
            pace,
            hold: SpeechHold::new(),
            bypass: false,
            state: ClockState::Running,
        }
    }

    /// A clock at game `id`'s saved pace (normal when none is saved).
    pub fn start(ctx: &Ctx<Parlor>, id: &str) -> RunClock {
        RunClock::new(get(ctx, id))
    }

    /// The run's pace, for its score boards and status.
    pub fn pace(&self) -> Pace {
        self.pace
    }

    /// This frame's game time: none while speech is playing (unless the
    /// game bypassed the hold for this frame), else `dt` at the pace's
    /// scale. Call it once a frame, from the screen's `update`.
    pub fn game_dt(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) -> f64 {
        let busy = !self.bypass && ctx.speech_busy();
        self.step(busy, dt)
    }

    /// The same, with the answer to "is speech playing?" given, and with
    /// every transition in one place.
    pub fn step(&mut self, busy: bool, dt: f64) -> f64 {
        let bypassed = std::mem::take(&mut self.bypass);
        let flowed = if bypassed {
            // Time runs this frame; the next hold starts afresh.
            self.hold.reset();
            dt.max(0.0)
        } else {
            self.hold.step(busy, dt)
        };
        self.state = match (bypassed, self.hold.held()) {
            (true, _) => ClockState::Bypassed,
            (false, true) => ClockState::Held,
            (false, false) => ClockState::Running,
        };
        flowed * self.pace.scale()
    }

    /// Lets game time run through speech on the next frame only: for an
    /// action the player is in the middle of, such as a scoop under a held
    /// key, which must not freeze. Call it before [`RunClock::game_dt`].
    pub fn bypass(&mut self) {
        self.bypass = true;
    }

    /// Whether the last frame held game time for speech.
    pub fn held(&self) -> bool {
        self.state == ClockState::Held
    }

    /// Where the clock stood on its last frame.
    pub fn state(&self) -> ClockState {
        self.state
    }

    /// Where the speech hold stands, as Hearken counts it.
    pub fn hold_state(&self) -> SpeechHoldState {
        self.hold.state()
    }

    /// Back to running, as after a pause.
    pub fn reset(&mut self) {
        self.hold.reset();
        self.bypass = false;
        self.state = ClockState::Running;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::testing::Base;
    use crate::OpenGame;
    use hearken::{Headless, Key, PacerConfig};

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn paces_have_words_scales_and_steps() {
        assert_eq!(Pace::default(), Pace::Normal);
        assert_eq!(Pace::ALL.map(Pace::word), ["relaxed", "normal", "quick"]);
        assert_eq!(Pace::ALL.map(Pace::scale), [0.5, 0.85, 1.0]);
        assert_eq!(Pace::Relaxed.step(1), Pace::Normal);
        assert_eq!(Pace::Quick.step(1), Pace::Relaxed);
        assert_eq!(Pace::Relaxed.step(-1), Pace::Quick);
    }

    #[test]
    fn only_relaxed_keeps_its_own_board() {
        assert_eq!(Pace::Relaxed.board("episode-3"), "episode-3-relaxed");
        assert_eq!(Pace::Normal.board("episode-3"), "episode-3");
        assert_eq!(Pace::Quick.board("episode-3"), "episode-3");
        assert_eq!(Pace::Relaxed.best_word(), "relaxed best");
        assert_eq!(Pace::Quick.best_word(), "best");
    }

    #[test]
    fn the_clock_scales_and_holds_for_speech() {
        let mut clock = RunClock::new(Pace::Normal);
        assert!(close(clock.step(false, 1.0), 0.85));
        assert_eq!(clock.state(), ClockState::Running);
        assert_eq!(clock.step(true, 0.5), 0.0);
        assert!(clock.held());
        assert!(close(clock.step(false, 0.5), 0.425));
        assert!(!clock.held());
    }

    #[test]
    fn a_bypass_runs_one_frame_through_speech() {
        let mut clock = RunClock::new(Pace::Relaxed);
        assert_eq!(clock.step(true, 0.1), 0.0);
        clock.bypass();
        assert!(close(clock.step(true, 0.1), 0.05));
        assert_eq!(clock.state(), ClockState::Bypassed);
        assert_eq!(clock.step(true, 0.1), 0.0, "the next frame holds again");
        assert_eq!(clock.state(), ClockState::Held);
    }

    #[test]
    fn a_hold_cannot_last_forever() {
        let mut clock = RunClock::new(Pace::Quick);
        let mut ran = 0.0;
        for _ in 0..300 {
            ran += clock.step(true, 0.1);
        }
        assert!(ran > 5.0, "the hold ends at Hearken's bound: {ran}");
    }

    #[test]
    fn the_clock_holds_while_a_line_is_spoken() {
        let mut h = Headless::new(Default::default(), Base);
        let mut clock = RunClock::new(Pace::Quick);
        let line = "Two customers wait at the counter.";
        h.app.ctx.say(line);
        assert_eq!(clock.game_dt(&mut h.app.ctx, 0.1), 0.0);
        let speak = PacerConfig::DEFAULT.duration_s(line);
        let mut t = 0.0;
        while t < speak + 0.2 {
            h.frame(0.1);
            t += 0.1;
        }
        assert!(close(clock.game_dt(&mut h.app.ctx, 0.1), 0.1));
    }

    #[test]
    fn the_saved_pace_falls_back_to_a_default() {
        let mut h = Headless::new(Default::default(), Base);
        let ctx = &mut h.app.ctx;
        Records::load::<Option<Pace>>(ctx, &key("dummy"));
        assert_eq!(stored(ctx, "dummy"), None);
        assert_eq!(get(ctx, "dummy"), Pace::Normal);
        assert_eq!(get_or(ctx, "dummy", Pace::Relaxed), Pace::Relaxed);
        set(ctx, "dummy", Pace::Quick);
        assert_eq!(get_or(ctx, "dummy", Pace::Relaxed), Pace::Quick);
        assert_eq!(RunClock::start(ctx, "dummy").pace(), Pace::Quick);
    }

    #[test]
    fn relaxed_challenge_results_keep_their_own_best() {
        use crate::{Challenge, Challenges, Date, Period, ScoreResult};
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.game.today = Some(Date::new(2026, 10, 16));
        let ctx = &mut h.app.ctx;
        let ch = Challenge::current(ctx, "dummy", Period::Daily);
        assert!(!Challenges::done(ctx, &ch));
        assert_eq!(
            Challenges::submit_at(ctx, &ch, Pace::Relaxed, 900),
            ScoreResult::First
        );
        assert!(
            Challenges::done(ctx, &ch),
            "a relaxed finish counts as done"
        );
        assert_eq!(Challenges::best(ctx, &ch), None);
        assert_eq!(Challenges::streak(ctx, "dummy", Period::Daily), 1);
        assert_eq!(
            Challenges::summary(ctx, &ch),
            "Daily challenge, done, relaxed best 900, a 1 day streak"
        );
        assert_eq!(
            Challenges::submit_at(ctx, &ch, Pace::Quick, 500),
            ScoreResult::First,
            "the normal board is separate from the relaxed one"
        );
        assert_eq!(
            Challenges::submit_at(ctx, &ch, Pace::Normal, 400),
            ScoreResult::Short { best: 500 },
            "normal and quick share a board"
        );
        assert_eq!(Challenges::best(ctx, &ch), Some(500));
        assert_eq!(Challenges::best_at(ctx, &ch, Pace::Relaxed), Some(900));
        assert_eq!(
            Challenges::summary(ctx, &ch),
            "Daily challenge, done, best 500, relaxed best 900, a 1 day streak"
        );
    }

    fn rows(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![row("dummy")]
    }

    #[test]
    fn the_row_steps_saves_and_explains() {
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.game.open = Some(OpenGame::new("dummy", "Dummy", rows));
        h.app
            .ctx
            .push(hearken::screens::SettingsScreen::<Parlor>::new());
        h.frame(0.0);
        h.press(Key::Return);
        h.frame(0.0);
        assert!(
            h.spoken().iter().any(|l| l.starts_with("Pace: normal")),
            "{:?}",
            h.spoken()
        );
        h.clear();
        h.press(Key::Return);
        assert_eq!(
            h.spoken(),
            ["Pace: quick. The original timing. Applies from the next run."]
        );
        assert_eq!(get(&h.app.ctx, "dummy"), Pace::Quick);
        h.clear();
        h.press(Key::Right);
        assert!(
            h.spoken()[0].starts_with("Pace: relaxed"),
            "{:?}",
            h.spoken()
        );
        assert_eq!(get(&h.app.ctx, "dummy"), Pace::Relaxed);
    }
}
