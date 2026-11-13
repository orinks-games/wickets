//! A short briefing before a run: what the run is, and number keys that
//! play the sounds it needs, before Enter or Space starts it.
//!
//! A game builds a [`Briefing`] when the player chooses a run (Play, an
//! episode, practice) and calls [`Briefing::show`] in place of pushing the
//! run. The screen stack is then title > briefing > play: `then` replaces
//! the briefing with the run, as a `Pages::briefing` does. With "Briefing
//! before play" off in Settings, `show` starts the run at once and nothing
//! is said.
//!
//! States ([`Phase`], named by `State::phase`):
//!
//! - `skipping`: pushed with the setting off. Entering runs `then` at once.
//! - `ready`: the explanation is said, and no cue is waiting.
//! - `naming`: a cue's label is being said. Its sound starts once the label
//!   is said, or after [`LABEL_WAIT_S`] at most.
//! - `sounding`: the cue's sound has started. A loop plays until another
//!   cue plays, the run starts or the player goes back.
//! - `started`: Enter or Space ran `then`.
//! - `left`: Escape went back to the menu that opened it.
//!
//! Keys:
//!
//! - 1 to 9, then 0 for a tenth: say the cue's label and play it. A number
//!   with no cue says nothing.
//! - Left or Up, Right or Down: step to the cue before or after and play
//!   it the same way, so a controller's d-pad can play them too.
//! - Enter or Space (a controller's A): start the run.
//! - Escape (B): go back without starting.
//! - The repeat key (R): say the briefing again.
//! - F1: say the keys.
//!
//! Every other key says nothing.
//!
//! On entering, the explanation is followed by one keys line, such as
//! "1 to 6 play the sounds. Enter starts.", until the player has started a
//! run from a briefing in this game once (`<game>/briefing_learned`); then
//! only the explanation is said. F1 always gives the keys.
//!
//! The briefing is beta work ([`STAGE`]). A stable copy has none: every
//! run starts at once with nothing said, as with the setting off, and
//! [`rows`] lists no Settings row. Games mark their help lines about it
//! `{beta}`. Promoting it is one edit: change [`STAGE`].

use std::rc::Rc;

use hearken::screens::SettingsRow;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, UiSound};

use crate::channel::{self, Stage};
use crate::menus::settings;
use crate::{Parlor, ParlorAction, Records};

/// The channel stage the briefing is at: a copy shows it only when
/// `channel::shows(STAGE)`.
pub const STAGE: Stage = Stage::Beta;

/// The screen's `State::name`, shared by every game, as `pages` is.
pub const NAME: &str = "briefing";

/// The most cues one briefing holds: keys 1 to 9, then 0.
pub const MAX_CUES: usize = 10;

/// The longest a cue's sound waits for its label to be said.
pub const LABEL_WAIT_S: f64 = 0.8;

/// The records key that turns briefings off, for the whole collection.
pub const OFF_KEY: &str = "parlor/briefing_off";

/// The Settings row's id.
pub const ROW_ID: &str = "briefing";

/// How fast a cue still sounding fades when another plays.
const FADE_MS: u32 = 100;

type Play = Rc<dyn Fn(&mut Ctx<Parlor>) -> Vec<Sound>>;
type Done = Box<dyn FnMut(&mut Ctx<Parlor>)>;

/// One sound the run needs: the label said before it, and how to play it.
#[derive(Clone)]
pub struct Cue {
    label: String,
    play: Play,
}

impl Cue {
    /// A sound key played once on the effects bus.
    pub fn new(label: impl Into<String>, key: impl Into<String>) -> Cue {
        let key = key.into();
        Cue::with(label, move |ctx| vec![ctx.audio.play(&key, Bus::EFFECTS)])
    }

