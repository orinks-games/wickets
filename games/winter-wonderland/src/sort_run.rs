//! Gift Sort's screen: the briefing, the belt, and where a run goes when
//! it ends.

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Sound, State};
use parlor_core::score::{DANGER, HURRY};
use parlor_core::{Mood, Pages, Parlor, ParlorAction, Rng, Score};

use crate::challenge::{self, Part};
use crate::gifts::{SendRefusal, Shift, ShiftState, SortEvent, Spot, SHIFTS};
use crate::mode::{story_bridge, Mode};
use crate::progress::{self, Progress};
use crate::sound::{self, cue, cue_at, stinger};
use crate::{fresh_rng, pause, results};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// The wishes are read; the belt waits for the player.
    Briefing,
    Playing,
}

/// What the workshop's score hears: it builds as the shift's quota fills,
/// the streak grows and the shifts go on. The shift's last ten seconds
/// hurry it, and a gift near the edge darkens it.
pub fn mood(shift: &Shift) -> Mood {
    if shift.state != ShiftState::Running {
        return Mood::new(0.0);
    }
    let quota = shift.sorted as f32 / shift.spec.quota.max(1) as f32;
    let streak = shift.streak.min(5) as f32 / 5.0;
    let night = (shift.spec.level - 1) as f32 / (SHIFTS - 1) as f32;
    Mood::new(0.2 + 0.45 * quota + 0.2 * streak + 0.15 * night)
        .with_flag(HURRY, shift.clock <= 10.0)
        .with_flag(DANGER, shift.gift.as_ref().is_some_and(|g| g.warned))
}

/// One Gift Sort run: shift after shift until the last or the clock wins.
pub struct SortRun {
    mode: Mode,
    /// The shift the run started from, for Play again.
    first: u32,
    level: u32,
    last: u32,
    shift: Shift,
    /// Points from shifts already cleared.
    banked: u64,
    phase: Phase,
    belt: Sound,
    rng: Rng,
    told_arrive: bool,
    told_warn: bool,
    /// The shift's first low-clock warning has been spoken.
    told_low: bool,
    bonus: u64,
    sorted: u32,
    broken: u32,
    wrongs: u32,
    /// "It sounds like a rattle." waits for the shake to finish.
    pending: Option<(f64, String)>,
    /// The workshop's music, while the belt runs.
    pub score: Score,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
    /// Run inside the tutorial: no "to go" count, since the lesson moves
    /// on before the quota, and the step prompts say when a gift lands.
    pub lesson: bool,
    /// Seconds the belt waits for a lesson prompt to be heard: the belt
    /// does not start, and a gift on it does not move.
    hold: f64,
}

/// "1 second", "5 seconds" or "5.5 seconds".
fn secs(s: f64) -> String {
    if (s - s.round()).abs() < 0.05 {
        let n = s.round();
        let unit = if n == 1.0 { "second" } else { "seconds" };
        format!("{n:.0} {unit}")
    } else {
        format!("{s:.1} seconds")
    }
}

impl SortRun {
    /// A run in `mode`; `start` is the first shift for a free run.
    pub fn new(mode: Mode, start: u32) -> SortRun {
        let (first, last) = mode.shifts(start.clamp(1, SHIFTS));
        let mut rng = match mode.plan() {
            Some(plan) => Rng::seeded(plan.run_seed),
            None => fresh_rng(),
        };
        let shift = Self::make_shift(&mode, first, Rng::seeded(rng.next_u64()));
        SortRun {
            status_key: Default::default(),
            mode,
            first,
            level: first,
            last,
            shift,
            banked: 0,
            phase: Phase::Briefing,
            belt: Sound::NONE,
            rng,
            told_arrive: false,
            told_warn: false,
            told_low: false,
            bonus: 0,
            sorted: 0,
            broken: 0,
            wrongs: 0,
            pending: None,
            score: Score::new(),
            lesson: false,
            hold: 0.0,
        }
    }

    /// Holds the belt for `secs`, while a lesson prompt is said.
    pub fn hold_for(&mut self, secs: f64) {
        self.hold = secs;
        if self.lesson {
            self.told_arrive = true;
            // The gift on the belt rides its whole way after the prompt.
            if let Some(g) = self.shift.gift.as_mut() {
                g.ridden = 0.0;
                g.warned = false;
            }
        }
    }

    /// Shift `level`, with the challenge's twist when there is one.
    fn make_shift(mode: &Mode, level: u32, rng: Rng) -> Shift {
        match mode.plan() {
            Some(plan) => plan.shift(level, rng),
            None => Shift::new(level, rng),
        }
    }

