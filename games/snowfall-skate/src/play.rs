//! The skate: the play screen for every mode.

use std::collections::HashSet;

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, UiSound, Vec3};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{Challenge, Date, Pages, Parlor, ParlorAction, Period, Records, ScoreResult};

use crate::challenge::{self, CHALLENGES_NAME};
use crate::events;
use crate::music::{self, Medley, Mood};
use crate::ponds::{self, pond};
use crate::progress::{self, Progress};
use crate::rules::{
    lane_words, lanterns_words, nearest_lane, points_words, stars_for, stars_words, Course, Cover,
    End, Ev, PatchKind, Skate, Spin, BANK, LINE_TOP,
};
use crate::{sound, HELP, ID, TITLE_NAME};

/// The play screen's `State::name`.
pub const PLAY_NAME: &str = "snowfall_skate_play";
/// Seconds from the end of a skate to its results.
pub const END_PAUSE: f64 = 3.5;
/// Seconds between the sounds of the before-skate hint.
pub const HINT_GAP: f64 = 2.6;
/// The score board of the midnight skate.
pub const MIDNIGHT_BOARD: &str = "midnight";
/// After midnight has passed, the show starts this long into a midnight
/// skate.
pub const LATE_SHOW_MS: i64 = 45_000;

/// What the skate is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// One of the ponds, by number from 1.
    Pond(u32),
    /// The Frost Fair, down the frozen river.
    Fair,
    /// The day's lantern skate.
    Lanterns(Date),
    /// New Year's Eve, to midnight and the fireworks.
    Midnight,
    /// Practice: slow, everything named, nothing scored.
    Practice,
    Challenge(Period),
}

impl Mode {
    pub fn name(self) -> String {
        match self {
            Mode::Pond(n) => pond(n).title(),
            Mode::Fair => events::FAIR_ROW.to_string(),
            Mode::Lanterns(_) => events::LANTERN_ROW.to_string(),
            Mode::Midnight => events::MIDNIGHT_ROW.to_string(),
            Mode::Practice => "Practice skate".to_string(),
            Mode::Challenge(period) => challenge::name(period).to_string(),
        }
    }
}

/// The course a mode skates, from `seed`. A challenge's comes from
/// [`challenge::course_for`] instead, and the lantern skate's from its day.
pub fn course(mode: Mode, seed: u64) -> Course {
    match mode {
        Mode::Pond(n) => Course::new(pond(n).level, seed, Vec::new()),
        Mode::Fair => Course::new(ponds::FAIR, seed, Vec::new()),
        Mode::Lanterns(day) => Course::new(
            ponds::LANTERN_SKATE,
            events::lantern_seed(day),
            events::tune(day),
        ),
        Mode::Midnight => Course::new(without_snow(pond(1).level), seed, Vec::new()),
        Mode::Practice => Course::new(ponds::PRACTICE, seed, Vec::new()),
        Mode::Challenge(period) => challenge::course_seeded(period, seed),
    }
}

/// The village pond with no snow settling: on New Year's Eve the skate
/// runs to midnight.
fn without_snow(mut level: crate::rules::Level) -> crate::rules::Level {
    level.snow = 0.0;
    level
}

/// A seed from the clock, for real play.
pub fn run_seed() -> u64 {
    parlor_core::Rng::from_time().next_u64()
}

/// Whether autoplay is asked for: a debug build with `ECHO_PARLOR_AUTOPLAY` set.
pub fn autoplay_asked() -> bool {
    cfg!(debug_assertions) && std::env::var_os("ECHO_PARLOR_AUTOPLAY").is_some()
}

#[derive(Clone, Copy, Debug)]
enum HintSound {
    Lantern(i32),
    Take,
    Wrong,
    Drift,
    Thin,
    Partner,
    SpinReady,
}

