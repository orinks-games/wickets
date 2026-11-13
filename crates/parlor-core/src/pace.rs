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
//! Holds are of two kinds. A game calls [`RunClock::reset`] before each
//! line the player must act on, and on resume: the hold that follows is a
//! *key-line hold*, bounded only by Hearken's own cap. A line the player
//! asks for ([`RunClock::asked`]) ends a key-line hold. Any other speech,
//! such as the status, repeat and hint keys or outcome chatter, holds the
//! clock only while the *information budget* lasts: [`INFO_HOLD_BUDGET_S`]
//! of real frame time, spent while such a hold keeps the clock still and
//! refilled by one second for every [`REFILL_RUN_S`] seconds the clock
//! runs. So a player cannot keep the dangers frozen by pressing an
//! information key over and over: under endless chatter the clock is held
//! a quarter of the time at most. When the budget is empty the clock runs
//! through the speech ([`ClockState::Spent`]).
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
use crate::{Parlor, ParlorAction, Records, ScoreResult};

/// The information budget when full, in seconds of real frame time: how
/// long speech that is not a key line (one after a [`RunClock::reset`], or
/// the run's opening lines) may hold the clock before it runs through. The same as Hearken's cap on one hold.
pub const INFO_HOLD_BUDGET_S: f64 = 20.0;

/// Seconds of running frame time (neither held nor bypassed) that refill
/// one second of the information budget. At 3, endless information speech
/// holds the clock for a quarter of the time: 1 held second for every 3
/// that run.
pub const REFILL_RUN_S: f64 = 3.0;

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