    /// "Shift 3".
    fn shift_name(&self) -> String {
        format!("Shift {}", self.shift.spec.level)
    }

    /// The shift on the belt, for the tutorial and tests.
    pub fn shift(&self) -> &Shift {
        &self.shift
    }

    /// Whether the belt is running, the briefing over.
    pub fn playing(&self) -> bool {
        self.phase == Phase::Playing
    }

    /// The run's score so far.
    pub fn score(&self) -> u64 {
        self.banked + self.shift.score
    }

    fn name(ctx: &Ctx<Parlor>, action: ParlorAction) -> String {
        ctx.control_name(action)
    }

    /// One recipient's wish, as a sentence.
    fn wish_line(&self, i: usize) -> String {
        let r = &self.shift.recipients[i];
        let which = if self.shift.spec.say_sounds {
            format!(", a {} in the box", r.wish.name())
        } else {
            String::new()
        };
        format!(
            "{}, {}, wishes for {}{which}.",
            r.name,
            r.spot.place(),
            r.item
        )
    }

    /// Every wish on this shift, as R says them.
    pub fn wishes(&self, ctx: &Ctx<Parlor>) -> String {
        let mut parts: Vec<String> = (0..self.shift.recipients.len())
            .map(|i| self.wish_line(i))
            .collect();
        if self.shift.spec.sack {
            parts.push(format!(
                "Gifts nobody wished for go in the return sack behind you, with {}.",
                Self::name(ctx, ParlorAction::Down)
            ));
        }
        parts.join(" ")
    }

    fn briefing(&self, ctx: &Ctx<Parlor>) -> String {
        let spec = &self.shift.spec;
        let primary = Self::name(ctx, ParlorAction::Primary);
        let of = if self.last > self.level || self.mode.is_story() {
            format!(" of {}", self.last)
        } else {
            String::new()
        };
        let mut text = format!(
            "Shift {}{of}. Sort {} gifts in {}. {}",
            spec.level,
            spec.quota,
            secs(spec.time.round()),
            self.wishes(ctx)
        );
        if spec.level == 1 {
            text.push_str(&format!(
                " Shake each gift with {primary} and listen. Then send it with {} or {} to the person who wished for it, before it rides off the end of the belt. A wrong guess costs time, but the gift stays on the belt.",
                Self::name(ctx, ParlorAction::Left),
                Self::name(ctx, ParlorAction::Right)
            ));
        } else if spec.level == 2 {
            text.push_str(&format!(
                " {} sends a gift straight ahead.",
                sentence_start(&Self::name(ctx, ParlorAction::Up))
            ));
        }
        if spec.level == 4 {
            text.push_str(" From now on, the wishes do not say what they sound like. Think about what each one would sound like in a box.");
        }
        if spec.mind_change {
            text.push_str(" Listen out: someone may change their mind half way through.");
        }
        if let Some(plan) = self.mode.plan() {
            if plan.twists_gifts() {
                text.push(' ');
                text.push_str(&plan.twist_line());
            }
        }
        text.push_str(&format!(" {} starts the belt.", sentence_start(&primary)));
        text
    }

    /// The status key, in layers. First: the gift on the belt and the
    /// clock. A second press: the shift, gifts sorted and the score.
    fn status(&self, ctx: &Ctx<Parlor>) -> Vec<String> {
        let s = &self.shift;
        let gift = match (&s.gift, self.phase) {
            (_, Phase::Briefing) => "The belt is waiting for you.".to_string(),
            (None, _) => "Belt empty. A gift is coming.".to_string(),
            (Some(g), _) => {
                let shaken = if g.shakes == 0 {
                    "not shaken yet"
                } else {
                    "shaken"
                };
                format!(
                    "A gift is on the belt, {shaken}, {} from the edge.",
                    secs(g.left().ceil())
                )
            }
        };
        let _ = ctx;
        vec![
            format!("{gift} {} on the clock.", secs(s.clock.ceil())),
            format!(
                "{}. {} of {} sorted. Score {}.",
                self.shift_name(),
                s.sorted,
                s.spec.quota,
                self.score()
            ),
        ]
    }

    fn start_belt(&mut self, ctx: &mut Ctx<Parlor>) {
        self.phase = Phase::Playing;
        self.belt = ctx.audio.start_loop(sound::BELT, None, Bus::AMBIENCE);
        ctx.audio
            .set_pitch(self.belt, 1.0 + 0.05 * (self.level - 1) as f32);
        let mood = mood(&self.shift);
        self.score.start(ctx, &sound::SCORE, mood);
        if !self.lesson {
            // The lesson's next step says the belt is running.
            ctx.say("The belt starts rolling.");
        }
    }

