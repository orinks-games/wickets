//! Test helpers every game can use from its own tests.
//!
//! - [`every_action_answers`] presses each shared control on the screen on
//!   top and fails if any of them is met with silence.
//! - [`same_at_frame_rates`] runs a simulation at 30, 60 and 144 frames a
//!   second and fails if the results differ.
//! - [`status_spam_until`] presses the status key every second from the
//!   first frame and says when the run's danger first moved.
//! - [`spend_budget`], [`quiet_until`] and [`held_for`] check that a key
//!   line the game says on its own still holds once the budget is spent.
//!
//! They sit in the library, not behind a feature, because they need only
//! `hearken::Headless`, which Hearken always builds. A game's tests call
//! them as `parlor_core::testkit::every_action_answers(&mut h)`.

use std::fmt::Debug;

use hearken::{Action, Headless, InputEvent};

use crate::{Parlor, ParlorAction};

/// The frame times [`same_at_frame_rates`] runs at: 30, 60 and 144 frames
/// a second.
pub const FRAME_RATES: [f64; 3] = [1.0 / 30.0, 1.0 / 60.0, 1.0 / 144.0];

/// One frame at 60 frames a second, run after each press so a game that
/// answers in `update` is heard too.
const AFTER_PRESS: f64 = 1.0 / 60.0;

/// The status key pressed once a second from the first frame, the voice
/// never quiet, as a player who keeps asking would: steps the run with
/// `frame` in tenths of a second for at most `seconds`, and gives the
/// seconds until `advanced` first reports the danger moved, or `None`.
///
/// A run's danger must move once the information budget
/// ([`crate::pace::INFO_HOLD_BUDGET_S`]) is spent, so a game's test asserts
/// the answer is a little over the budget at most. A start that waits on
/// speech rather than on the clock shows up as `None`.
pub fn status_spam_until(
    h: &mut Headless<Parlor>,
    seconds: u32,
    mut frame: impl FnMut(&mut Headless<Parlor>),
    mut advanced: impl FnMut(&Headless<Parlor>) -> bool,
) -> Option<f64> {
    for tenth in 0..seconds * 10 {
        if tenth % 10 == 0 {
            h.press(hearken::Key::S);
        }
        frame(h);
        if advanced(h) {
            return Some(f64::from(tenth + 1) / 10.0);
        }
    }
    None
}

/// The most a status spam from the first frame may keep a run's danger
/// still: the information budget, and a little for the key lines a run
/// opens with.
pub const SPAM_BOUND_S: f64 = crate::pace::INFO_HOLD_BUDGET_S + 5.0;

/// Spends the information budget as a player who keeps asking would: the
/// status key once a second for [`SPAM_BOUND_S`] seconds of `frame`s in
/// tenths, then the voice is quiet.
pub fn spend_budget(h: &mut Headless<Parlor>, mut frame: impl FnMut(&mut Headless<Parlor>)) {
    for tenth in 0..(SPAM_BOUND_S as u32) * 10 {
        if tenth % 10 == 0 {
            h.press(hearken::Key::S);
        }
        frame(h);
    }
    h.app.ctx.stop_speech();
}

/// Steps `frame` in tenths, the voice quiet before each, for at most
/// `seconds` until `said` sees the line it waits for. Whether it came.
pub fn quiet_until(
    h: &mut Headless<Parlor>,
    seconds: u32,
    mut frame: impl FnMut(&mut Headless<Parlor>),
    mut said: impl FnMut(&Headless<Parlor>) -> bool,
) -> bool {
    for _ in 0..seconds * 10 {
        if said(h) {
            return true;
        }
        h.app.ctx.stop_speech();
        frame(h);
    }
    said(h)
}

/// Steps `frame` in tenths with the line just said still being spoken,
/// for at most `seconds`, and gives the seconds `measure` (the danger's
/// clock) stayed where it was: how long that line held the run.
pub fn held_for(
    h: &mut Headless<Parlor>,
    seconds: u32,
    mut frame: impl FnMut(&mut Headless<Parlor>),
    measure: impl Fn(&Headless<Parlor>) -> f64,
) -> f64 {
    let before = measure(h);
    for tenth in 0..seconds * 10 {
        frame(h);
        if measure(h) != before {
            return f64::from(tenth) / 10.0;
        }
    }
    f64::from(seconds)
}

/// Presses F1 and, when the help opens as pages over play, reads them all
/// with Enter until play is back on top. Everything said stays in
/// `h.spoken()`.
pub fn read_help(h: &mut Headless<Parlor>) {
    h.press(hearken::Key::F1);
    h.frame(AFTER_PRESS);
    read_through(h);
}

/// Reads pages on top, if any, with Enter until they close: a run's
/// opening, or help opened over play. Everything said stays in
/// `h.spoken()`.
pub fn read_through(h: &mut Headless<Parlor>) {
    for _ in 0..30 {
        if h.app.top_name() != "pages" {
            break;
        }
        h.press(hearken::Key::Return);
        h.frame(AFTER_PRESS);
    }
}

/// Presses `action`'s bound key, lets one frame run and lets go of the key.
/// Uses the player's current binding, so a test that rebinds first presses
/// the new key.
pub fn press_action(h: &mut Headless<Parlor>, action: ParlorAction) {
    let chord = h
        .app
        .ctx
        .bindings
        .chords(action)
        .first()
        .copied()
        .unwrap_or_else(|| action.default_keys()[0]);
    h.send(InputEvent::KeyDown {
        key: chord.key,
        mods: chord.mods,
        text: None,
        repeat: false,
    });
    h.frame(AFTER_PRESS);
    h.send(InputEvent::KeyUp {
        key: chord.key,
        mods: chord.mods,
    });
}