/// One beat of the midnight fireworks.
#[derive(Clone, Copy, Debug)]
enum Beat {
    Launch(f32),
    Burst(&'static str, f32),
    Crowd(&'static str),
    Song,
}

/// Autoplay: the skater skates on their own, a debug build only.
#[derive(Clone, Copy, Debug, Default)]
struct Auto {
    wait: f64,
}

pub struct SkateScreen {
    pub skate: Skate,
    pub mode: Mode,
    challenge: Option<Challenge>,
    taught: HashSet<&'static str>,
    pub music: Medley,
    pond: Option<Sound>,
    snowfall: Option<Sound>,
    blades: Option<(Sound, Sound)>,
    partner: Option<Sound>,
    flames: Vec<(usize, Sound)>,
    patches: Vec<(usize, Sound)>,
    stalls: Vec<(usize, Sound)>,
    spin: Option<Sound>,
    held: Vec<Sound>,
    delayed: Vec<(f64, HintSound)>,
    show: Vec<(f64, Beat)>,
    /// When the midnight show starts, in skate time.
    show_at: Option<i64>,
    showing: bool,
    ending: Option<f64>,
    paused: bool,
    call_outs: bool,
    status: parlor_core::StatusKey,
    last: Option<String>,
    auto: Option<Auto>,
    laps: bool,
    lapped: bool,
    last_lean: i32,
    at_bank: bool,
    /// The start key is still down from the push-off.
    start_held: bool,
    /// Whether this is the last day of the year or the 1st of January.
    new_year: bool,
    /// The line before this frame's events, so a drift says "Line lost"
    /// only when there was one.
    line_before: u32,
}

impl SkateScreen {
    pub fn open(ctx: &mut Ctx<Parlor>, mode: Mode) -> SkateScreen {
        let s = SkateScreen::with_seed(ctx, mode, run_seed());
        if autoplay_asked() {
            s.autoplay()
        } else {
            s
        }
    }

    pub fn autoplay(mut self) -> SkateScreen {
        self.auto = Some(Auto { wait: 3.0 });
        self
    }

    pub fn with_seed(ctx: &mut Ctx<Parlor>, mode: Mode, seed: u64) -> SkateScreen {
        SkateScreen::with_course(ctx, mode, course(mode, seed), seed)
    }

    fn with_course(ctx: &mut Ctx<Parlor>, mode: Mode, course: Course, seed: u64) -> SkateScreen {
        let call_outs = mode == Mode::Practice || crate::call_outs(ctx);
        let today = parlor_core::challenges::today(ctx);
        let show_at = (mode == Mode::Midnight).then(|| {
            let secs = events::secs_to_midnight(parlor_core::story::now(ctx));
            if secs > 0 {
                secs * 1000
            } else {
                LATE_SHOW_MS
            }
        });
        SkateScreen {
            skate: Skate::new(course),
            mode,
            challenge: None,
            taught: HashSet::new(),
            music: Medley::new(seed ^ 0x50f7_c0de),
            pond: None,
            snowfall: None,
            blades: None,
            partner: None,
            flames: Vec::new(),
            patches: Vec::new(),
            stalls: Vec::new(),
            spin: None,
            held: Vec::new(),
            delayed: Vec::new(),
            show: Vec::new(),
            show_at,
            showing: false,
            ending: None,
            paused: false,
            call_outs,
            status: parlor_core::StatusKey::new(),
            last: None,
            auto: None,
            laps: false,
            lapped: false,
            last_lean: 0,
            at_bank: false,
            start_held: false,
            line_before: 0,
            new_year: mode == Mode::Midnight || events::new_year(today),
        }
    }

    /// This skate going round again when it ends, for the tutorial.
    pub fn laps(mut self) -> SkateScreen {
        self.laps = true;
        self
    }

    pub fn challenge(ctx: &mut Ctx<Parlor>, ch: Challenge) -> SkateScreen {
        let course = challenge::course_for(&ch);
        let mut s = SkateScreen::with_course(ctx, Mode::Challenge(ch.period), course, ch.seed);
        s.challenge = Some(ch);
        s
    }

    fn first(&mut self, cue: &'static str) -> bool {
        self.taught.insert(cue)
    }

    fn key(ctx: &Ctx<Parlor>, action: ParlorAction) -> String {
        ctx.control_name(action)
    }

    fn speak(&mut self, ctx: &mut Ctx<Parlor>, line: impl Into<String>) {
        let line = line.into();
        self.last = Some(line.clone());
        ctx.say(line);
    }

    fn practice(&self) -> bool {
        self.mode == Mode::Practice
    }

    fn pairs(&self) -> bool {
        self.skate.course.level.pairs
    }

    fn start_prompt(&self, ctx: &Ctx<Parlor>) -> String {
        format!(
            "{} pushes off. {} plays the sounds.",
            sentence_start(&Self::key(ctx, ParlorAction::Primary)),
            sentence_start(&Self::key(ctx, ParlorAction::Hint))
        )
    }

    fn intro(&self, ctx: &Ctx<Parlor>) -> String {
        let what = match self.mode {
            Mode::Pond(n) => pond(n).scene.to_string(),
            Mode::Fair => format!(
                "The river is frozen, and the fair is out on the ice. {} down to the far bridge.",
                sentence_start(&lanterns_words(ponds::FAIR_LANTERNS))
            ),
            Mode::Lanterns(_) => format!(
                "Tonight's lanterns light in pairs. Take the first lit of each. {} pairs.",
                ponds::LANTERN_PAIRS
            ),
            Mode::Midnight => {
                let secs = events::secs_to_midnight(parlor_core::story::now(ctx));
                if secs > 0 {
                    format!(
                        "New Year's Eve on the village pond. Midnight in {}.",
                        minutes_words(secs)
                    )
                } else {
                    "Just past midnight on the village pond. The late show starts soon.".to_string()
                }
            }
            Mode::Practice => {
                "The village pond, slowly, everything named. Nothing is scored.".to_string()
            }
            Mode::Challenge(_) => challenge::rule(&self.skate.course),
        };
        format!("{}. {what} {}", self.mode.name(), self.start_prompt(ctx))
    }

    /// Every control, in a few sentences: gameplay only.
    pub fn controls(&self, ctx: &Ctx<Parlor>) -> String {
        let k = |a| Self::key(ctx, a);
        let mut parts = vec![format!(
            "Hold {} or {} to lean across the ice. Be in a lantern's lane as it passes to take it.",
            k(ParlorAction::Left),
            k(ParlorAction::Right)
        )];
        if self.pairs() {
            parts.push("Lanterns come in pairs: take the one that lit first.".to_string());
        } else {
            parts.push("Lanterns in a row make a line, worth more each time.".to_string());
        }
        if self.skate.course.has_drifts() || self.skate.course.has_ice() {
            parts.push("Keep out of drifts and thin ice.".to_string());
        }
        if self.skate.course.level.partner {
            parts.push("Your partner skates the easy way a little ahead.".to_string());
        }
        parts.push(format!(
            "Hold {} for a spin, and let go once it is ready.",
            k(ParlorAction::Primary)
        ));
        parts.push(format!(
            "{} speaks the status, twice for more. {} repeats, {} hints, and Escape pauses.",
            sentence_start(&k(ParlorAction::Status)),
            sentence_start(&k(ParlorAction::Repeat)),
            k(ParlorAction::Hint)
        ));
        parts.join(" ")
    }

    /// Where you are, in words: "the middle", "a little left".
    pub fn you_words(&self) -> String {
        let x = self.skate.x;
        if x <= -BANK + 4.0 {
            return "at the left bank".to_string();
        }
        if x >= BANK - 4.0 {
            return "at the right bank".to_string();
        }
        lane_words(nearest_lane(x)).to_string()
    }

    /// The status: what is coming and where you are, then the night so far.
    pub fn status_layers(&self, ctx: &Ctx<Parlor>) -> Vec<String> {
        let s = &self.skate;
        let name = self.mode.name();
        if !s.started() {
            return vec![format!("{name}, not started.")];
        }
        if s.ended().is_some() || self.showing {
            return vec![format!("{name}, over, score {}.", s.score)];
        }
        let mut now: Vec<String> = Vec::new();
        if let Some(i) = s.patch_ahead() {
            let p = s.course.patches[i];
            now.push(format!(
                "{}, {}.",
                p.kind.words(),
                lane_words(nearest_lane(p.x))
            ));
        }
        if let Some(i) = s.next_lantern() {
            let l = s.course.lanterns[i];
            now.push(format!("Lantern, {}.", lane_words(l.lane)));
        }
        if s.spin_ready() {
            now.push("Spin ready.".to_string());
        }
        now.push(format!("You, {}.", self.you_words()));
        let mut more = Vec::new();
        if !self.practice() {
            more.push(format!("score {}", s.score));
        }
        more.push(format!("line {}", s.line));
        if s.snows() {
            more.push(s.cover().words().to_string());
        }
        match self.mode {
            Mode::Lanterns(_) => more.push(format!(
                "pair {} of {}",
                (s.passed_count() + 1).min(ponds::LANTERN_PAIRS as usize),
                ponds::LANTERN_PAIRS
            )),
            Mode::Fair | Mode::Practice => more.push(format!(
                "lantern {} of {}",
                (s.passed_count() + 1).min(s.course.to_take()),
                s.course.to_take()
            )),
            Mode::Midnight => {
                let secs = events::secs_to_midnight(parlor_core::story::now(ctx));
                if secs > 0 {
                    more.push(format!("midnight in {}", minutes_words(secs)));
                }
            }
            _ => {}
        }
        vec![
            now.join(" "),
            sentence_start(&format!("{}.", more.join(", "))),
        ]
    }

    fn hint_all(&mut self, ctx: &mut Ctx<Parlor>) {
        let mut lines = vec![
            "A lantern on the left.".to_string(),
            "A lantern taken.".to_string(),
        ];
        self.delayed.clear();
        self.delayed.push((0.0, HintSound::Lantern(-1)));
        let mut at = HINT_GAP;
        self.delayed.push((at, HintSound::Take));
        at += HINT_GAP;
        if self.pairs() {
            lines.push("The second lit, the wrong one.".to_string());
            self.delayed.push((at, HintSound::Wrong));
            at += HINT_GAP;
        }
        if self.skate.course.level.partner {
            lines.push("Your partner, ahead.".to_string());
            self.delayed.push((at, HintSound::Partner));
            at += HINT_GAP;
        }
        if self.skate.course.has_drifts() {
            lines.push("A drift.".to_string());
            self.delayed.push((at, HintSound::Drift));
            at += HINT_GAP;
        }
        if self.skate.course.has_ice() {
            lines.push("Thin ice.".to_string());
            self.delayed.push((at, HintSound::Thin));
            at += HINT_GAP;
        }
        lines.push("A spin, ready to let go.".to_string());
        self.delayed.push((at, HintSound::SpinReady));
        ctx.say(lines.join(" "));
        self.play_due(ctx, 0.0);
    }

    fn play_due(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let mut due = Vec::new();
        self.delayed.retain_mut(|(wait, what)| {
            *wait -= dt;
            if *wait <= 1e-9 {
                due.push(*what);
                false
            } else {
                true
            }
        });
        for what in due {
            match what {
                HintSound::Lantern(lane) => {
                    sound::play_at(ctx, &sound::light_key(2), sound::lane_place(lane));
                    let s = sound::play_at(ctx, sound::FLAME, sound::lane_place(lane));
                    ctx.audio.stop(s, 1800);
                }
                HintSound::Take => {
                    sound::play_at(ctx, &sound::take_key(4), Vec3::new(0.0, 0.5, 0.0));
                }
                HintSound::Drift => {
                    let s = sound::play_at(ctx, sound::DRIFT, sound::lane_place(1));
                    ctx.audio.stop(s, 2000);
                }
                HintSound::Wrong => {
                    sound::play(ctx, sound::WRONG);
                }
                HintSound::Thin => {
                    let s = sound::play_at(ctx, sound::THIN, sound::lane_place(1));
                    ctx.audio.stop(s, 2000);
                }
                HintSound::Partner => {
                    let s = sound::play_at(ctx, sound::PARTNER, sound::place(0.0, 0.0, 2.5));
                    ctx.audio.stop(s, 2000);
                }
                HintSound::SpinReady => {
                    sound::play(ctx, sound::SPIN_READY);
                }
            }
        }
    }

    fn stop_cues(&mut self, ctx: &mut Ctx<Parlor>) {
        self.delayed.clear();
        for (_, s) in self.flames.drain(..) {
            ctx.audio.stop(s, 200);
        }
        for (_, s) in self.patches.drain(..) {
            ctx.audio.stop(s, 200);
        }
        if let Some(s) = self.spin.take() {
            ctx.audio.stop(s, 100);
        }
    }

    fn stop_air(&mut self, ctx: &mut Ctx<Parlor>) {
        for s in [self.pond.take(), self.snowfall.take(), self.partner.take()]
            .into_iter()
            .flatten()
        {
            ctx.audio.stop(s, 400);
        }
        if let Some((a, b)) = self.blades.take() {
            ctx.audio.stop(a, 300);
            ctx.audio.stop(b, 300);
        }
        for (_, s) in self.stalls.drain(..) {
            ctx.audio.stop(s, 400);
        }
    }

    /// Stops every sound and the music, for the tutorial's end.
    pub fn silence(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_all(ctx);
        self.music.stop(&mut ctx.audio, 300);
        ctx.audio.set_bed(None);
    }

    fn stop_all(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_cues(ctx);
        self.stop_air(ctx);
        for s in self.held.drain(..) {
            ctx.audio.stop(s, 100);
        }
    }

    /// The air round you: the pond or the river, the snowfall, and your
    /// partner's blades.
    fn start_air(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.pond.is_none() {
            let key = if self.mode == Mode::Fair {
                sound::RIVER
            } else {
                sound::POND
            };
            self.pond = Some(ctx.audio.start_loop(key, None, Bus::AMBIENCE));
        }
        if self.snowfall.is_none() {
            let s = ctx.audio.start_loop(sound::SNOWFALL, None, Bus::AMBIENCE);
            ctx.audio
                .set_volume(s, sound::snowfall_volume(self.skate.clear));
            self.snowfall = Some(s);
        }
        self.mix(ctx);
    }

    fn start_moving(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.blades.is_none() {
            let a = ctx
                .audio
                .start_loop(sound::BLADES, Some(sound::UNDER), Bus::EFFECTS);
            let b = ctx
                .audio
                .start_loop(sound::BLADES_SOFT, Some(sound::UNDER), Bus::EFFECTS);
            self.blades = Some((a, b));
        }
        if self.partner.is_none() && self.skate.course.level.partner {
            let at = sound::place(self.skate.partner_x(), self.skate.x, 2.5);
            self.partner = Some(ctx.audio.start_loop(sound::PARTNER, Some(at), Bus::EFFECTS));
        }
        self.mix(ctx);
    }

    /// Every sound that moves or follows the snow, placed for now.
    fn mix(&mut self, ctx: &mut Ctx<Parlor>) {
        let s = &self.skate;
        let now = s.now();
        if let Some(snow) = self.snowfall {
            ctx.audio.set_volume(snow, sound::snowfall_volume(s.clear));
        }
        if let Some((crisp, soft)) = self.blades {
            let c = sound::crisp_volume(s.clear);
            let slow = if s.slowed() || s.stumbling() {
                0.4
            } else {
                1.0
            };
            ctx.audio.set_volume(crisp, c * slow);
            ctx.audio.set_volume(soft, (1.0 - c).max(0.15) * slow);
        }
        if let Some(p) = self.partner {
            ctx.audio
                .set_position(p, Some(sound::place(s.partner_x(), s.x, 2.5)));
        }
        for &(i, f) in &self.flames {
            let l = s.course.lanterns[i];
            let at = sound::place(l.x(), s.x, sound::ahead(l.at, l.lit, now));
            ctx.audio.set_position(f, Some(at));
        }
        for &(i, h) in &self.patches {
            let p = s.course.patches[i];
            let at = sound::place(p.x, s.x, sound::ahead(p.at, p.heard_at(), now));
            ctx.audio.set_position(h, Some(at));
        }
        for &(i, h) in &self.stalls {
            let st = s.course.stalls[i];
            ctx.audio
                .set_position(h, Some(sound::stall_place(st.side, st.at, now)));
        }
    }

    fn mood(&self) -> Mood {
        Mood {
            deep: self.skate.snows() && self.skate.cover() <= Cover::Deep,
            fair: self.mode == Mode::Fair,
            new_year: self.new_year,
        }
    }

    fn start(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_cues(ctx);
        self.skate.start();
        self.start_held = true;
        sound::play(ctx, sound::START);
        self.start_moving(ctx);
        let line = format!(
            "Away. {} and {} lean.",
            sentence_start(&Self::key(ctx, ParlorAction::Left)),
            Self::key(ctx, ParlorAction::Right)
        );
        if self.first("away") || self.practice() {
            ctx.say(line);
        } else {
            ctx.say("Away.");
        }
    }

    fn lantern_line(&self, lane: i32) -> String {
        format!("Lantern, {}.", lane_words(lane))
    }

    fn on_skate(&mut self, ctx: &mut Ctx<Parlor>, ev: Ev) {
        match ev {
            Ev::Lit { lantern } => {
                let l = self.skate.course.lanterns[lantern];
                let at = sound::place(l.x(), self.skate.x, sound::AHEAD_FROM);
                sound::play_at(ctx, &sound::light_key(l.note), at);
                let f = ctx.audio.start_loop(sound::FLAME, Some(at), Bus::EFFECTS);
                self.flames.push((lantern, f));
                if l.decoy {
                    if self.first("pair") {
                        self.speak(ctx, "Two lit. Take the first.");
                    }
                    return;
                }
                let line = self.lantern_line(l.lane);
                let r = Self::key(ctx, ParlorAction::Right);
                let le = Self::key(ctx, ParlorAction::Left);
                if self.first("lantern") {
                    let way = if l.x() < self.skate.x - 10.0 {
                        format!(" Lean with {le}.")
                    } else if l.x() > self.skate.x + 10.0 {
                        format!(" Lean with {r}.")
                    } else {
                        " Stay in its lane.".to_string()
                    };
                    self.speak(ctx, format!("{line}{way}"));
                } else if self.call_outs {
                    self.speak(ctx, line);
                } else {
                    self.last = Some(line);
                }
            }
            Ev::Taken {
                lantern,
                points,
                line,
            } => {
                self.drop_flame(ctx, lantern);
                let note = self.skate.course.lanterns[lantern].note;
                sound::play_at(ctx, &sound::take_key(note), Vec3::new(0.0, 0.5, 0.0));
                let words = if self.first("taken") {
                    Some(format!(
                        "Lantern taken, {}. Take the next for a line.",
                        points_words(points)
                    ))
                } else if line == LINE_TOP {
                    Some(format!("Line of {LINE_TOP}."))
                } else if self.practice() {
                    Some("Taken.".to_string())
                } else {
                    None
                };
                match words {
                    Some(w) => self.speak(ctx, w),
                    None => self.last = Some(format!("Line {line}.")),
                }
            }
            Ev::Wrong { lantern } => {
                self.drop_flame(ctx, lantern);
                sound::play(ctx, sound::WRONG);
                if self.first("wrong") {
                    self.speak(ctx, "The second lit. Line lost. Take the first lit.");
                } else {
                    self.speak(ctx, "Second lit.");
                }
            }
            Ev::Passed { lantern, line_lost } => {
                self.drop_flame(ctx, lantern);
                sound::play_at(ctx, sound::PASSED, Vec3::new(0.0, -1.0, 0.0));
                if line_lost {
                    self.speak(ctx, "Line lost.");
                } else if self.first("passed") || self.practice() {
                    self.speak(ctx, "Missed the lantern.");
                }
            }
            Ev::PatchHeard { patch } => {
                let p = self.skate.course.patches[patch];
                let at = sound::place(p.x, self.skate.x, sound::AHEAD_FROM);
                let h = ctx
                    .audio
                    .start_loop(sound::patch_key(p.kind), Some(at), Bus::EFFECTS);
                self.patches.push((patch, h));
                let line = format!("{}, {}.", p.kind.words(), lane_words(nearest_lane(p.x)));
                let taught = match p.kind {
                    PatchKind::Drift => "drift",
                    PatchKind::Thin => "thin",
                };
                if self.first(taught) {
                    let what = match p.kind {
                        PatchKind::Drift => "It slows you.",
                        PatchKind::Thin => "Lean clear before it cracks.",
                    };
                    self.speak(ctx, format!("{line} {what}"));
                } else if self.call_outs {
                    self.speak(ctx, line);
                } else {
                    self.last = Some(line);
                }
            }
            Ev::Drifted { patch } => {
                self.drop_patch(ctx, patch);
                sound::play_at(ctx, sound::DRIFTED, sound::UNDER);
                let line = if self.line_before > 0 {
                    "Drift. Line lost."
                } else {
                    "Drift."
                };
                self.speak(ctx, line);
            }
            Ev::Cracked { patch } => {
                self.drop_patch(ctx, patch);
                sound::play_at(ctx, sound::CRACK, sound::UNDER);
                let line = if self.line_before > 0 {
                    "Crack! Line lost."
                } else {
                    "Crack!"
                };
                self.speak(ctx, line);
            }
            Ev::PatchClear { patch } => {
                let kind = self.skate.course.patches[patch].kind;
                self.drop_patch(ctx, patch);
                if self.first("clear") {
                    let what = match kind {
                        PatchKind::Drift => "Past the drift.",
                        PatchKind::Thin => "Clear of the thin ice.",
                    };
                    self.speak(ctx, what);
                }
            }
            Ev::SpinReady => {
                sound::play(ctx, sound::SPIN_READY);
                if self.first("spin ready") {
                    self.speak(ctx, "Spin ready. Let go.");
                }
            }
            Ev::Spun(spin) => self.spun(ctx, spin),
            Ev::Deeper(cover) => {
                let line = match cover {
                    Cover::Settling => "Snow settling.",
                    Cover::Deep => "Snow deep. Spins score more.",
                    _ => return,
                };
                self.speak(ctx, line);
                let mood = self.mood();
                self.music.set_mood(mood);
            }
            Ev::Stall { stall } => {
                let st = self.skate.course.stalls[stall];
                let at = sound::stall_place(st.side, st.at, self.skate.now());
                let h = ctx
                    .audio
                    .start_loop(sound::stall_key(st.kind), Some(at), Bus::AMBIENCE);
                self.stalls.push((stall, h));
                let side = if st.side < 0 { "left" } else { "right" };
                let line = format!("{}, on the {side}.", st.kind.words());
                self.speak(ctx, line);
            }
            Ev::Ended(_) if self.laps => {
                self.stop_cues(ctx);
                self.skate.again();
                self.lapped = true;
                self.speak(ctx, "Round again.");
            }
            Ev::Ended(end) => self.ended(ctx, end),
        }
    }

    fn spun(&mut self, ctx: &mut Ctx<Parlor>, spin: Spin) {
        if let Some(s) = self.spin.take() {
            ctx.audio.stop(s, 80);
        }
        let p = Self::key(ctx, ParlorAction::Primary);
        match spin {
            Spin::Landed(points) => {
                sound::play(ctx, sound::SPIN_LAND);
                ctx.audio.play(music::SWEEP, Bus::MUSIC);
                if self.practice() {
                    self.speak(ctx, "Spin landed.");
                } else {
                    self.speak(ctx, format!("Spin, {}.", points_words(points)));
                }
            }
            Spin::Stride => {
                sound::play_at(ctx, sound::STRIDE, sound::UNDER);
                if self.first("stride") {
                    self.speak(ctx, format!("A stride. Hold {p} longer for a spin."));
                }
            }
            Spin::Wobble => {
                sound::play(ctx, sound::WOBBLE);
                let line = if self.line_before > 0 {
                    "Wobble. Line lost. Let go once the spin is ready."
                } else {
                    "Wobble. Let go once the spin is ready."
                };
                self.speak(ctx, line);
            }
            Spin::TooSoon => {
                sound::play_at(ctx, sound::STRIDE, sound::UNDER);
                self.speak(ctx, "One spin a lantern.");
            }
        }
    }

    fn drop_flame(&mut self, ctx: &mut Ctx<Parlor>, lantern: usize) {
        if let Some(i) = self.flames.iter().position(|(l, _)| *l == lantern) {
            let (_, s) = self.flames.remove(i);
            ctx.audio.stop(s, 250);
        }
    }

    fn drop_patch(&mut self, ctx: &mut Ctx<Parlor>, patch: usize) {
        if let Some(i) = self.patches.iter().position(|(p, _)| *p == patch) {
            let (_, s) = self.patches.remove(i);
            ctx.audio.stop(s, 300);
        }
    }

    fn ended(&mut self, ctx: &mut Ctx<Parlor>, end: End) {
        self.stop_cues(ctx);
        if let Some(p) = self.partner.take() {
            ctx.audio.stop(p, 1500);
        }
        if let Some((a, b)) = self.blades.take() {
            ctx.audio.stop(a, 800);
            ctx.audio.stop(b, 800);
        }
        let line = match (end, self.mode) {
            (End::SnowedOver, _) => "The snow has covered the ice.",
            (End::Done, Mode::Fair) => "Under the far bridge. The fair is behind you.",
            (End::Done, Mode::Lanterns(_)) => "The last pair.",
            (End::Done, Mode::Practice) => "Practice done.",
            (End::Done, _) => "The end of the course.",
        };
        sound::play(
            ctx,
            if end == End::SnowedOver {
                sound::SNOWED_OVER
            } else {
                sound::DONE
            },
        );
        self.speak(ctx, line);
        if self.new_year {
            self.music.finale(&mut ctx.audio);
            self.ending = Some(music::YEAR.secs().min(14.0));
        } else {
            self.music.stop(&mut ctx.audio, 2500);
            let won = self.won();
            sound::play(ctx, if won { sound::WON } else { sound::LOST });
            self.ending = Some(END_PAUSE);
        }
    }

    /// Whether the skate earned what it can: a star, or for a mode with no
    /// stars, anything at all.
    fn won(&self) -> bool {
        match self.mode {
            Mode::Pond(n) => stars_for(self.skate.score, pond(n).marks) > 0,
            Mode::Fair => stars_for(self.skate.score, ponds::FAIR_MARKS) > 0,
            _ => self.skate.tally.taken > 0,
        }
    }

    /// The midnight fireworks: the skate stops, the show goes up, and the
    /// village sings the new year in.
    fn start_show(&mut self, ctx: &mut Ctx<Parlor>) {
        self.showing = true;
        self.stop_cues(ctx);
        if let Some(p) = self.partner.take() {
            ctx.audio.stop(p, 1500);
        }
        if let Some((a, b)) = self.blades.take() {
            ctx.audio.stop(a, 800);
            ctx.audio.stop(b, 800);
        }
        self.music.stop(&mut ctx.audio, 1500);
        self.speak(ctx, "Midnight! Happy New Year.");
        let mut rng = parlor_core::Rng::seeded(self.skate.course.seed ^ 0xf1_7e);
        let mut at = 1.5;
        let kinds = [
            "liberty-lights/fireworks/boom",
            "liberty-lights/fireworks/crackle",
            "liberty-lights/fireworks/sparkle",
            "liberty-lights/fireworks/whistle",
        ];
        for i in 0..16 {
            let side = [-1.2f32, 0.0, 1.2][rng.below(3) as usize];
            self.show.push((at, Beat::Launch(side)));
            self.show
                .push((at + 0.9, Beat::Burst(rng.pick(&kinds), side)));
            if i % 5 == 2 {
                self.show
                    .push((at + 1.5, Beat::Crowd("liberty-lights/crowd/ooh")));
            }
            at += 1.2 + rng.unit() * 0.9;
        }
        for k in 0..5 {
            let side = [-1.2f32, -0.6, 0.0, 0.6, 1.2][k];
            self.show.push((at + k as f64 * 0.2, Beat::Launch(side)));
            self.show
                .push((at + 0.9 + k as f64 * 0.2, Beat::Burst(kinds[0], side)));
        }
        self.show
            .push((at + 2.5, Beat::Crowd("liberty-lights/crowd/cheer")));
        self.show.push((at + 4.0, Beat::Song));
    }

    fn play_show(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let mut due = Vec::new();
        self.show.retain_mut(|(wait, beat)| {
            *wait -= dt;
            if *wait <= 1e-9 {
                due.push(*beat);
                false
            } else {
                true
            }
        });
        for beat in due {
            match beat {
                Beat::Launch(side) => {
                    ctx.audio.play_at(
                        "liberty-lights/launch/whoosh",
                        Vec3::new(side * 4.0, 6.0, 0.0),
                        Bus::EFFECTS,
                    );
                }
                Beat::Burst(key, side) => {
                    ctx.audio
                        .play_at(key, Vec3::new(side * 5.0, 9.0, 3.0), Bus::EFFECTS);
                }
                Beat::Crowd(key) => {
                    ctx.audio.play(key, Bus::AMBIENCE);
                }
                Beat::Song => {
                    self.music.finale(&mut ctx.audio);
                    self.ending = Some(music::YEAR.secs().min(14.0));
                }
            }
        }
    }

    fn repeat(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.skate.started() {
            let p = self.start_prompt(ctx);
            ctx.say(p);
            return;
        }
        match self.last.clone() {
            Some(line) => ctx.say(line),
            None => ctx.say("Nothing yet."),
        }
    }

    fn hint(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.skate.started() {
            self.hint_all(ctx);
            return;
        }
        let s = &self.skate;
        let l = Self::key(ctx, ParlorAction::Left);
        let r = Self::key(ctx, ParlorAction::Right);
        let p = Self::key(ctx, ParlorAction::Primary);
        let lean_to = |x: f64| {
            if x < s.x - 8.0 {
                format!("lean with {l}")
            } else if x > s.x + 8.0 {
                format!("lean with {r}")
            } else {
                "hold your lane".to_string()
            }
        };
        let line = if self.showing || s.ended().is_some() {
            "The night is over.".to_string()
        } else if s.spin_ready() {
            format!("Spin ready: let go of {p}.")
        } else if let Some(i) = s.patch_ahead().filter(|&i| s.course.patches[i].under(s.x)) {
            let patch = s.course.patches[i];
            let away = if patch.x > s.x || (patch.x - s.x).abs() < 1.0 && s.x > 0.0 {
                format!("lean with {l}")
            } else {
                format!("lean with {r}")
            };
            format!("{} in your lane: {away}.", patch.kind.words())
        } else if let Some(i) = s.next_lantern() {
            let lantern = s.course.lanterns[i];
            format!(
                "Lantern {}: {}.",
                lane_words(lantern.lane),
                lean_to(lantern.x())
            )
        } else if s.course.level.partner {
            "Follow your partner for the easy way.".to_string()
        } else {
            "A lantern comes next.".to_string()
        };
        ctx.say(line);
    }

    /// Autoplay: follows the partner, takes the first lit, spins now and
    /// then. A debug build only.
    fn auto_lean(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) -> Option<i32> {
        let mut auto = self.auto?;
        if !self.skate.started() {
            auto.wait -= dt;
            self.auto = Some(auto);
            if auto.wait <= 0.0 {
                self.start(ctx);
            }
            return Some(0);
        }
        Some(auto_target(&self.skate))
    }

    fn preview(&mut self, ctx: &mut Ctx<Parlor>, action: ParlorAction) {
        self.delayed.clear();
        match action {
            ParlorAction::Left | ParlorAction::Right => {
                ctx.say(format!(
                    "Once away, hold {} and {} to lean.",
                    Self::key(ctx, ParlorAction::Left),
                    Self::key(ctx, ParlorAction::Right)
                ));
            }
            _ => {
                ctx.play_ui(UiSound::Error);
                let p = self.start_prompt(ctx);
                ctx.say(p);
            }
        }
    }

    /// The skate is over: record it and read the results.
    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_all(ctx);
        self.music.stop(&mut ctx.audio, 1500);
        ctx.audio.set_bed(None);
        ctx.audio.play_music(None, false);
        if let Some(ch) = self.challenge.take() {
            let pages = challenge::finish(ctx, &ch, &self.skate);
            let summary = challenge::summary(&ch, &self.skate);
            let mut results = Some(after_challenge(ch.period, summary, pages.clone()));
            ctx.replace(Pages::new(pages).then(move |ctx| {
                if let Some(results) = results.take() {
                    ctx.replace(results);
                }
            }));
            return;
        }
        let s = &self.skate;
        let (pages, results) = match self.mode {
            Mode::Pond(n) => {
                let saved = progress::save_pond(ctx, n, s);
                let def = pond(n);
                let summary = format!(
                    "{} over. {}, {}.",
                    def.subject(),
                    sentence_start(&points_words(s.score)),
                    stars_words(saved.stars)
                );
                let mut first = format!(
                    "{}. {} {}.",
                    def.title(),
                    sentence_start(&score_words(saved.score, s.score)),
                    sentence_start(&stars_words(saved.stars))
                );
                if saved.better.is_some() && !matches!(saved.score, ScoreResult::First) {
                    first.push_str(" More stars than ever on this pond.");
                }
                if let Some(open) = saved.opened {
                    first.push_str(&format!(" {} is open.", pond(open).subject()));
                }
                if saved.kept {
                    first.push_str(&format!(" {}", crate::KEPT_LINE));
                }
                let mut pages = vec![first, tally_words(s)];
                if saved.stars == 0 {
                    pages.push(format!(
                        "{} earns its first star at {}. A star opens the next pond.",
                        def.subject(),
                        points_words(def.marks[0])
                    ));
                }
                (
                    pages.clone(),
                    after_pond(n, summary, pages, saved.next_open),
                )
            }
            Mode::Fair => {
                let saved = progress::save_fair(ctx, s);
                let summary = format!(
                    "The Frost Fair. {}, {}.",
                    sentence_start(&points_words(s.score)),
                    stars_words(saved.stars)
                );
                let mut first = format!(
                    "The Frost Fair. {} {}.",
                    sentence_start(&score_words(saved.score, s.score)),
                    sentence_start(&stars_words(saved.stars))
                );
                if saved.ribbon {
                    first.push_str(" A Frost Fair ribbon for this winter.");
                } else if saved.on_the_day && saved.stars == 0 {
                    first.push_str(&format!(
                        " A star earns this winter's ribbon, at {}.",
                        points_words(ponds::FAIR_MARKS[0])
                    ));
                }
                if saved.on_the_day {
                    first.push_str(" The fair stays on the river the rest of this winter.");
                }
                let pages = vec![first, tally_words(s)];
                (pages.clone(), after_mode(Mode::Fair, summary, pages))
            }
            Mode::Lanterns(day) => {
                let saved = progress::save_lanterns(ctx, s);
                let taken = s.tally.taken;
                let summary = format!(
                    "Lantern skate. {} of {} in order.",
                    taken,
                    ponds::LANTERN_PAIRS
                );
                let mut first = summary.clone();
                if saved.whole {
                    first.push_str(" The whole tune.");
                }
                match saved.before {
                    Some(b) if taken > b => {
                        first.push_str(&format!(" A new best tonight, beating {b}."))
                    }
                    Some(b) => first.push_str(&format!(" Your best tonight is {b}.")),
                    None => {}
                }
                first.push_str(" New lanterns tomorrow evening.");
                let pages = vec![first, tally_words(s)];
                (
                    pages.clone(),
                    after_mode(Mode::Lanterns(day), summary, pages),
                )
            }
            Mode::Midnight => {
                let result = Records::record(ctx, ID, MIDNIGHT_BOARD, s.score as u64);
                progress::save_challenge(ctx, s);
                let summary = format!(
                    "Midnight skate. {}.",
                    sentence_start(&points_words(s.score))
                );
                let mut first = format!(
                    "Midnight skate. {} Happy New Year.",
                    sentence_start(&score_words(result, s.score))
                );
                if Records::best(ctx, "liberty-lights", "total").is_some() {
                    first.push_str(
                        " The fireworks were the Liberty Lights crew, the same as on the Fourth of July.",
                    );
                } else {
                    first.push_str(" The fireworks were the Liberty Lights crew.");
                }
                let pages = vec![first, tally_words(s)];
                (pages.clone(), after_mode(Mode::Midnight, summary, pages))
            }
            Mode::Practice => {
                let taken = s.tally.taken;
                progress::save_practice(ctx, taken);
                let summary = format!(
                    "Practice done. {} of {} lanterns taken.",
                    taken,
                    s.course.to_take()
                );
                let pages = vec![summary.clone(), tally_words(s)];
                (pages.clone(), after_mode(Mode::Practice, summary, pages))
            }
            Mode::Challenge(_) => return,
        };
        let mut results = Some(results);
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(results);
            }
        }));
    }

    fn pause(&mut self, ctx: &mut Ctx<Parlor>) {
        self.held.clear();
        let mut keep: Vec<Sound> = self.flames.iter().map(|(_, s)| *s).collect();
        keep.extend(self.patches.iter().map(|(_, s)| *s));
        keep.extend(self.spin);
        if let Some((a, b)) = self.blades {
            keep.push(a);
            keep.push(b);
        }
        keep.extend(self.partner);
        keep.extend(self.stalls.iter().map(|(_, s)| *s));
        for s in keep {
            ctx.audio.pause(s);
            self.held.push(s);
        }
        self.music.pause(&mut ctx.audio);
        self.paused = true;
        let leave = if self.challenge.is_some() {
            Leave::Challenge(CHALLENGES_NAME)
        } else {
            Leave::Title(TITLE_NAME)
        };
        menus::pause(ctx, HELP, leave);
    }

    fn primary_down(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.skate.started() {
            self.start(ctx);
            return;
        }
        if self.skate.stumbling() {
            ctx.say("Finding your feet.");
            return;
        }
        self.skate.spin_down();
        if self.skate.spinning() && self.spin.is_none() {
            self.spin = Some(ctx.audio.start_loop(sound::SPIN_WIND, None, Bus::EFFECTS));
        }
    }
}