    fn stop_belt(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.stop(self.belt, 300);
        self.belt = Sound::NONE;
        self.score.stop(ctx);
    }

    fn who(&self, spot: Spot) -> &'static str {
        self.shift.at(spot).map(|r| r.name).unwrap_or("Nobody")
    }

    fn voice(&mut self, ctx: &mut Ctx<Parlor>, event: SortEvent) {
        match event {
            SortEvent::Arrived => {
                ctx.audio
                    .play_at(sound::ARRIVE, sound::GIFT_POS, Bus::EFFECTS);
                if !self.told_arrive {
                    self.told_arrive = true;
                    let primary = Self::name(ctx, ParlorAction::Primary);
                    ctx.say_queued(format!("A gift on the belt. Shake it with {primary}."));
                }
            }
            SortEvent::Warn => {
                cue(ctx, sound::WARN);
                if !self.told_warn {
                    self.told_warn = true;
                    ctx.say("Beeping: near the edge.");
                }
            }
            SortEvent::Fell { penalty, .. } => {
                self.pending = None;
                cue(ctx, sound::FALL);
                ctx.say(format!("Crash. Minus {}.", secs(penalty)));
            }
            SortEvent::Correct { spot, points, .. } => {
                self.pending = None;
                let to_go = self.shift.spec.quota.saturating_sub(self.shift.sorted);
                let rest = if to_go > 0 && !self.lesson {
                    format!(" {to_go} to go.")
                } else {
                    String::new()
                };
                if spot == Spot::Sack {
                    cue_at(ctx, sound::SACK, spot);
                    ctx.say(format!("Into the return sack. Plus {points}.{rest}"));
                } else {
                    cue_at(ctx, sound::SEND, spot);
                    cue_at(ctx, sound::CORRECT, spot);
                    ctx.say(format!(
                        "{} is delighted. Plus {points}.{rest}",
                        self.who(spot)
                    ));
                }
            }
            SortEvent::Wrong { spot, penalty } => {
                cue_at(ctx, sound::WRONG, spot);
                let who = if spot == Spot::Sack {
                    "Someone wished for that.".to_string()
                } else {
                    format!("{} didn't wish for that.", self.who(spot))
                };
                ctx.say(format!("{who} Minus {}.", secs(penalty)));
            }
            SortEvent::ClockLow(n) => {
                cue(ctx, sound::CLOCK);
                if !self.told_low {
                    self.told_low = true;
                    let unit = if n == 1 { "second" } else { "seconds" };
                    ctx.say_queued(format!("{n} {unit} left!"));
                }
            }
            SortEvent::MindChanged { who } => {
                cue(ctx, sound::CHANGE);
                let r = &self.shift.recipients[who];
                let which = if self.shift.spec.say_sounds {
                    format!(", a {} in the box", r.wish.name())
                } else {
                    String::new()
                };
                ctx.say(format!(
                    "Change of plans. {}, {}, now wishes for {}{which}.",
                    r.name,
                    r.spot.place(),
                    r.item
                ));
            }
            SortEvent::Cleared { bonus } => self.bonus = bonus,
            SortEvent::TimeUp => {}
        }
    }

    fn after(&mut self, ctx: &mut Ctx<Parlor>, events: Vec<SortEvent>) {
        for e in events {
            self.voice(ctx, e);
        }
        match self.shift.state {
            ShiftState::Running => {}
            ShiftState::Cleared => self.cleared(ctx),
            ShiftState::Failed => self.finish(ctx, false),
        }
    }

    fn tally(&mut self) {
        self.sorted += self.shift.sorted;
        self.broken += self.shift.broken;
        self.wrongs += self.shift.wrongs;
    }

    fn cleared(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_belt(ctx);
        self.pending = None;
        cue(ctx, sound::CLEAR);
        self.banked += self.shift.score;
        self.tally();
        if self.mode.plan().is_none() {
            Progress::open_shift(ctx, self.level + 1);
        }
        if self.level >= self.last {
            self.finish(ctx, true);
            return;
        }
        let done = self.level;
        self.level += 1;
        self.shift = Self::make_shift(&self.mode, self.level, Rng::seeded(self.rng.next_u64()));
        self.phase = Phase::Briefing;
        self.told_low = false;
        ctx.say_queued(format!(
            "Shift {done} complete. {} bonus points for the time left. Score {}.",
            self.bonus,
            self.score()
        ));
        let brief = self.briefing(ctx);
        ctx.say_queued(brief);
    }

    fn summary(&self) -> String {
        let gifts = if self.sorted == 1 { "gift" } else { "gifts" };
        let guesses = if self.wrongs == 1 { "guess" } else { "guesses" };
        format!(
            "{} {gifts} delivered, {} broken, {} wrong {guesses}.",
            self.sorted, self.broken, self.wrongs
        )
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>, success: bool) {
        self.stop_belt(ctx);
        self.pending = None;
        if !success {
            self.banked += self.shift.score;
            self.tally();
        }
        let score = self.banked;
        let level = self.level;
        let head = if success {
            stinger(ctx, sound::WIN);
            String::new()
        } else {
            stinger(ctx, sound::LOSE);
            format!("Time's up on shift {level}. ")
        };
        match self.mode.clone() {
            Mode::Story { blizzard, .. } if success => {
                let mut pages = vec![format!(
                    "The last shift is sorted. {} Gift Sort score {score}.",
                    self.summary()
                )];
                pages.extend(story_bridge(blizzard));
                ctx.replace(crate::desk_run::story_desk(blizzard, score, pages));
            }
            Mode::Story { blizzard, .. } => {
                let board = if blizzard {
                    progress::BLIZZARD_BOARD
                } else {
                    progress::STORY_BOARD
                };
                let line = progress::record(ctx, board, score);
                let pages = vec![format!(
                    "{head}The story ends on the workshop floor this year. {} Story score {score}. {line}",
                    self.summary()
                )];
                let summary = format!("Time's up on shift {level}. Story score {score}");
                results::show(
                    ctx,
                    pages.clone(),
                    results::story(blizzard, false, summary, pages),
                );
            }
            Mode::Challenge { plan, .. } if success && plan.part == Part::Double => {
                let pages = vec![
                    format!(
                        "The last shift is sorted. {} Gift Sort score {score}.",
                        self.summary()
                    ),
                    "Then a stack of letters at the Wish Desk. Your Gift Sort score comes with you.".to_string(),
                ];
                ctx.replace(Pages::new(pages).then(move |ctx| {
                    let mode = Mode::Challenge {
                        plan: plan.clone(),
                        gifts: score,
                    };
                    ctx.replace(crate::desk_run::DeskRun::new(mode, plan.stacks().0));
                }));
            }
            Mode::Challenge { plan, .. } => {
                let cleared = match (success, plan.part) {
                    (true, _) => "Challenge cleared. ",
                    (false, Part::Double) => "The holiday double ends on the workshop floor. ",
                    _ => "",
                };
                let line = challenge::result_line(ctx, &plan, score);
                let pages = vec![format!("{head}{cleared}{} {line}", self.summary())];
                let summary = if success {
                    format!("Challenge cleared. Score {score}")
                } else {
                    format!("Time's up on shift {level}. Score {score}")
                };
                results::show(
                    ctx,
                    pages.clone(),
                    results::challenge(&plan, summary, pages),
                );
            }
            _ => {
                let line = progress::record(ctx, progress::GIFT_BOARD, score);
                let mut text = if success {
                    Progress::mark(ctx, |p| &mut p.all_gifts);
                    "Every shift is sorted. ".to_string()
                } else {
                    head
                };
                text.push_str(&format!("{} Final score {score}. {line}", self.summary()));
                let open = Progress::load(ctx).shifts_open();
                if !success && open > 1 {
                    text.push_str(&format!(
                        " You can start from any shift up to {open} on the title."
                    ));
                }
                let summary = if success {
                    format!("Every shift sorted. Score {score}")
                } else {
                    format!("Time's up on shift {level}. Score {score}")
                };
                let pages = vec![text];
                results::show(
                    ctx,
                    pages.clone(),
                    results::gift_sort(self.first, success, summary, pages),
                );
            }
        }
    }

    fn shake(&mut self, ctx: &mut Ctx<Parlor>) {
        match self.shift.shake() {
            Some(kind) => {
                ctx.audio
                    .play_at(sound::gift(kind), sound::GIFT_POS, Bus::EFFECTS);
                if self.shift.spec.level <= 2 {
                    self.pending = Some((1.1, format!("It sounds like a {}.", kind.name())));
                }
            }
            None => {
                cue(ctx, sound::EMPTY);
                ctx.say("Belt empty. A gift is coming.");
            }
        }
    }

    fn send(&mut self, ctx: &mut Ctx<Parlor>, spot: Spot) {
        match self.shift.send(spot) {
            Ok(events) => self.after(ctx, events),
            Err(SendRefusal::NoGift) => {
                cue(ctx, sound::EMPTY);
                ctx.say("Nothing on the belt to send yet.");
            }
            Err(SendRefusal::NoOne) => {
                cue(ctx, sound::EMPTY);
                if spot == Spot::Sack {
                    ctx.say("No return sack this shift.");
                } else {
                    ctx.say(format!("Nobody is standing {}.", spot.place()));
                }
            }
        }
    }

    fn hint(&mut self, ctx: &mut Ctx<Parlor>) {
        let Some(gift) = self.shift.gift.clone() else {
            ctx.say("No gift on the belt yet.");
            return;
        };
        if gift.shakes == 0 {
            let primary = Self::name(ctx, ParlorAction::Primary);
            ctx.say(format!("Shake it first with {primary}."));
            return;
        }
        self.shift.take_hint();
        cue(ctx, sound::HINT);
        let home = self.shift.home_of(gift.kind);
        let text = match self.shift.at(home) {
            Some(r) => format!(
                "That was a {}, {} for {}: send it {} with {}.",
                gift.kind.name(),
                r.item,
                r.name,
                r.spot.place(),
                Self::spot_key(ctx, home)
            ),
            None => format!(
                "That was a {}. Nobody wished for it: return sack, with {}.",
                gift.kind.name(),
                Self::spot_key(ctx, Spot::Sack)
            ),
        };
        ctx.say(format!("{text} Half points for this gift."));
    }

    fn spot_key(ctx: &Ctx<Parlor>, spot: Spot) -> String {
        Self::name(
            ctx,
            match spot {
                Spot::Left => ParlorAction::Left,
                Spot::Ahead => ParlorAction::Up,
                Spot::Right => ParlorAction::Right,
                Spot::Sack => ParlorAction::Down,
            },
        )
    }

    fn reminder(&self, ctx: &Ctx<Parlor>) -> String {
        match self.phase {
            Phase::Briefing => format!(
                "{} starts the {}, {} repeats the wishes.",
                sentence_start(&Self::name(ctx, ParlorAction::Primary)),
                "belt",
                Self::name(ctx, ParlorAction::Repeat)
            ),
            Phase::Playing => {
                let ahead = if self.shift.spec.recipients == 2 {
                    String::new()
                } else {
                    format!(", {}", Self::name(ctx, ParlorAction::Up))
                };
                format!(
                    "{} shakes the gift. {}{ahead} and {} send it. F1 explains the keys.",
                    sentence_start(&Self::name(ctx, ParlorAction::Primary)),
                    sentence_start(&Self::name(ctx, ParlorAction::Left)),
                    Self::name(ctx, ParlorAction::Right)
                )
            }
        }
    }
}