/// Whether the screen answered since the last `h.clear()`: a spoken line on
/// either speech channel, or a sound.
pub fn answered(h: &Headless<Parlor>) -> bool {
    let speech = h.speech.borrow();
    !speech.main_lines().is_empty() || !speech.event_lines().is_empty() || !h.played().is_empty()
}

/// Presses every [`ParlorAction`] in turn on the screen on top of `h`, and
/// panics, naming each silent one, unless each press is answered with
/// speech or a sound. Call it with play running, after the start prompt.
///
/// Each press really reaches the game, so put the game where every key is
/// safe: a Primary that ends the round still counts, as long as it is
/// heard. The speech and sound captures are cleared before each press.
pub fn every_action_answers(h: &mut Headless<Parlor>) {
    let mut silent = Vec::new();
    for &action in ParlorAction::all() {
        h.clear();
        let screen = h.app.top_name();
        press_action(h, action);
        if !answered(h) {
            silent.push(format!("{} on {screen}", action.label()));
        }
    }
    h.clear();
    assert!(
        silent.is_empty(),
        "no speech or sound after: {}",
        silent.join(", ")
    );
}

/// Runs `run(dt)` once for each of [`FRAME_RATES`] and panics unless all
/// three results are equal. Returns the result.
///
/// `run` drives the simulation for a fixed span of time in frames of `dt`,
/// for example `(2.0 / dt).round() as usize` frames for two seconds.
/// Floating-point state only matches exactly when the simulation runs on
/// [`crate::sim::FixedStep`]; for a variable-step one, compare rounded
/// values or use [`close_at_frame_rates`].
pub fn same_at_frame_rates<T: PartialEq + Debug>(mut run: impl FnMut(f64) -> T) -> T {
    let mut results = FRAME_RATES.map(&mut run).into_iter();
    let first = results.next().expect("three frame rates");
    for (result, dt) in results.zip(&FRAME_RATES[1..]) {
        assert_eq!(
            first,
            result,
            "1/30 and 1/{:.0} seconds a frame give different results",
            1.0 / dt
        );
    }
    first
}

/// Like [`same_at_frame_rates`] for a list of numbers, allowing each to
/// differ by up to `tolerance`.
pub fn close_at_frame_rates(tolerance: f64, mut run: impl FnMut(f64) -> Vec<f64>) -> Vec<f64> {
    let results = FRAME_RATES.map(&mut run);
    for (result, dt) in results[1..].iter().zip(&FRAME_RATES[1..]) {
        assert_eq!(
            results[0].len(),
            result.len(),
            "1/{:.0} seconds a frame gives a different count",
            1.0 / dt
        );
        for (i, (a, b)) in results[0].iter().zip(result).enumerate() {
            assert!(
                (a - b).abs() <= tolerance,
                "value {i}: {a} at 1/30 seconds a frame, {b} at 1/{:.0}",
                1.0 / dt
            );
        }
    }
    results.into_iter().next().expect("three frame rates")
}

#[cfg(test)]
mod tests {
    use super::*;
    use hearken::{Bus, Ctx, State};

    /// A dummy play screen: speaks for every action but `quiet`, and
    /// answers Hint with a sound only, from its next update.
    struct Dummy {
        quiet: Option<ParlorAction>,
        chime: bool,
    }

    impl State<Parlor> for Dummy {
        fn name(&self) -> &'static str {
            "dummy"
        }
        fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
            ctx.say("Dummy.");
        }
        fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
            match ctx.action_for(event) {
                Some(a) if Some(a) == self.quiet => {}
                Some(ParlorAction::Hint) => self.chime = true,
                Some(a) => ctx.say(format!("{}.", a.label())),
                None => {}
            }
        }
        fn update(&mut self, ctx: &mut Ctx<Parlor>, _dt: f64) {
            if std::mem::take(&mut self.chime) {
                ctx.audio.play("dummy/chime", Bus::EFFECTS);
            }
        }
        fn help(&self, _ctx: &Ctx<Parlor>) -> String {
            String::new()
        }
    }

    fn dummy(quiet: Option<ParlorAction>) -> Headless<Parlor> {
        let mut h = Headless::new(
            Default::default(),
            Dummy {
                quiet,
                chime: false,
            },
        );
        h.frame(0.0);
        h
    }

    #[test]
    fn a_dummy_game_answers_every_action() {
        let mut h = dummy(None);
        every_action_answers(&mut h);
    }

    #[test]
    #[should_panic(expected = "Status on dummy")]
    fn a_silent_action_fails() {
        let mut h = dummy(Some(ParlorAction::Status));
        every_action_answers(&mut h);
    }

    #[test]
    fn press_action_follows_rebinding() {
        let mut h = dummy(None);
        let chord = hearken::Chord::plain(hearken::Key::P);
        let _ = h.app.ctx.bindings.set_chord(ParlorAction::Primary, chord);
        h.clear();
        press_action(&mut h, ParlorAction::Primary);
        assert_eq!(h.spoken(), vec!["Main action."]);
    }

    #[test]
    fn frame_rate_helpers_compare_results() {
        assert_eq!(same_at_frame_rates(|_| 7), 7);
        let near = close_at_frame_rates(0.1, |dt| vec![1.0 + dt]);
        assert!((near[0] - 1.0).abs() < 0.04);
    }

    #[test]
    #[should_panic(expected = "different results")]
    fn frame_rate_differences_fail() {
        same_at_frame_rates(|dt| (1.0 / dt).round() as u32);
    }
}