    /// A sound key looped on the effects bus until another cue plays.
    pub fn looped(label: impl Into<String>, key: impl Into<String>) -> Cue {
        let key = key.into();
        Cue::with(label, move |ctx| {
            vec![ctx.audio.start_loop(&key, None, Bus::EFFECTS)]
        })
    }

    /// A sound the game plays its own way: panned, pitched, two at once
    /// or a loop. `play` returns what it started, which the briefing stops
    /// when another cue plays.
    pub fn with(
        label: impl Into<String>,
        play: impl Fn(&mut Ctx<Parlor>) -> Vec<Sound> + 'static,
    ) -> Cue {
        Cue {
            label: label.into(),
            play: Rc::new(play),
        }
    }

    /// The label said before the sound, such as "Scoop full".
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// Where the briefing is (see the module's list of states).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Skipping,
    Ready,
    Naming { cue: usize, waited: f64 },
    Sounding { cue: usize },
    Started,
    Left,
}

impl Phase {
    pub fn name(self) -> &'static str {
        match self {
            Phase::Skipping => "skipping",
            Phase::Ready => "ready",
            Phase::Naming { .. } => "naming",
            Phase::Sounding { .. } => "sounding",
            Phase::Started => "started",
            Phase::Left => "left",
        }
    }
}

/// The briefing screen. Build it with [`Briefing::new`], add cues and
/// `then`, and open it with [`Briefing::show`].
pub struct Briefing {
    text: String,
    cues: Vec<Cue>,
    done: Option<Done>,
    phase: Phase,
    /// The cue the arrows step from: the last one played.
    at: Option<usize>,
    /// What the last cue started, stopped when another plays.
    playing: Vec<Sound>,
    /// The music and ambience bed playing on entering, put back on
    /// Escape.
    music: Option<String>,
    bed: Option<String>,
    /// Shown in place of a results menu: its sting is not put back on
    /// Escape, since the screen beneath starts its own music.
    in_place: bool,
}

/// Whether this copy shows briefings at all ([`STAGE`]).
pub fn shown() -> bool {
    channel::shows(STAGE)
}

/// Whether briefings are on: shown in this copy, and "Briefing before
/// play" in Settings, on unless the player turned it off.
pub fn enabled(ctx: &Ctx<Parlor>) -> bool {
    shown() && !Records::peek::<bool>(ctx, OFF_KEY)
}

/// Opens `briefing`, or starts its run at once with the setting off.
pub fn open(ctx: &mut Ctx<Parlor>, briefing: Briefing) {
    briefing.show(ctx);
}

/// The Settings rows a game that shows briefings lists among its own:
/// [`row`], or none on a copy that does not show briefings.
pub fn rows() -> Vec<SettingsRow<Parlor>> {
    if shown() {
        vec![row()]
    } else {
        Vec::new()
    }
}

/// The shared "Briefing before play" row for a game's Settings. Games list
/// it through [`rows`].
pub fn row() -> SettingsRow<Parlor> {
    settings::toggle(
        ROW_ID,
        |on| {
            if on {
                "Briefing before play: on".to_string()
            } else {
                "Briefing before play: off".to_string()
            }
        },
        enabled,
        |ctx, on| Records::store(ctx, OFF_KEY, &!on),
        |_ctx, _on| None,
        "Before each run, a short briefing says what the run is. Number keys play the sounds it needs. Off starts the run at once.",
    )
}

/// Where a game keeps that its player has started a run from a briefing:
/// under the open game's id, or the launcher's own key outside a game.
fn learned_key(ctx: &Ctx<Parlor>) -> String {
    match ctx.game.open.map(|g| g.id) {
        Some(id) => format!("{id}/briefing_learned"),
        None => "parlor/briefing_learned".to_string(),
    }
}

/// "1 to 6 play the sounds. Enter starts."
pub fn keys_line(cues: usize) -> String {
    match cues {
        0 => "Enter starts.".to_string(),
        n => format!("{} Enter starts.", numbers_line(n)),
    }
}