impl State<Parlor> for SortRun {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        "gift_sort"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        ctx.audio.set_bed(Some(sound::FIRE));
        let brief = self.briefing(ctx);
        ctx.say(brief);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_belt(ctx);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        match self.phase {
            Phase::Briefing => {
                let brief = self.briefing(ctx);
                ctx.say(brief);
            }
            Phase::Playing => {
                let status = self.status(ctx).remove(0);
                ctx.say(format!("Back on the belt. {status}"));
            }
        }
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() {
            return;
        }
        let action = ctx.action_for(event);
        if action.is_none() {
            match event.pressed() {
                Some(hearken::Key::Escape) => {
                    pause(ctx, &self.mode);
                }
                Some(_) => {
                    let r = self.reminder(ctx);
                    ctx.say(r);
                }
                None => {}
            }
            return;
        }
        let Some(action) = action else { return };
        match (self.phase, action) {
            (_, ParlorAction::Status) => {
                let layers = self.status(ctx);
                self.status_key.speak(ctx, layers);
            }
            (Phase::Briefing, ParlorAction::Primary) if self.hold > 0.0 => {
                // The lesson is still saying the wishes: a soft knock, and
                // nothing said over them.
                cue(ctx, sound::EMPTY);
            }
            (Phase::Briefing, ParlorAction::Primary) => self.start_belt(ctx),
            (Phase::Briefing, ParlorAction::Repeat | ParlorAction::Secondary) => {
                let b = self.briefing(ctx);
                ctx.say(b);
            }
            (Phase::Briefing, ParlorAction::Hint) => {
                let primary = Self::name(ctx, ParlorAction::Primary);
                ctx.say(format!(
                    "Remember who wants what, and where they stand. Each wish would make its own sound in a box. Press {primary} when you are ready."
                ));
            }
            (Phase::Briefing, _) => {
                let r = self.reminder(ctx);
                ctx.say(r);
            }
            (Phase::Playing, ParlorAction::Primary) => self.shake(ctx),
            (Phase::Playing, ParlorAction::Left) => self.send(ctx, Spot::Left),
            (Phase::Playing, ParlorAction::Up) => self.send(ctx, Spot::Ahead),
            (Phase::Playing, ParlorAction::Right) => self.send(ctx, Spot::Right),
            (Phase::Playing, ParlorAction::Down) => self.send(ctx, Spot::Sack),
            (Phase::Playing, ParlorAction::Repeat | ParlorAction::Secondary) => {
                let w = self.wishes(ctx);
                ctx.say(w);
            }
            (Phase::Playing, ParlorAction::Hint) => self.hint(ctx),
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.hold > 0.0 {
            self.hold = (self.hold - dt).max(0.0);
            return;
        }
        if self.phase != Phase::Playing {
            return;
        }
        if let Some((left, text)) = self.pending.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                let text = std::mem::take(text);
                self.pending = None;
                ctx.say(text);
            }
        }
        if self.lesson {
            // A lesson shift never runs out of time, so it never ends in
            // results or a saved score.
            self.shift.clock = self.shift.clock.max(self.shift.spec.time);
        }
        let events = self.shift.update(dt);
        let mood = mood(&self.shift);
        self.score.follow(ctx, mood, dt);
        self.after(ctx, events);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let s = &self.shift;
        let mut out = vec![format!(
            "Gift Sort, shift {} ({}), {} of {} sorted, clock {:.1}, score {}",
            s.spec.level,
            if self.phase == Phase::Briefing {
                "briefing"
            } else {
                "playing"
            },
            s.sorted,
            s.spec.quota,
            s.clock,
            self.score()
        )];
        for r in &s.recipients {
            out.push(format!(
                "{}: {} wants {} ({})",
                r.spot.label(),
                r.name,
                r.item,
                r.wish.name()
            ));
        }
        if s.spec.sack {
            out.push("Sack: anything nobody wished for".to_string());
        }
        out.push(match &s.gift {
            None => "Gift: none on the belt".to_string(),
            Some(g) if g.shakes == 0 => {
                format!("Gift: not shaken, {:.1} seconds to the edge", g.left())
            }
            Some(g) => format!(
                "Gift: sounds like {}, {:.1} seconds to the edge",
                g.kind.name(),
                g.left()
            ),
        });
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let n = |a| ctx.control_name(a);
        match self.phase {
            Phase::Briefing => format!(
                "{} briefing. {} starts the {}. {} repeats the wishes, {} speaks the {}, Escape pauses.",
                "Gift Sort",
                sentence_start(&n(ParlorAction::Primary)),
                "belt",
                sentence_start(&n(ParlorAction::Repeat)),
                n(ParlorAction::Status),
                "shift"
            ),
            Phase::Playing => {
                let sack = if self.shift.spec.sack {
                    format!(", and {} drops it in the return sack behind you", n(ParlorAction::Down))
                } else {
                    String::new()
                };
                let ahead = if self.shift.spec.recipients == 2 {
                    String::new()
                } else {
                    format!(", {} straight ahead", n(ParlorAction::Up))
                };
                format!(
                    "Gift Sort. {} shakes the gift. {} sends it left{ahead}, {} right{sack}. {} repeats the wishes, {} speaks the clock and score, {} gives a hint at half points. Escape pauses.",
                    sentence_start(&n(ParlorAction::Primary)),
                    sentence_start(&n(ParlorAction::Left)),
                    n(ParlorAction::Right),
                    sentence_start(&n(ParlorAction::Repeat)),
                    sentence_start(&n(ParlorAction::Status)),
                    sentence_start(&n(ParlorAction::Hint))
                )
            }
        }
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

#[cfg(test)]
mod tests {
    use super::secs;

    #[test]
    fn one_second_is_singular() {
        assert_eq!(secs(1.0), "1 second");
        assert_eq!(secs(0.0), "0 seconds");
        assert_eq!(secs(5.0), "5 seconds");
        assert_eq!(secs(5.5), "5.5 seconds");
        assert_eq!(secs(1.5), "1.5 seconds");
    }
}

mod visuals_sortrun;