/// The lean autoplay and the tests use: towards the first lit lantern in
/// reach, round any patch in the way, else after the partner. Spins are
/// left to the caller.
pub fn auto_target(s: &Skate) -> i32 {
    let now = s.now();
    let mut want = if s.course.level.partner {
        s.course.partner_x(now + 300)
    } else {
        s.x
    };
    if let Some(i) = s.next_lantern() {
        let l = s.course.lanterns[i];
        if !s.course.level.partner || l.at - now < 900 {
            want = l.x();
        }
    }
    if let Some(i) = s.patch_ahead() {
        let p = s.course.patches[i];
        if (want - p.x).abs() < p.width + 4.0 && p.at - now < 1500 {
            want = if p.x > 0.0 {
                p.x - p.width - 8.0
            } else {
                p.x + p.width + 8.0
            };
        }
    }
    if want < s.x - 3.0 {
        -1
    } else if want > s.x + 3.0 {
        1
    } else {
        0
    }
}

/// "4 minutes", "1 minute", "less than a minute".
pub fn minutes_words(secs: i64) -> String {
    match (secs + 59) / 60 {
        i64::MIN..=1 => "a minute".to_string(),
        n => format!("{n} minutes"),
    }
}

/// Keys a player presses for their screen reader or as a modifier.
fn is_quiet_key(key: Key) -> bool {
    matches!(
        key,
        Key::LCtrl
            | Key::RCtrl
            | Key::LShift
            | Key::RShift
            | Key::LAlt
            | Key::RAlt
            | Key::Insert
            | Key::Other(_)
    )
}