/// "1 plays the sound.", "1 to 6 play the sounds.", "1 to 9 and 0 play
/// the sounds."
fn numbers_line(cues: usize) -> String {
    match cues {
        0 => String::new(),
        1 => "1 plays the sound.".to_string(),
        n if n < MAX_CUES => format!("1 to {n} play the sounds."),
        _ => "1 to 9 and 0 play the sounds.".to_string(),
    }
}

/// The cue a number key plays, from 0: 1 to 9 are the first nine, 0 the
/// tenth, on the main keys or the keypad.
fn number(key: Key) -> Option<usize> {
    Some(match key {
        Key::Num1 | Key::Kp1 => 0,
        Key::Num2 | Key::Kp2 => 1,
        Key::Num3 | Key::Kp3 => 2,
        Key::Num4 | Key::Kp4 => 3,
        Key::Num5 | Key::Kp5 => 4,
        Key::Num6 | Key::Kp6 => 5,
        Key::Num7 | Key::Kp7 => 6,
        Key::Num8 | Key::Kp8 => 7,
        Key::Num9 | Key::Kp9 => 8,
        Key::Num0 | Key::Kp0 => 9,
        _ => return None,
    })
}

impl Briefing {
    /// A briefing that says `text`, the run in a sentence or two, on
    /// entering.
    pub fn new(text: impl Into<String>) -> Briefing {
        Briefing {
            text: text.into(),
            cues: Vec::new(),
            done: None,
            phase: Phase::Ready,
            at: None,
            playing: Vec::new(),
            music: None,
            bed: None,
            in_place: false,
        }
    }

    /// Adds a cue played once from its sound key.
    pub fn cue(self, label: impl Into<String>, key: impl Into<String>) -> Briefing {
        self.cues(vec![Cue::new(label, key)])
    }

    /// Adds a cue looped from its sound key until another plays.
    pub fn looped(self, label: impl Into<String>, key: impl Into<String>) -> Briefing {
        self.cues(vec![Cue::looped(label, key)])
    }

    /// Adds a cue the game plays its own way (see [`Cue::with`]).
    pub fn cue_with(
        self,
        label: impl Into<String>,
        play: impl Fn(&mut Ctx<Parlor>) -> Vec<Sound> + 'static,
    ) -> Briefing {
        self.cues(vec![Cue::with(label, play)])
    }

    /// Adds cues built for this run. At most [`MAX_CUES`] in all.
    pub fn cues(mut self, cues: Vec<Cue>) -> Briefing {
        self.cues.extend(cues);
        debug_assert!(
            self.cues.len() <= MAX_CUES,
            "a briefing holds at most {MAX_CUES} cues, not {}",
            self.cues.len()
        );
        self
    }

    /// What Enter or Space runs: usually `ctx.replace(Run::new(..))`,
    /// which puts the run in the briefing's place. Without it, starting
    /// closes the briefing.
    pub fn then(mut self, done: impl FnMut(&mut Ctx<Parlor>) + 'static) -> Briefing {
        self.done = Some(Box::new(done));
        self
    }

    /// The explanation said on entering.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The cues' labels, in key order.
    pub fn labels(&self) -> Vec<&str> {
        self.cues.iter().map(Cue::label).collect()
    }

    /// Where the briefing is now.
    pub fn current(&self) -> Phase {
        self.phase
    }

    /// Opens the briefing over the current screen, or, with "Briefing
    /// before play" off, starts its run at once with nothing said.
    pub fn show(mut self, ctx: &mut Ctx<Parlor>) {
        if !shown() || Records::load::<bool>(ctx, OFF_KEY) {
            self.phase = Phase::Skipping;
        }
        ctx.push(self);
    }