/// "best 1200, relaxed best 900": `word` and the value for normal and
/// quick, then "relaxed" `word` and the relaxed value, leaving out one
/// with no value. `None` when neither has one, so a view never says
/// "relaxed best 0".
pub fn both<T: std::fmt::Display>(
    word: &str,
    normal: Option<T>,
    relaxed: Option<T>,
) -> Option<String> {
    let parts: Vec<String> = [normal.map(|v| format!("{word} {v}"))]
        .into_iter()
        .chain([relaxed.map(|v| format!("relaxed {word} {v}"))])
        .flatten()
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// The bests on board `base` of game `id` and on its relaxed board, as
/// [`both`] says them: "best score 30, relaxed best score 41".
pub fn bests(ctx: &mut Ctx<Parlor>, id: &str, base: &str, word: &str) -> Option<String> {
    let normal = Records::best(ctx, id, base);
    let relaxed = Records::best(ctx, id, &Pace::Relaxed.board(base));
    both(word, normal, relaxed)
}

/// [`bests`] for a game that saves its scores with [`record`]: a best of 0
/// saved before counts as no score, so the view never says "best 0".
pub fn scored_bests(ctx: &mut Ctx<Parlor>, id: &str, base: &str, word: &str) -> Option<String> {
    let normal = best(ctx, id, base);
    let relaxed = best(ctx, id, &Pace::Relaxed.board(base));
    both(word, normal, relaxed)
}

/// Game `id`'s best on `board`, with a best of 0 (saved before
/// [`record`] kept 0s off the boards) counted as no score.
pub fn best(ctx: &mut Ctx<Parlor>, id: &str, board: &str) -> Option<u64> {
    Records::best(ctx, id, board).filter(|&b| b > 0)
}

/// Records `score` on game `id`'s `board` (the pace's own board) as
/// [`Records::record`] does, except that a 0 goes on no board, at any pace:
/// `None`, so no result or view says "best 0" or "relaxed best 0". A 0
/// still marks the game played, and a best of 0 saved before counts as no
/// score, so beating it is a first score.
pub fn record(ctx: &mut Ctx<Parlor>, id: &str, board: &str, score: u64) -> Option<ScoreResult> {
    if score == 0 {
        crate::tutorial::mark_played(ctx, id.split('/').next().unwrap_or(id));
        return None;
    }
    Some(match Records::record(ctx, id, board, score) {
        ScoreResult::NewBest { previous: 0 } => ScoreResult::First,
        r => r,
    })
}

/// "Pace: relaxed".
pub fn label(pace: Pace) -> String {
    format!("Pace: {}", pace.word())
}

/// The game's Pace row in Settings, defaulting to normal.
pub fn row(id: &'static str) -> SettingsRow<Parlor> {
    row_with(id, move |ctx| get(ctx, id))
}

/// The game's Pace row for a game where only `part` is timed, such as
/// "Gift Sort": its help says the pace applies there only.
pub fn row_for(id: &'static str, part: &str) -> SettingsRow<Parlor> {
    let help = format!("{HELP} Pace applies to {part} only.");
    row_helped(id, move |ctx| get(ctx, id), &help)
}

/// What the Pace row's help says.
const HELP: &str = "How fast timed play runs. Relaxed is the slowest and quick the fastest. A change applies from the next run. Stars and unlocks count at every pace. Relaxed scores are saved separately.";

/// The game's Pace row, reading the current pace with `current`: for a
/// game with its own default or an older setting to carry over.
pub fn row_with(
    id: &'static str,
    current: impl Fn(&Ctx<Parlor>) -> Pace + 'static,
) -> SettingsRow<Parlor> {
    row_helped(id, current, HELP)
}

fn row_helped(
    id: &'static str,
    current: impl Fn(&Ctx<Parlor>) -> Pace + 'static,
    help: &str,
) -> SettingsRow<Parlor> {
    let current = std::rc::Rc::new(current);
    let read = std::rc::Rc::clone(&current);
    let after = std::rc::Rc::clone(&current);
    setting(
        &format!("{}_pace", id.replace('-', "_")),
        move |ctx| label(read(ctx)),
        move |ctx, delta| {
            let pace = current(ctx).step(delta);
            set(ctx, id, pace);
            Some(changed(pace))
        },
        help,
    )
    // Left and Right say the same as Enter, so a change made during a run
    // says it waits for the next one.
    .value_first(move |ctx| changed(after(ctx)))
}

/// What the row says once the pace has changed to `pace`.
fn changed(pace: Pace) -> String {
    format!(
        "{}. {} Applies from the next run.",
        label(pace),
        pace.meaning()
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
    /// Speech that was not a key line (see [`RunClock::reset`]) was
    /// playing, but the information budget ran out during this hold, so game
    /// time ran through it (on the first such frame, through the part the budget
    /// could not cover).
    Spent,
}

impl ClockState {
    /// The state's name, for logs and tests.
    pub fn name(self) -> &'static str {
        match self {
            ClockState::Running => "running",
            ClockState::Held => "held",
            ClockState::Bypassed => "bypassed",
            ClockState::Spent => "spent",
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
    /// Game time the hold has kept back since it was last taken: what a
    /// timed score adds back, so a hold never improves a time.
    held_time: f64,
    /// Whether the hold now (or the next one) follows a [`RunClock::reset`]
    /// or the run's start: a key-line hold, not drawn from the budget. It ends once the hold
    /// is over and speech has been quiet.
    keyed: bool,
    /// Seconds of real frame time left for holds that did not follow a
    /// reset, up to [`INFO_HOLD_BUDGET_S`].
    budget: f64,
    /// Whether the budget ran out during the hold now: the clock runs
    /// through the rest of it. It ends with the hold, as `keyed` does.
    spent: bool,
}

impl Default for RunClock {
    fn default() -> Self {
        RunClock::new(Pace::default())
    }
}

impl RunClock {
    /// A clock at `pace`. The run's opening lines are a key-line hold, as
    /// after a [`RunClock::reset`]: the player did not ask for them, so
    /// they do not spend the budget.
    pub fn new(pace: Pace) -> RunClock {
        RunClock {
            pace,
            hold: SpeechHold::new(),
            bypass: false,
            state: ClockState::Running,
            held_time: 0.0,
            keyed: true,
            budget: INFO_HOLD_BUDGET_S,
            spent: false,
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
    ///
    /// - Bypassed: time runs; the budget is neither spent nor refilled.
    /// - Hearken would hold, after a reset: a key-line hold, in full.
    /// - Hearken would hold, otherwise: held as far as the budget goes,
    ///   which pays for it; past that the clock runs ([`ClockState::Spent`])
    ///   until that hold is over, even if the budget refills meanwhile.
    /// - Every second that runs unbypassed refills the budget by a
    ///   [`REFILL_RUN_S`]th of a second, up to [`INFO_HOLD_BUDGET_S`].
    pub fn step(&mut self, busy: bool, dt: f64) -> f64 {
        let dt = dt.max(0.0);
        let bypassed = std::mem::take(&mut self.bypass);
        let (flowed, state) = if bypassed {
            // Time runs this frame; the next hold starts afresh.
            self.hold.reset();
            (dt, ClockState::Bypassed)
        } else {
            let allowed = self.hold.step(busy, dt).clamp(0.0, dt);
            let asked = dt - allowed;
            // A frame of no time still holds while Hearken is holding.
            if asked <= 0.0 && !self.hold.held() {
                (dt, ClockState::Running)
            } else if self.keyed {
                (allowed, ClockState::Held)
            } else if self.spent {
                (dt, ClockState::Spent)
            } else {
                let paid = asked.min(self.budget);
                self.budget -= paid;
                self.spent = paid < asked;
                let state = if self.spent {
                    ClockState::Spent
                } else {
                    ClockState::Held
                };
                (dt - paid, state)
            }
        };
        if !bypassed {
            self.held_time += (dt - flowed) * self.pace.scale();
            if matches!(state, ClockState::Running | ClockState::Spent) {
                self.budget = (self.budget + flowed / REFILL_RUN_S).min(INFO_HOLD_BUDGET_S);
            }
            // A hold, key line or spent, is over once speech has been quiet.
            if self.hold.state() == SpeechHoldState::Flowing {
                self.keyed = false;
                self.spent = false;
            }
        }
        self.state = state;
        flowed * self.pace.scale()
    }

    /// Marks a line the player asked for, such as the status, a repeat or a
    /// hint: call it where such a line is spoken. It ends any key-line hold
    /// a [`RunClock::reset`] granted, so a held key line cannot be stretched
    /// by asking again and again: the hold from here on, and the next one,
    /// draw on the information budget.
    pub fn asked(&mut self) {
        self.keyed = false;
    }

    /// [`RunClock::asked`] when `action` is the status, repeat or hint key:
    /// call it with each press, before the press is handled. A game's own
    /// information keys call `asked` themselves.
    pub fn asked_by(&mut self, action: Option<ParlorAction>) {
        if matches!(
            action,
            Some(ParlorAction::Status | ParlorAction::Repeat | ParlorAction::Hint)
        ) {
            self.asked();
        }
    }

    /// Lets game time run through speech on the next frame only: for an
    /// action the player is in the middle of, such as a scoop under a held
    /// key, which must not freeze. Call it before [`RunClock::game_dt`].
    pub fn bypass(&mut self) {
        self.bypass = true;
    }

    /// Seconds of real frame time left for holds that do not follow a
    /// [`RunClock::reset`].
    pub fn budget(&self) -> f64 {
        self.budget
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

    /// Game time held for speech since the last [`RunClock::take_held_time`]
    /// (or the clock's start): the time the run would have spent at its
    /// pace had nothing been said. Bypassed frames are not held.
    pub fn held_time(&self) -> f64 {
        self.held_time
    }

    /// The held time so far, zeroed for the next scored part, such as a
    /// floor or a crossing.
    pub fn take_held_time(&mut self) -> f64 {
        std::mem::take(&mut self.held_time)
    }

    /// Back to running, as after a pause, with the next hold a key-line
    /// hold: call it before each line the player must act on, and on
    /// resume. That hold is granted in full whatever the budget, and does
    /// not refill it.
    pub fn reset(&mut self) {
        self.hold.reset();
        self.bypass = false;
        self.keyed = true;
        self.spent = false;
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
    fn both_boards_leave_out_one_with_no_score() {
        assert_eq!(
            both("best", Some(30), Some(41)).as_deref(),
            Some("best 30, relaxed best 41")
        );
        assert_eq!(both("best", Some(30), None).as_deref(), Some("best 30"));
        assert_eq!(
            both("best run", None, Some(7)).as_deref(),
            Some("relaxed best run 7")
        );
        assert_eq!(both::<u32>("best", None, None), None);
        let mut h = Headless::new(Default::default(), Base);
        let ctx = &mut h.app.ctx;
        assert_eq!(bests(ctx, "dummy", "shift", "best score"), None);
        Records::record(ctx, "dummy", &Pace::Relaxed.board("shift"), 12);
        assert_eq!(
            bests(ctx, "dummy", "shift", "best score").as_deref(),
            Some("relaxed best score 12")
        );
    }

    #[test]
    fn the_clock_scales_and_holds_for_speech() {
        let mut clock = RunClock::new(Pace::Normal);
        assert!(close(clock.step(false, 1.0), 0.85));
        assert_eq!(clock.state(), ClockState::Running);
        assert_eq!(clock.step(true, 0.5), 0.0);
        assert!(clock.held());
        assert_eq!(clock.step(true, 0.0), 0.0);
        assert!(clock.held(), "a frame of no time still holds");
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
    fn held_time_counts_what_the_hold_kept_back_at_the_pace() {
        let mut clock = RunClock::new(Pace::Relaxed);
        clock.step(false, 1.0);
        assert_eq!(clock.held_time(), 0.0, "running is not held");
        clock.step(true, 1.0);
        clock.step(true, 0.5);
        assert!(close(clock.held_time(), 0.75), "{}", clock.held_time());
        clock.bypass();
        clock.step(true, 1.0);
        assert!(close(clock.held_time(), 0.75), "a bypass is not held");
        clock.reset();
        assert!(close(clock.held_time(), 0.75), "a reset keeps it");
        assert!(close(clock.take_held_time(), 0.75));
        assert_eq!(clock.held_time(), 0.0, "taken for the next part");
        clock.step(true, 0.2);
        assert!(close(clock.held_time(), 0.1));
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

    /// Steps `frames` frames of `dt`, all busy or all quiet, and returns the
    /// game time that ran.
    fn steps(clock: &mut RunClock, busy: bool, frames: usize, dt: f64) -> f64 {
        (0..frames).map(|_| clock.step(busy, dt)).sum()
    }

    /// Information speech until the budget runs out, then quiet long enough
    /// for the hold to end, leaving the budget a little above empty.
    fn drain(clock: &mut RunClock) {
        clock.asked();
        while clock.state() != ClockState::Spent {
            clock.step(true, 0.1);
        }
        steps(clock, false, 6, 0.1);
        assert_eq!(clock.hold_state(), SpeechHoldState::Flowing);
    }

    #[test]
    fn a_runs_opening_lines_hold_in_full_and_spend_nothing() {
        // The run's intro is not a line the player asked for.
        let mut clock = RunClock::new(Pace::Normal);
        assert_eq!(steps(&mut clock, true, 150, 0.1), 0.0, "15 seconds held");
        assert_eq!(clock.state(), ClockState::Held);
        assert_eq!(clock.budget(), INFO_HOLD_BUDGET_S);
        // Asking during it ends the free hold, as after a reset.
        clock.asked();
        steps(&mut clock, true, 10, 0.1);
        assert!(close(clock.budget(), INFO_HOLD_BUDGET_S - 1.0));
        // Once the intro is over, the next line draws on the budget too.
        let mut clock = RunClock::new(Pace::Normal);
        steps(&mut clock, true, 30, 0.1);
        steps(&mut clock, false, 6, 0.1);
        steps(&mut clock, true, 10, 0.1);
        assert!(
            clock.budget() < INFO_HOLD_BUDGET_S - 0.5,
            "{}",
            clock.budget()
        );
        assert!(clock.held());
    }

    #[test]
    fn a_key_line_after_a_reset_holds_in_full_with_an_empty_budget() {
        let mut clock = RunClock::new(Pace::Quick);
        drain(&mut clock);
        let before = clock.budget();
        assert!(before < 0.5, "{before}");
        clock.reset();
        assert_eq!(steps(&mut clock, true, 150, 0.1), 0.0, "15 seconds held");
        assert_eq!(clock.state(), ClockState::Held);
        assert!(close(clock.budget(), before), "neither spent nor refilled");
        // A short breath between two key lines keeps one key-line hold.
        steps(&mut clock, false, 2, 0.1);
        assert_eq!(steps(&mut clock, true, 10, 0.1), 0.0);
        assert!(close(clock.budget(), before + 0.2 / REFILL_RUN_S));
    }

    #[test]
    fn back_to_back_information_lines_hold_for_the_budget_then_run() {
        // Lines of 3 seconds, each after a gap long enough for Hearken to
        // hold afresh: without the budget every line would hold in full.
        let mut clock = RunClock::new(Pace::Quick);
        clock.asked();
        let mut held = 0.0;
        let mut gaps = 0.0;
        while clock.state() != ClockState::Spent {
            for _ in 0..30 {
                held += 0.1 - clock.step(true, 0.1);
                if clock.state() == ClockState::Spent {
                    break;
                }
            }
            if clock.state() != ClockState::Spent {
                steps(&mut clock, false, 6, 0.1);
                gaps += 0.6;
            }
        }
        let budget = INFO_HOLD_BUDGET_S + gaps / REFILL_RUN_S;
        assert!((held - budget).abs() < 1e-6, "held {held} of {budget}");
        assert!(held < INFO_HOLD_BUDGET_S + 3.0, "{held}");
        // The rest of that line runs, though the budget refills meanwhile.
        assert!(close(steps(&mut clock, true, 20, 0.1), 2.0));
        assert_eq!(clock.state(), ClockState::Spent);
        assert!(!clock.held());
        assert!(clock.budget() > 0.5);
        // Once the voice has been quiet, the next line holds again.
        steps(&mut clock, false, 6, 0.1);
        assert_eq!(clock.step(true, 0.1), 0.0);
        assert!(clock.held());
    }

    #[test]
    fn the_budget_refills_at_its_rate_and_never_past_full() {
        let mut clock = RunClock::new(Pace::Relaxed);
        clock.asked();
        steps(&mut clock, true, 100, 0.1);
        assert!((clock.budget() - (INFO_HOLD_BUDGET_S - 10.0)).abs() < 1e-9);
        steps(&mut clock, false, 60, 0.1);
        let expected = INFO_HOLD_BUDGET_S - 10.0 + 6.0 / REFILL_RUN_S;
        assert!(
            (clock.budget() - expected).abs() < 1e-9,
            "{}",
            clock.budget()
        );
        steps(&mut clock, false, 1000, 0.1);
        assert_eq!(clock.budget(), INFO_HOLD_BUDGET_S);
        // Held frames do not refill it.
        steps(&mut clock, true, 10, 0.1);
        let budget = clock.budget();
        steps(&mut clock, true, 10, 0.1);
        assert!(close(clock.budget(), budget - 1.0));
    }

    #[test]
    fn under_endless_information_speech_the_clock_holds_a_quarter() {
        // Status pressed again and again: two seconds of speech, then just
        // enough quiet for the next press to hold afresh.
        let mut clock = RunClock::new(Pace::Quick);
        clock.asked();
        let mut ran = 0.0;
        let mut total = 0.0;
        for _ in 0..3000 {
            clock.asked();
            ran += steps(&mut clock, true, 40, 0.05);
            ran += steps(&mut clock, false, 12, 0.05);
            total += 52.0 * 0.05;
        }
        let share = 1.0 - ran / total;
        assert!((share - 0.25).abs() < 0.01, "held share {share}");
        assert!((clock.held_time() - (total - ran)).abs() < 1e-6);
        // Speech with no quiet at all is bounded by Hearken's own cap.
        let mut clock = RunClock::new(Pace::Quick);
        let ran = steps(&mut clock, true, 60_000, 0.05);
        let share = 1.0 - ran / 3000.0;
        assert!(share < 0.25, "held share {share}");
    }

    #[test]
    fn a_normal_status_read_is_held_once_the_budget_has_refilled() {
        let mut clock = RunClock::new(Pace::Normal);
        for _ in 0..5 {
            drain(&mut clock);
            // Fifteen seconds of play, then a four-second status read.
            steps(&mut clock, false, 150, 0.1);
            assert_eq!(steps(&mut clock, true, 40, 0.1), 0.0);
            assert!(clock.held());
            steps(&mut clock, false, 6, 0.1);
        }
    }

    #[test]
    fn bypassed_frames_neither_spend_nor_refill() {
        let mut clock = RunClock::new(Pace::Quick);
        clock.asked();
        steps(&mut clock, true, 50, 0.1);
        let budget = clock.budget();
        for busy in [true, false] {
            for _ in 0..50 {
                clock.bypass();
                clock.step(busy, 0.1);
            }
            assert_eq!(clock.state(), ClockState::Bypassed);
            assert_eq!(clock.budget(), budget);
        }
        assert!(close(clock.held_time(), 5.0));
    }

    /// Status pressed again and again for `seconds`: each press asks, then
    /// two seconds of speech and a short breath that does not end the hold.
    /// Gives the time held.
    fn spam(clock: &mut RunClock, seconds: f64) -> f64 {
        let mut held = 0.0;
        let mut t = 0.0;
        while t < seconds {
            clock.asked();
            for _ in 0..20 {
                held += 0.1 - clock.step(true, 0.1);
            }
            held += 0.1 - clock.step(false, 0.1);
            t += 2.1;
        }
        held
    }

    #[test]
    fn asking_during_a_key_line_ends_its_free_hold() {
        let mut clock = RunClock::new(Pace::Quick);
        // Half the budget spent on earlier information lines.
        clock.asked();
        steps(&mut clock, true, 100, 0.1);
        steps(&mut clock, false, 6, 0.1);
        let budget = clock.budget();
        clock.reset();
        let key = 2.0;
        assert_eq!(steps(&mut clock, true, 20, 0.1), 0.0, "the key line holds");
        let held = key + spam(&mut clock, 60.0);
        let refill = 60.0 / REFILL_RUN_S / 2.0;
        assert!(
            held <= key + budget + refill,
            "held {held}, key line {key}, budget {budget}"
        );
        assert!(held < 20.0 - 1.0, "short of Hearken's cap: {held}");
        assert_ne!(clock.state(), ClockState::Held);
    }

    #[test]
    fn asking_after_a_resume_draws_on_the_budget() {
        let mut clock = RunClock::new(Pace::Quick);
        drain(&mut clock);
        for _ in 0..3 {
            // Pause and resume, then status again and again: the hold is
            // what the budget had, not a fresh freeze to Hearken's cap.
            let budget = clock.budget();
            clock.reset();
            let held = spam(&mut clock, 20.0);
            // The breaths between presses run, and refill a little.
            assert!(held <= budget + 0.5, "held {held} of {budget}");
            assert!(held < 10.0, "{held}");
        }
    }

    #[test]
    fn a_reset_after_asking_grants_the_next_key_line_in_full() {
        let mut clock = RunClock::new(Pace::Quick);
        drain(&mut clock);
        clock.asked();
        clock.reset();
        assert_eq!(steps(&mut clock, true, 50, 0.1), 0.0);
        assert!(clock.held());
    }

    #[test]
    fn a_score_of_0_goes_on_no_board() {
        let mut h = Headless::new(Default::default(), Base);
        let ctx = &mut h.app.ctx;
        let board = Pace::Relaxed.board("meadow");
        assert_eq!(record(ctx, "dummy", &board, 0), None);
        assert_eq!(Records::best(ctx, "dummy", &board), None);
        assert_eq!(scored_bests(ctx, "dummy", "meadow", "best"), None);
        assert_eq!(record(ctx, "dummy", &board, 12), Some(ScoreResult::First));
        assert_eq!(
            record(ctx, "dummy", &board, 0),
            None,
            "a 0 after a score changes nothing"
        );
        assert_eq!(best(ctx, "dummy", &board), Some(12));
        // A 0 saved before counts as no score.
        Records::record(ctx, "dummy", "meadow", 0);
        assert_eq!(best(ctx, "dummy", "meadow"), None);
        assert_eq!(
            scored_bests(ctx, "dummy", "meadow", "best").as_deref(),
            Some("relaxed best 12")
        );
        assert_eq!(
            bests(ctx, "dummy", "meadow", "best").as_deref(),
            Some("best 0, relaxed best 12"),
            "a game still saving 0s keeps its view"
        );
        assert_eq!(record(ctx, "dummy", "meadow", 5), Some(ScoreResult::First));
        assert_eq!(
            record(ctx, "dummy", "meadow", 9),
            Some(ScoreResult::NewBest { previous: 5 })
        );
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
        // Left and Right confirm the change as Enter does.
        assert_eq!(
            h.spoken(),
            ["Pace: relaxed. Much slower timing. Relaxed scores are saved separately. Applies from the next run."]
        );
        assert_eq!(get(&h.app.ctx, "dummy"), Pace::Relaxed);
        h.clear();
        h.press(Key::Left);
        assert_eq!(
            h.spoken(),
            ["Pace: quick. The original timing. Applies from the next run."]
        );
        assert_eq!(get(&h.app.ctx, "dummy"), Pace::Quick);
    }

    fn part_rows(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![row_for("dummy", "Gift Sort")]
    }

    #[test]
    fn a_row_for_one_part_says_where_it_applies() {
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.game.open = Some(OpenGame::new("dummy", "Dummy", part_rows));
        h.app
            .ctx
            .push(hearken::screens::SettingsScreen::<Parlor>::new());
        h.frame(0.0);
        h.press(Key::Return);
        h.frame(0.0);
        h.clear();
        h.press(Key::F1);
        let help = h.spoken().last().cloned().unwrap_or_default();
        assert!(help.starts_with("How fast timed play runs."), "{help}");
        assert!(help.ends_with("Pace applies to Gift Sort only."), "{help}");
    }
}