fn score_words(result: ScoreResult, score: u32) -> String {
    match result {
        ScoreResult::First => format!("{}, your first score here.", points_words(score)),
        ScoreResult::NewBest { previous } => {
            format!("{}, a new best, beating {previous}.", points_words(score))
        }
        ScoreResult::Short { best } => format!("{}. Your best is {best}.", points_words(score)),
    }
}

/// What the skate came to, in a page.
pub fn tally_words(s: &Skate) -> String {
    let t = &s.tally;
    let mut parts = vec![format!(
        "Lanterns taken: {} of {}. Longest line: {}.",
        t.taken,
        s.passed_count(),
        t.best_line
    )];
    if t.wrong > 0 {
        parts.push(format!("Taken out of order: {}.", t.wrong));
    }
    if t.spins > 0 || t.wobbles > 0 {
        parts.push(format!(
            "Spins: {} landed for {}, {} wobbled.",
            t.spins,
            points_words(t.spin_points),
            t.wobbles
        ));
    }
    if s.course.has_drifts() {
        parts.push(format!("Drifts: {}.", t.drifts));
    }
    if s.course.has_ice() {
        parts.push(format!("Cracks: {}.", t.cracks));
    }
    parts.join(" ")
}

/// The results menu after a pond.
pub fn after_pond(n: u32, summary: String, pages: Vec<String>, next: Option<u32>) -> Results {
    let mut results = Results::new(summary, pages, Back::Title(TITLE_NAME));
    if let Some(next) = next.filter(|&x| x != n) {
        let def = pond(next);
        results = results.next(
            format!("Next pond: {}", def.title()),
            def.scene,
            move |ctx| {
                let s = SkateScreen::open(ctx, Mode::Pond(next));
                ctx.replace(s)
            },
        );
    }
    results.again(
        labels::PLAY_AGAIN,
        "The same pond, with new lanterns.",
        move |ctx| {
            let s = SkateScreen::open(ctx, Mode::Pond(n));
            ctx.replace(s)
        },
    )
}