    /// [`Briefing::show`] in place of the screen on top, such as a results
    /// menu that goes on to a different run: the briefing takes its place,
    /// so Escape goes back to the screen beneath it.
    pub fn show_in_place(mut self, ctx: &mut Ctx<Parlor>) {
        if !shown() || Records::load::<bool>(ctx, OFF_KEY) {
            self.phase = Phase::Skipping;
        }
        self.in_place = true;
        ctx.replace(self);
    }

    fn learned(ctx: &mut Ctx<Parlor>) -> bool {
        let key = learned_key(ctx);
        Records::load(ctx, &key)
    }

    fn speak(&self, ctx: &mut Ctx<Parlor>) {
        if Self::learned(ctx) {
            ctx.say(self.text.clone());
        } else {
            ctx.say(format!("{} {}", self.text, keys_line(self.cues.len())));
        }
    }

    /// Stops what the last cue started.
    fn hush(&mut self, ctx: &mut Ctx<Parlor>) {
        for s in self.playing.drain(..) {
            ctx.audio.stop(s, FADE_MS);
        }
    }

    /// Says cue `i`'s label; its sound follows in `update`.
    fn name_cue(&mut self, ctx: &mut Ctx<Parlor>, i: usize) {
        let Some(label) = self.cues.get(i).map(|c| c.label.clone()) else {
            return;
        };
        ctx.stop_speech();
        self.hush(ctx);
        ctx.say(label);
        self.at = Some(i);
        self.phase = Phase::Naming {
            cue: i,
            waited: 0.0,
        };
    }

    fn sound_cue(&mut self, ctx: &mut Ctx<Parlor>, i: usize) {
        let play = Rc::clone(&self.cues[i].play);
        self.playing = play(ctx);
        self.phase = Phase::Sounding { cue: i };
    }

    /// Left or Right: the cue before or after the last one played, held
    /// at either end. With none played yet, the first.
    fn step(&mut self, ctx: &mut Ctx<Parlor>, forward: bool) {
        let last = self.cues.len().saturating_sub(1);
        let i = match (self.at, forward) {
            (None, _) => 0,
            (Some(i), true) => (i + 1).min(last),
            (Some(i), false) => i.saturating_sub(1),
        };
        self.name_cue(ctx, i);
    }

    fn start(&mut self, ctx: &mut Ctx<Parlor>) {
        let key = learned_key(ctx);
        if !Records::load::<bool>(ctx, &key) {
            Records::store(ctx, &key, &true);
        }
        self.run(ctx);
    }

    fn run(&mut self, ctx: &mut Ctx<Parlor>) {
        self.hush(ctx);
        self.phase = Phase::Started;
        match self.done.take() {
            Some(mut done) => done(ctx),
            None => ctx.pop(),
        }
    }

    fn leave(&mut self, ctx: &mut Ctx<Parlor>) {
        self.hush(ctx);
        self.phase = Phase::Left;
        if let Some(music) = self.music.take() {
            ctx.audio.play_music(Some(&music), true);
        }
        if let Some(bed) = self.bed.take() {
            ctx.audio.set_bed(Some(&bed));
        }
        ctx.play_ui(UiSound::Back);
        ctx.pop();
    }
}