/// The results menu after a skate with no next pond.
pub fn after_mode(mode: Mode, summary: String, pages: Vec<String>) -> Results {
    let (label, help) = match mode {
        Mode::Practice => (
            "Practise again",
            "The village pond again, everything named.",
        ),
        Mode::Lanterns(_) => (
            "Skate again",
            "Tonight's lanterns again, in the same order.",
        ),
        _ => ("Skate again", "The same skate, with new lanterns."),
    };
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(label, help, move |ctx| {
        let s = SkateScreen::open(ctx, mode);
        ctx.replace(s)
    })
}

pub fn after_challenge(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same lanterns, straight away. Only your best result is kept.",
        move |ctx| {
            let ch = Challenge::current(ctx, ID, period);
            let s = SkateScreen::challenge(ctx, ch);
            ctx.replace(s)
        },
    )
}

impl State<Parlor> for SkateScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        PLAY_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        ctx.audio.set_bed(None);
        ctx.audio.play_music(None, false);
        let mood = self.mood();
        self.music.set_mood(mood);
        self.music.start(&mut ctx.audio);
        self.start_air(ctx);
        let intro = self.intro(ctx);
        ctx.say(intro);
        if !Progress::load(ctx).any() || self.practice() {
            let c = self.controls(ctx);
            ctx.say_queued(c);
        } else {
            ctx.say_queued("F1 lists the controls.");
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_all(ctx);
        self.music.stop(&mut ctx.audio, 600);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.ending.is_none() && self.skate.ended().is_some() {
            return;
        }
        self.call_outs = self.practice() || crate::call_outs(ctx);
        ctx.audio.play_music(None, false);
        self.music.resume(&mut ctx.audio);
        if !self.showing {
            self.music.start(&mut ctx.audio);
        }
        self.start_air(ctx);
        if self.paused {
            self.paused = false;
            for s in self.held.drain(..) {
                ctx.audio.resume(s);
            }
        }
        if !self.skate.started() {
            let p = self.start_prompt(ctx);
            ctx.say(format!("Back on the ice. {p}"));
        } else {
            let line = format!("Back on the ice. You, {}.", self.you_words());
            ctx.say(line);
        }
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.ending.is_some()
            || self.showing
            || self.skate.ended().is_some()
            || event.is_repeat()
        {
            return;
        }
        if !matches!(event, InputEvent::KeyDown { .. }) && event.pressed().is_none() {
            return;
        }
        let action = ctx.action_for(event);
        if action.is_none() && event.pressed() == Some(Key::Escape) {
            self.pause(ctx);
            return;
        }
        match action {
            Some(ParlorAction::Primary) => self.primary_down(ctx),
            Some(ParlorAction::Status) => {
                let layers = self.status_layers(ctx);
                let line = self.status.speak(ctx, layers);
                self.last = Some(line);
            }
            Some(ParlorAction::Repeat) => self.repeat(ctx),
            Some(ParlorAction::Hint) => self.hint(ctx),
            Some(a @ (ParlorAction::Left | ParlorAction::Right)) if !self.skate.started() => {
                self.preview(ctx, a)
            }
            // Left and Right are read as held, in update.
            Some(ParlorAction::Left | ParlorAction::Right) => {}
            Some(ParlorAction::Up | ParlorAction::Down) if self.skate.started() => {
                ctx.play_ui(UiSound::Error);
                let line = format!(
                    "{} and {} lean. You glide forward on your own.",
                    sentence_start(&Self::key(ctx, ParlorAction::Left)),
                    Self::key(ctx, ParlorAction::Right)
                );
                ctx.say(line);
            }
            Some(_) => {
                ctx.play_ui(UiSound::Error);
                let line = format!(
                    "{} and {} lean. {} pushes off, and held, spins.",
                    sentence_start(&Self::key(ctx, ParlorAction::Left)),
                    Self::key(ctx, ParlorAction::Right),
                    sentence_start(&Self::key(ctx, ParlorAction::Primary))
                );
                ctx.say(line);
            }
            None => {
                if let InputEvent::KeyDown { key, .. } = event {
                    if !is_quiet_key(*key) {
                        ctx.play_ui(UiSound::Error);
                        ctx.say("Not a skating key. F1 lists the controls.");
                    }
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        self.music.update(&mut ctx.audio, dt);
        if self.showing {
            self.play_show(ctx, dt);
        }
        if let Some(left) = self.ending.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                self.ending = None;
                self.finish(ctx);
            }
            return;
        }
        if self.showing {
            return;
        }
        self.play_due(ctx, dt);
        let held = ctx.action_held(ParlorAction::Primary);
        if self.start_held && !held {
            self.start_held = false;
        }
        if self.skate.spinning() && !held && self.auto.is_none() {
            let evs = self.skate.spin_up();
            for ev in evs {
                self.on_skate(ctx, ev);
            }
        }
        let lean = match self.auto_lean(ctx, dt) {
            Some(l) => {
                // Autoplay spins once in each long gap.
                if self.skate.started() && !self.skate.spinning() && auto_spin(&self.skate) {
                    self.skate.spin_down();
                }
                if self.skate.spin_ready() {
                    for ev in self.skate.spin_up() {
                        self.on_skate(ctx, ev);
                    }
                }
                l
            }
            None => {
                let l = ctx.action_held(ParlorAction::Left);
                let r = ctx.action_held(ParlorAction::Right);
                match (l, r) {
                    (true, false) => -1,
                    (false, true) => 1,
                    _ => 0,
                }
            }
        };
        if lean != 0 && lean != self.last_lean && self.skate.started() && !self.skate.spinning() {
            sound::play_at(ctx, sound::CARVE, sound::UNDER);
        }
        self.last_lean = lean;
        self.line_before = self.skate.line;
        let evs = self.skate.update(dt, lean);
        for ev in evs {
            self.on_skate(ctx, ev);
            if self.ending.is_some() {
                break;
            }
        }
        if self.skate.started() && self.skate.ended().is_none() {
            let bank = self.skate.x.abs() >= BANK - 0.5;
            if bank && !self.at_bank {
                ctx.say("The bank.");
            }
            self.at_bank = bank;
            if let Some(at) = self.show_at {
                if self.skate.now() >= at {
                    self.start_show(ctx);
                    return;
                }
            }
        }
        self.mix(ctx);
        self.music
            .set_lead(&mut ctx.audio, music::lead_level(self.skate.line));
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let s = &self.skate;
        let mut out = vec![self.mode.name()];
        out.push(format!("You: {}", self.you_words()));
        if !self.practice() {
            out.push(format!("Score: {}", s.score));
        }
        out.push(format!("Line: {}", s.line));
        if s.snows() {
            out.push(sentence_start(s.cover().words()));
        }
        if let Some(i) = s.next_lantern() {
            out.push(format!("Lantern {}", lane_words(s.course.lanterns[i].lane)));
        }
        if let Some(i) = s.patch_ahead() {
            let p = s.course.patches[i];
            out.push(format!(
                "{} {}",
                p.kind.words(),
                lane_words(nearest_lane(p.x))
            ));
        }
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let now = if !self.skate.started() {
            self.start_prompt(ctx)
        } else {
            "Each lantern in a line is worth 1 more point, up to 5 points.".to_string()
        };
        let mut text = format!("{now} {}", self.controls(ctx));
        if self.skate.snows() {
            text.push_str(
                " Snow settles as you skate, and the night ends when it covers the ice. Each lantern taken keeps it clear a little longer.",
            );
        }
        if let Some(ch) = &self.challenge {
            text.push_str(&format!(
                " This is the {} challenge, {}. Leaving from the pause menu does not count the skate.",
                ch.period.label().to_lowercase(),
                challenge::name(ch.period)
            ));
        }
        text
    }

    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

/// Whether autoplay spins now: once a gap, well clear of the next lantern.
fn auto_spin(s: &Skate) -> bool {
    let now = s.now();
    let clear = s
        .next_lantern()
        .map(|i| s.course.lanterns[i].at - now > 2600)
        .unwrap_or(true);
    let calm = s.patch_ahead().is_none();
    clear && calm && now % 7000 < 40
}

mod visuals_ice;