impl State<Parlor> for Briefing {
    fn name(&self) -> &'static str {
        NAME
    }

    fn phase(&self) -> Option<&'static str> {
        Some(self.phase.name())
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.phase == Phase::Skipping {
            self.run(ctx);
            return;
        }
        // The cues play with the music and bed stopped, as in a sound
        // check.
        if !self.in_place {
            self.music = ctx.audio.music().map(str::to_string);
            self.bed = ctx.audio.bed().map(str::to_string);
        }
        ctx.audio.play_music(None, false);
        ctx.audio.set_bed(None);
        self.speak(ctx);
    }

    fn cover(&mut self, ctx: &mut Ctx<Parlor>) {
        self.hush(ctx);
        if matches!(self.phase, Phase::Naming { .. } | Phase::Sounding { .. }) {
            self.phase = Phase::Ready;
        }
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        self.speak(ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.hush(ctx);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() || matches!(self.phase, Phase::Started | Phase::Left) {
            return;
        }
        let action = ctx.action_for(event);
        if action == Some(ParlorAction::Repeat) {
            ctx.stop_speech();
            self.speak(ctx);
            return;
        }
        let key = event.pressed();
        let plain = !matches!(event, InputEvent::KeyDown { mods, .. } if mods.ctrl || mods.alt);
        if let Some(i) = key.filter(|_| plain).and_then(number) {
            if i < self.cues.len() {
                self.name_cue(ctx, i);
            }
            return;
        }
        let back = matches!(action, Some(ParlorAction::Left | ParlorAction::Up))
            || matches!(key, Some(Key::Left | Key::Up));
        let on = matches!(action, Some(ParlorAction::Right | ParlorAction::Down))
            || matches!(key, Some(Key::Right | Key::Down));
        if (back || on) && !self.cues.is_empty() {
            self.step(ctx, on);
            return;
        }
        if action == Some(ParlorAction::Primary)
            || matches!(key, Some(Key::Return | Key::KpEnter | Key::Space))
        {
            self.start(ctx);
            return;
        }
        if key == Some(Key::Escape) {
            self.leave(ctx);
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if let Phase::Naming { cue, waited } = self.phase {
            let waited = waited + dt;
            if waited >= LABEL_WAIT_S || (waited > 0.0 && !ctx.speech_busy()) {
                self.sound_cue(ctx, cue);
            } else {
                self.phase = Phase::Naming { cue, waited };
            }
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let mut out = vec![self.text.clone()];
        out.extend(
            self.cues
                .iter()
                .enumerate()
                .map(|(i, c)| format!("{}: {}", (i + 1) % 10, c.label)),
        );
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let repeat = ctx.control_name(ParlorAction::Repeat);
        let mut line = numbers_line(self.cues.len());
        if !self.cues.is_empty() {
            line.push_str(" Left and Right step through them. ");
        }
        line.push_str(&format!(
            "Enter or Space starts. Escape goes back. {} repeats the briefing.",
            hearken::text::sentence_start(&repeat)
        ));
        line
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::menus::testing::Base;
    use crate::OpenGame;
    use hearken::Headless;

    const KEYS: [&str; 3] = ["test/scoop", "test/pour", "test/bell"];

    fn briefing(ran: Rc<Cell<bool>>) -> Briefing {
        Briefing::new("Serve the orders.")
            .cue("Scoop full", KEYS[0])
            .cue("Enough sauce", KEYS[1])
            .looped("Door bell", KEYS[2])
            .then(move |ctx| {
                ran.set(true);
                ctx.replace(Base);
            })
    }

    fn open_with(setup: impl FnOnce(&mut Headless<Parlor>)) -> (Headless<Parlor>, Rc<Cell<bool>>) {
        let ran = Rc::new(Cell::new(false));
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.game.open = Some(OpenGame::new("game", "Game", |_| Vec::new()));
        setup(&mut h);
        briefing(Rc::clone(&ran)).show(&mut h.app.ctx);
        h.frame(0.0);
        (h, ran)
    }

    fn open() -> (Headless<Parlor>, Rc<Cell<bool>>) {
        open_with(|_| {})
    }

    /// Lets the label be said and the cue's sound start.
    fn settle(h: &mut Headless<Parlor>) {
        for _ in 0..60 {
            h.frame(1.0 / 60.0);
        }
    }

    #[test]
    fn entering_says_the_run_and_the_keys() {
        let (h, ran) = open();
        assert_eq!(h.app.top_name(), NAME);
        assert_eq!(
            h.spoken(),
            ["Serve the orders. 1 to 3 play the sounds. Enter starts."]
        );
        assert!(!ran.get());
    }

    #[test]
    fn a_number_says_the_label_then_plays_its_cue() {
        let (mut h, _) = open();
        h.clear();
        h.press(Key::Num2);
        assert_eq!(h.spoken(), ["Enough sauce"]);
        assert!(h.played().is_empty(), "the label comes first");
        assert_eq!(h.phase(), Some("naming"));
        settle(&mut h);
        assert_eq!(h.played(), [KEYS[1]]);
        assert_eq!(h.phase(), Some("sounding"));
        h.clear();
        h.press(Key::Kp3);
        settle(&mut h);
        assert_eq!(h.spoken(), ["Door bell"]);
        assert_eq!(h.played(), [KEYS[2]]);
    }

    #[test]
    fn a_number_with_no_cue_and_an_unused_key_say_nothing() {
        let (mut h, ran) = open();
        h.clear();
        for key in [Key::Num4, Key::Num0, Key::S, Key::H, Key::Tab, Key::A] {
            h.press(key);
            settle(&mut h);
        }
        assert!(h.spoken().is_empty(), "{:?}", h.spoken());
        assert!(h.played().is_empty(), "{:?}", h.played());
        assert!(!ran.get());
        assert_eq!(h.app.top_name(), NAME);
    }

    #[test]
    fn enter_and_space_both_start() {
        for key in [Key::Return, Key::Space] {
            let (mut h, ran) = open();
            h.press(key);
            h.frame(0.0);
            assert!(ran.get(), "{key:?} starts");
            assert_eq!(h.app.top_name(), "base");
        }
    }

    #[test]
    fn escape_goes_back_without_starting() {
        let (mut h, ran) = open();
        h.press(Key::Num3);
        settle(&mut h);
        h.press(Key::Escape);
        h.frame(0.0);
        assert!(!ran.get());
        assert_eq!(h.app.top_name(), "base");
        assert!(!Records::load::<bool>(
            &mut h.app.ctx,
            "game/briefing_learned"
        ));
    }

    #[test]
    fn the_keys_line_stops_once_a_run_has_started_in_that_game() {
        let (mut h, _) = open();
        h.press(Key::Return);
        h.frame(0.0);
        h.clear();
        briefing(Rc::new(Cell::new(false))).show(&mut h.app.ctx);
        h.frame(0.0);
        assert_eq!(h.spoken(), ["Serve the orders."]);
        h.clear();
        h.press(Key::F1);
        assert_eq!(
            h.spoken(),
            ["1 to 3 play the sounds. Left and Right step through them. Enter or Space starts. Escape goes back. R repeats the briefing."]
        );
        h.clear();
        h.press(Key::R);
        assert_eq!(h.spoken(), ["Serve the orders."]);
    }

    #[test]
    fn with_the_setting_off_the_run_starts_at_once() {
        let (h, ran) = open_with(|h| Records::store(&mut h.app.ctx, OFF_KEY, &true));
        assert!(ran.get());
        assert_eq!(h.app.top_name(), "base");
        assert!(h.spoken().is_empty(), "{:?}", h.spoken());
    }

    #[test]
    fn left_and_right_step_through_the_cues() {
        let (mut h, _) = open();
        let mut heard = Vec::new();
        for key in [
            Key::Right,
            Key::Right,
            Key::Right,
            Key::Right,
            Key::Left,
            Key::Up,
        ] {
            h.clear();
            h.press(key);
            settle(&mut h);
            heard.push((h.spoken().concat(), h.played().concat()));
        }
        let names: Vec<&str> = heard.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(
            names,
            [
                "Scoop full",
                "Enough sauce",
                "Door bell",
                "Door bell",
                "Enough sauce",
                "Scoop full"
            ]
        );
        assert_eq!(heard[1].1, KEYS[1]);
    }

    #[test]
    fn a_looping_cue_stops_on_escape_and_on_start() {
        for key in [Key::Escape, Key::Return] {
            let (mut h, _) = open();
            h.press(Key::Num3);
            settle(&mut h);
            let live = |h: &Headless<Parlor>| {
                h.app
                    .ctx
                    .audio
                    .live()
                    .iter()
                    .any(|l| l.key == KEYS[2] && l.looping)
            };
            assert!(live(&h), "the door bell loops");
            h.press(key);
            h.frame(1.0);
            assert!(!live(&h), "{key:?} stops the loop");
        }
    }

    #[test]
    fn music_and_bed_stop_while_open_and_come_back_on_escape() {
        let ran = Rc::new(Cell::new(false));
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.audio.play_music(Some("test/music"), true);
        h.app.ctx.audio.set_bed(Some("test/bed"));
        h.frame(0.0);
        briefing(Rc::clone(&ran)).show(&mut h.app.ctx);
        h.frame(1.0);
        assert_eq!(h.app.ctx.audio.music(), None);
        assert_eq!(h.app.ctx.audio.bed(), None);
        h.press(Key::Escape);
        h.frame(0.0);
        assert_eq!(h.app.ctx.audio.music(), Some("test/music"));
        assert_eq!(h.app.ctx.audio.bed(), Some("test/bed"));
    }

    #[test]
    fn shown_in_place_it_takes_the_top_screens_place() {
        struct Results;
        impl State<Parlor> for Results {
            fn name(&self) -> &'static str {
                "results"
            }
            fn help(&self, _ctx: &Ctx<Parlor>) -> String {
                String::new()
            }
        }
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.push(Results);
        h.frame(0.0);
        briefing(Rc::new(Cell::new(false))).show_in_place(&mut h.app.ctx);
        h.frame(0.0);
        assert_eq!(h.app.top_name(), NAME);
        h.press(Key::Escape);
        h.frame(0.0);
        assert_eq!(h.app.top_name(), "base");
    }

    #[test]
    fn keys_lines_count_the_cues() {
        assert_eq!(keys_line(0), "Enter starts.");
        assert_eq!(keys_line(1), "1 plays the sound. Enter starts.");
        assert_eq!(keys_line(6), "1 to 6 play the sounds. Enter starts.");
        assert_eq!(keys_line(10), "1 to 9 and 0 play the sounds. Enter starts.");
    }

    #[test]
    fn a_stable_copy_starts_the_run_at_once_and_lists_no_row() {
        channel::pin(Some(Stage::Stable));
        let (h, ran) = open();
        let rows = rows().len();
        let on = enabled(&h.app.ctx);
        channel::pin(None);
        assert!(ran.get());
        assert_ne!(h.app.top_name(), NAME);
        assert!(h.spoken().is_empty(), "{:?}", h.spoken());
        assert_eq!(rows, 0);
        assert!(!on);
    }

    #[test]
    fn a_beta_copy_shows_the_briefing_and_its_row() {
        channel::pin(Some(Stage::Beta));
        let (h, ran) = open();
        let rows = rows().len();
        channel::pin(None);
        assert!(!ran.get());
        assert_eq!(h.app.top_name(), NAME);
        assert_eq!(rows, 1);
    }

    #[test]
    fn the_settings_row_turns_briefings_off() {
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.game.open = Some(OpenGame::new("game", "Game", |_| vec![row()]));
        h.app
            .ctx
            .push(hearken::screens::SettingsScreen::<Parlor>::new());
        h.frame(0.0);
        h.press(Key::Return);
        h.frame(0.0);
        assert!(
            h.spoken()
                .iter()
                .any(|l| l.starts_with("Briefing before play: on")),
            "{:?}",
            h.spoken()
        );
        h.clear();
        h.press(Key::Return);
        assert!(
            h.spoken()[0].starts_with("Briefing before play: off"),
            "{:?}",
            h.spoken()
        );
        assert!(!enabled(&h.app.ctx));
        h.press(Key::F1);
        assert!(h.spoken().last().unwrap().starts_with("Before each run"));
    }
}
