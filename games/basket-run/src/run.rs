//! The meadow itself, and the results after a run.

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{
    Challenge, Challenges, Mood, Pace, Pages, Parlor, ParlorAction, Rng, RunClock, Score,
    ScoreResult,
};

use crate::challenge::{briefing, CHALLENGES_NAME};
use crate::easter::{self, Keepsakes};
use crate::progress::Progress;
use crate::rules::{
    baskets_left, eggs_phrase, lanes_phrase, Bank, Event, Kind, Mode, Move, Pattern, Phase, Run,
    LEAD_IN, ROUNDS, STREAK_CAP, WARNING_AT, WEEKLY_FROM,
};
use crate::sound::{self, drop_pos, ground};
use crate::{HELP, ID};

/// The title's screen name.
pub const TITLE_NAME: &str = "basket_run";
/// The meadow's screen name.
pub const RUN_NAME: &str = "basket_run_run";
/// Seconds between the cart's bell.
pub const BEACON_EVERY: f64 = 3.0;
/// The longest the lead-in waits for the round's announcement.
pub const MAX_LEAD_IN: f64 = 10.0;
/// A drop call or catch count waits at most this long behind an important
/// line; a later one is dropped (its sound, and the status key, still tell).
pub const CHATTER_WAIT: f64 = 0.8;

/// About how long a line takes to say, at a conservative voice.
pub fn speech_secs(text: &str) -> f64 {
    0.4 + text.chars().count() as f64 / 15.0
}

/// How long the lead-in waits so the round's announcement is heard whole
/// before anything falls.
pub fn lead_in_for(text: &str) -> f64 {
    speech_secs(text).clamp(LEAD_IN, MAX_LEAD_IN)
}

/// What starting a run needs: the mode, and a seed for tests and
/// challenges (`None` seeds from the clock). A challenge mode's results
/// and pause go back to the challenges, every other mode's to the title.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Launch {
    pub mode: Mode,
    pub seed: Option<u64>,
}

impl Launch {
    pub fn build(&self) -> Run {
        let rng = self.seed.map_or_else(Rng::from_time, Rng::seeded);
        Run::new(self.mode, rng)
    }
}

/// The controls in a paragraph, naming the player's own keys.
pub fn controls_text(ctx: &Ctx<Parlor>) -> String {
    let left = ctx.control_name(ParlorAction::Left);
    let right = ctx.control_name(ParlorAction::Right);
    let bank = ctx.control_name(ParlorAction::Primary);
    let cart = ctx.control_name(ParlorAction::Secondary);
    let status = ctx.control_name(ParlorAction::Status);
    let repeat = ctx.control_name(ParlorAction::Repeat);
    let hint = ctx.control_name(ParlorAction::Hint);
    format!(
        "{} and {right} step one lane. {} empties your basket into the cart, standing in the cart's lane. {} rings the cart's bell and says where it is. {} speaks your lane, basket and what lands next; press it again for the clock and baskets left, and again for your score. {} repeats the last announcement. {} says what to do right now. Escape pauses.",
        sentence_start(&left),
        sentence_start(&bank),
        sentence_start(&cart),
        sentence_start(&status),
        sentence_start(&repeat),
        sentence_start(&hint),
    )
}

/// The page explaining a kind the first time it falls.
pub fn introduction(kind: Kind) -> String {
    match kind {
        Kind::Egg => {
            "Your first egg. Step into its lane and stay until it lands."
                .to_string()
        }
        Kind::Golden => "A golden egg. Worth five eggs. Catch it.".to_string(),
        Kind::Mud => {
            "A mud clod. It spoils your basket. Step out of its lane.".to_string()
        }
        Kind::Bee => {
            "A bee. Its sting breaks a basket. Step out of its lane.".to_string()
        }
    }
}

/// What is new in a round, said between rounds.
pub fn round_news(mode: Mode, n: u32) -> &'static str {
    match (mode, n) {
        (Mode::Meadow, 2) => "Mud clods start falling, and the cart moves to the right edge.",
        (Mode::Meadow, 3) => "A fourth lane, golden eggs and bees, and the basket holds five.",
        (Mode::Meadow, 4) => "Eggs sometimes come in pairs, one close behind the other.",
        (Mode::Meadow, 5) => "A fifth lane, and bees that dive right behind an egg.",
        (Mode::Meadow, 7) => "Faster, and the basket holds six.",
        (Mode::Meadow, _) => "Faster.",
        (Mode::Endless, _) => "Faster still.",
        (Mode::Weekly, n) => round_news(Mode::Meadow, n + WEEKLY_FROM - 1),
        (Mode::Festival, _) => "The patterns fall faster.",
        (Mode::Easter, 2) => "The orchard: a fourth lane, eggs in pairs, and a little mud.",
        (Mode::Easter, _) => "The big hunt: five lanes, and golden eggs everywhere.",
        _ => "Another round.",
    }
}

/// Opens a run, with the challenge's briefing first for a challenge.
/// `replace` swaps out the screen that asked (results' "Play again").
pub fn start(ctx: &mut Ctx<Parlor>, launch: Launch, replace: bool) {
    if let Some(period) = launch.mode.period() {
        let pages = briefing(ctx, period);
        if replace {
            ctx.replace(pages);
        } else {
            ctx.push(pages);
        }
        return;
    }
    if replace {
        ctx.replace(RunScreen::new(launch, None));
    } else {
        ctx.push(RunScreen::new(launch, None));
    }
}

/// The meadow during a run.
pub struct RunScreen {
    pub run: Run,
    launch: Launch,
    challenge: Option<Challenge>,
    loops: Vec<(u32, Sound)>,
    last: String,
    beacon_in: f64,
    callouts: bool,
    /// Seconds of play, for pacing speech.
    clock: f64,
    /// When the last important line should be done being said.
    busy_until: f64,
    /// The eggs the bell took, said again in the round's summary.
    time_note: Option<String>,
    intro_pending: Vec<Kind>,
    introducing: Option<Kind>,
    frozen: bool,
    summary: Option<String>,
    /// The ending in a few words, for the results' spoken title.
    headline: Option<String>,
    awaiting_next: bool,
    done: bool,
    /// The meadow's music, following the run.
    pub score: Score,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
    /// The meadow's clock: it holds for speech, at the run's pace.
    pub run_clock: RunClock,
}

impl RunScreen {
    pub fn new(launch: Launch, challenge: Option<Challenge>) -> RunScreen {
        RunScreen {
            status_key: Default::default(),
            run: launch.build(),
            launch,
            challenge,
            loops: Vec::new(),
            last: String::new(),
            beacon_in: 0.5,
            callouts: true,
            clock: 0.0,
            busy_until: 0.0,
            time_note: None,
            intro_pending: Vec::new(),
            introducing: None,
            frozen: false,
            summary: None,
            headline: None,
            awaiting_next: false,
            done: false,
            score: Score::new(),
            run_clock: RunClock::default(),
        }
    }

    /// What the score hears: later rounds and a longer streak build it up,
    /// the last ten seconds hurry it, and one basket left darkens it.
    pub fn mood(&self) -> Mood {
        let r = &self.run;
        let rounds = r.mode.rounds().unwrap_or(ROUNDS).max(2);
        let progress = r.round.saturating_sub(1).min(rounds - 1) as f32 / (rounds - 1) as f32;
        let streak = r.streak.min(STREAK_CAP) as f32 / STREAK_CAP as f32;
        let intensity = match r.phase {
            Phase::Playing => 0.2 + 0.45 * progress + 0.4 * streak,
            _ => 0.15,
        };
        Mood::new(intensity)
            .with_flag(
                parlor_core::score::HURRY,
                r.phase == Phase::Playing && r.time_left() <= WARNING_AT,
            )
            .with_flag(
                parlor_core::score::DANGER,
                r.mode != Mode::Practice && r.lives == 1,
            )
    }

    /// An important line: said now, remembered for the repeat key, and
    /// protected from drop calls until it is likely done.
    fn tell(&mut self, ctx: &mut Ctx<Parlor>, text: impl Into<String>) {
        let text = text.into();
        self.busy_until = self.clock + speech_secs(&text);
        ctx.say(text.clone());
        self.last = text;
    }

    /// An important line said after what is being said.
    fn tell_queued(&mut self, ctx: &mut Ctx<Parlor>, text: impl Into<String>) {
        let text = text.into();
        self.busy_until = self.busy_until.max(self.clock) + speech_secs(&text);
        ctx.say_queued(text.clone());
        self.last = text;
    }

    /// A passing line (a drop call, a catch count): it never cuts off an
    /// important line. It waits a moment behind one, and is dropped when it
    /// would come too late to matter.
    fn chatter(&mut self, ctx: &mut Ctx<Parlor>, text: String) {
        let wait = self.busy_until - self.clock;
        if wait <= 0.0 {
            ctx.say(text);
        } else if wait <= CHATTER_WAIT {
            ctx.say_queued(text);
        }
    }

    fn lanes(&self) -> usize {
        self.run.lanes()
    }

    fn announce_round(&mut self, ctx: &mut Ctx<Parlor>) {
        let mood = self.mood();
        self.score.start(ctx, &sound::SCORE, mood);
        sound::listen_from(ctx, self.run.lane, self.lanes());
        self.beacon_in = 0.5;
        let intro = format!(
            "{} {} skips the wait.",
            self.run.round_intro(),
            sentence_start(&ctx.control_name(ParlorAction::Primary))
        );
        // Nothing falls until the announcement has been heard.
        if matches!(self.run.phase, Phase::LeadIn(_)) {
            self.run.phase = Phase::LeadIn(lead_in_for(&intro));
        }
        self.tell(ctx, intro);
    }

    fn cart_text(&self) -> String {
        let cart = self.run.cart_lane();
        if self.run.at_cart() {
            format!("You are at the cart, lane {}.", cart + 1)
        } else {
            format!(
                "The cart is in lane {}, {}.",
                cart + 1,
                self.run.relative(cart)
            )
        }
    }

    fn stop_loops(&mut self, ctx: &mut Ctx<Parlor>) {
        for (_, s) in self.loops.drain(..) {
            ctx.audio.stop(s, 60);
        }
    }

    fn sync_loops(&mut self, ctx: &mut Ctx<Parlor>) {
        let lanes = self.lanes();
        let live: Vec<u32> = self.run.falling.iter().map(|f| f.id).collect();
        let mut kept = Vec::new();
        for (id, s) in self.loops.drain(..) {
            if live.contains(&id) {
                kept.push((id, s));
            } else {
                ctx.audio.stop(s, 40);
            }
        }
        self.loops = kept;
        for f in &self.run.falling {
            let p = f.progress();
            let pos = drop_pos(f.lane, lanes, p);
            let s = match self.loops.iter().find(|(id, _)| *id == f.id) {
                Some((_, s)) => *s,
                None => {
                    let s = ctx
                        .audio
                        .start_loop(sound::falling(f.kind), Some(pos), Bus::EFFECTS);
                    self.loops.push((f.id, s));
                    s
                }
            };
            ctx.audio.set_position(s, Some(pos));
            ctx.audio.set_pitch(s, sound::fall_pitch(p));
            ctx.audio.set_volume(s, sound::fall_volume(p));
        }
    }

    fn on_rule_event(&mut self, ctx: &mut Ctx<Parlor>, event: Event) {
        let lanes = self.lanes();
        match event {
            Event::Go => {
                self.run_clock.reset();
                ctx.audio.play(sound::READY, Bus::EFFECTS);
                ctx.say_queued("Go.");
            }
            Event::Introduce(kind) => {
                self.intro_pending.push(kind);
                // Practice counts too: the tutorial's egg is not explained
                // again in round 1.
                let mut progress = Progress::load(ctx);
                progress.meet(kind);
                progress.save(ctx);
            }
            Event::Spawn { id, kind, lane } => {
                // A pattern's own drops are covered by its call.
                let grouped = self.run.falling.iter().any(|f| f.id == id && f.group != 0);
                if self.callouts && !grouped && !self.intro_pending.contains(&kind) {
                    let text = format!("{}, lane {}.", sentence_start(kind.name()), lane + 1);
                    self.chatter(ctx, text);
                }
            }
            Event::Caught {
                kind, lane, held, ..
            } => {
                let key = if kind == Kind::Golden {
                    sound::CATCH_GOLDEN
                } else {
                    sound::CATCH_EGG
                };
                ctx.audio.play_at(key, ground(lane, lanes), Bus::EFFECTS);
                let text = if kind == Kind::Golden {
                    format!("Golden egg. {held} in the basket.")
                } else {
                    format!("{held} in the basket.")
                };
                self.chatter(ctx, text);
            }
            Event::BasketFull => {
                self.run_clock.reset();
                ctx.audio.play(sound::FULL, Bus::EFFECTS);
                let cart = self.cart_text();
                self.tell_queued(ctx, format!("Basket full. {cart}"));
            }
            Event::Overflow { kind, lane } => {
                let key = if kind == Kind::Golden {
                    sound::CRACK_GOLDEN
                } else {
                    sound::CRACK_EGG
                };
                ctx.audio.play_at(key, ground(lane, lanes), Bus::EFFECTS);
                ctx.say(format!("Basket full: the {} cracked.", kind.name()));
            }
            Event::Cracked { kind, lane } => {
                let key = if kind == Kind::Golden {
                    sound::CRACK_GOLDEN
                } else {
                    sound::CRACK_EGG
                };
                ctx.audio.play_at(key, ground(lane, lanes), Bus::EFFECTS);
                if kind == Kind::Golden {
                    self.chatter(ctx, "Golden egg lost.".to_string());
                }
            }
            Event::StreakBroken(n) => ctx.say_queued(format!("Streak of {n} broken.")),
            Event::Streak(n) => {
                ctx.audio.play(sound::STREAK, Bus::EFFECTS);
                ctx.say_queued(format!("Streak {n}."));
            }
            Event::Splat { lane, spoiled } => {
                self.run_clock.reset();
                ctx.audio
                    .play_at(sound::SPLAT, ground(lane, lanes), Bus::EFFECTS);
                if spoiled > 0 {
                    ctx.audio.play(sound::SPILL, Bus::EFFECTS);
                    self.tell(
                        ctx,
                        format!("Mud! {} spoiled.", sentence_start(&eggs_phrase(spoiled))),
                    );
                } else {
                    self.tell(ctx, "Mud. Nothing spoiled.");
                }
            }
            Event::MudMissed { lane } => {
                ctx.audio
                    .play_at(sound::MUD_THUD, ground(lane, lanes), Bus::EFFECTS);
            }
            Event::Stung { lane, lives } => {
                self.run_clock.reset();
                ctx.audio
                    .play_at(sound::STING, ground(lane, lanes), Bus::EFFECTS);
                if self.run.mode == Mode::Practice {
                    self.tell(ctx, "Stung! Free in practice.");
                } else {
                    ctx.audio.play(sound::LIFE_LOST, Bus::EFFECTS);
                    self.tell(
                        ctx,
                        format!("Stung! A basket breaks. {}", baskets_left(lives)),
                    );
                }
            }
            Event::BeeMissed { lane } => {
                ctx.audio
                    .play_at(sound::BEE_AWAY, ground(lane, lanes), Bus::EFFECTS);
            }
            Event::CrackedBasket { lives } => {
                ctx.audio.play(sound::LIFE_LOST, Bus::EFFECTS);
                self.tell_queued(
                    ctx,
                    format!(
                        "Too many cracked: a basket wears out. {}",
                        baskets_left(lives)
                    ),
                );
            }
            Event::Warning => {
                self.run_clock.reset();
                ctx.audio.play(sound::WARNING, Bus::EFFECTS);
                if self.run.basket.is_empty() {
                    self.tell(ctx, "Ten seconds left.");
                } else {
                    let cart = self.cart_text();
                    self.tell(ctx, format!("Ten seconds! Bank your eggs. {cart}"));
                }
            }
            Event::Tick(_) => {
                ctx.audio.play(sound::TICK, Bus::EFFECTS);
            }
            Event::TimeUp { lost } => {
                ctx.audio.play(sound::BELL, Bus::EFFECTS);
                if lost > 0 {
                    ctx.audio.play(sound::SPILL, Bus::EFFECTS);
                    let note = format!("{} lost.", sentence_start(&eggs_phrase(lost)));
                    self.tell(ctx, format!("Time. {note}"));
                    self.time_note = Some(note);
                } else {
                    self.tell(ctx, "Time.");
                }
            }
            Event::RoundCleared {
                round,
                banked,
                bonus,
                perfect,
            } => {
                if self.run.mode == Mode::Meadow {
                    let mut progress = Progress::load(ctx);
                    if round > progress.cleared {
                        progress.cleared = round;
                        progress.save(ctx);
                    }
                }
                let mut text = match self.run.mode {
                    Mode::Practice => format!(
                        "Practice round {round} done. You banked {}.",
                        eggs_phrase(banked as usize)
                    ),
                    _ => format!(
                        "Round {round} cleared. You banked {}. Round bonus {bonus}{}. Score {}.",
                        eggs_phrase(banked as usize),
                        if perfect {
                            ", with the perfect round bonus for no cracked eggs"
                        } else {
                            ""
                        },
                        self.run.score
                    ),
                };
                if self.run.phase == Phase::Between {
                    ctx.audio.play_music(Some(sound::ROUND_CLEAR), false);
                    text.push_str(&format!(
                        " Next, round {}: {}",
                        round + 1,
                        round_news(self.run.mode, round + 1)
                    ));
                }
                if let Some(note) = self.time_note.take() {
                    text = format!("{note} {text}");
                }
                self.summary = Some(text);
            }
            Event::RoundFailed { banked, needed } => {
                // The bell already said the lost eggs; the full results
                // pages keep them.
                let note = self
                    .time_note
                    .take()
                    .map_or(String::new(), |n| format!("{n} "));
                self.summary = Some(format!(
                    "{note}You banked {banked} of the {needed} eggs this round needed."
                ));
                self.headline = Some(format!("{banked} of {needed} banked, run over."));
            }
            Event::OutOfBaskets => {
                self.summary = Some("No baskets left. The run is over.".to_string());
                self.headline = Some("No baskets left, run over.".to_string());
            }
            Event::Won => {
                let ending = match self.run.mode {
                    Mode::Weekly => format!(
                        "Marathon complete: all {} rounds.",
                        crate::rules::WEEKLY_ROUNDS
                    ),
                    Mode::Festival => format!(
                        "Festival over. You caught {} of its patterns cleanly.",
                        self.run.patterns_done
                    ),
                    Mode::Easter => match self.run.golden_caught {
                        1 => "The hunt is over, with 1 golden egg found. Happy Easter.".to_string(),
                        n => format!("The hunt is over, with {n} golden eggs found. Happy Easter."),
                    },
                    _ => {
                        format!("Meadow cleared: all {ROUNDS} rounds. The endless meadow is open.")
                    }
                };
                self.add_to_summary(ending);
            }
            Event::Finished => self.add_to_summary("Today's meadow is done.".to_string()),
            Event::Pattern { pattern, lane } => {
                self.run_clock.reset();
                ctx.audio.play(sound::PATTERN_CALL, Bus::EFFECTS);
                let text = pattern_call(pattern, lane, self.lanes(), self.callouts);
                self.tell(ctx, text);
            }
            Event::PatternDone { pattern, bonus } => {
                ctx.audio.play(sound::PATTERN_DONE, Bus::EFFECTS);
                let text = format!(
                    "{} caught. {bonus} bonus points.",
                    sentence_start(pattern.name())
                );
                self.tell_queued(ctx, text);
            }
            Event::PatternBroken { pattern } => {
                ctx.audio.play(sound::PATTERN_BROKEN, Bus::EFFECTS);
                self.chatter(ctx, format!("{} broken.", sentence_start(pattern.name())));
            }
        }
    }

    /// The run's ending, after the last round's own summary.
    fn add_to_summary(&mut self, ending: String) {
        self.headline = Some(ending.clone());
        self.summary = Some(match self.summary.take() {
            Some(round) => format!("{round} {ending}"),
            None => ending,
        });
    }

    /// Saves the run, plays its ending and opens the results.
    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        self.done = true;
        self.stop_loops(ctx);
        self.score.stop(ctx);
        ctx.audio.set_bed(None);
        let won = self.run.over() == Some(true);
        ctx.audio
            .play_music(Some(if won { sound::WIN } else { sound::LOSE }), false);
        let mut lines = Vec::new();
        let mut title = self.headline.take().unwrap_or_default();
        if let Some(s) = self.summary.take() {
            if title.is_empty() {
                title = s.clone();
            }
            lines.push(s);
        }
        lines.push(format!(
            "Final score {}. {} caught, {} banked, {} cracked. Longest streak {}.",
            self.run.score,
            eggs_phrase(self.run.caught_total as usize),
            self.run.banked_total,
            self.run.cracks_total,
            self.run.best_streak
        ));
        let pace = self.run_clock.pace();
        if let Some(challenge) = &self.challenge {
            let r = Challenges::submit_at(ctx, challenge, pace, self.run.score);
            let period = challenge.period;
            let streak = Challenges::streak(ctx, ID, period);
            let result = format!(
                "{} result: {}. {}",
                challenge.title(),
                self.run.score,
                score_line(r, pace)
            );
            title = format!("{title} {result}");
            lines.push(result);
            let streak = format!(
                "{} streak: {streak} {}{}. {}",
                period.label(),
                period.unit(),
                if streak == 1 { "" } else { "s" },
                challenge.resets()
            );
            title = format!("{title} {streak}");
            lines.push(streak);
        } else if let Some(board) = self.run.mode.board() {
            // A 0 goes on no board, at any pace, so no view says "best 0".
            let score = self.run.score;
            let Some(r) = parlor_core::pace::record(ctx, ID, &pace.board(board), score) else {
                title = format!("{title} {ZERO_SCORE}");
                lines.push(ZERO_SCORE.to_string());
                ctx.replace(results(self.launch, won, title.trim().to_string(), lines));
                return;
            };
            title = format!("{title} {}", short_score_line(self.run.score, r, pace));
            lines.push(score_line(r, pace));
            if self.run.mode == Mode::Easter {
                let year = easter::year_of(parlor_core::challenges::today(ctx));
                let mut keep = Keepsakes::load_at(ctx, pace);
                let kept = keep.keep(year, self.run.score, self.run.golden_caught);
                keep.store_at(ctx, pace);
                let kept = kept.line(year, pace);
                title = format!("{title} {kept}");
                lines.push(kept);
            }
        } else {
            lines.push("Practice scores are not saved.".to_string());
        }
        ctx.replace(results(self.launch, won, title.trim().to_string(), lines));
    }
}

/// A festival pattern's call, as it starts to fall. With drop calls off it
/// names the pattern but not its lane.
pub fn pattern_call(pattern: Pattern, lane: usize, lanes: usize, callouts: bool) -> String {
    match (pattern, callouts) {
        (Pattern::Sweep, _) => {
            let (from, first) = if lane == 0 {
                ("left", 1)
            } else {
                ("right", lanes)
            };
            format!("Egg sweep from the {from}! Start in lane {first} and run along with it.")
        }
        (Pattern::Fence, true) => format!("Bee fence! The gap is in lane {}.", lane + 1),
        (Pattern::Fence, false) => "Bee fence! Find the gap.".to_string(),
        (Pattern::Nest, true) => format!(
            "A nest in lane {}: three eggs, then mud. Catch, then step aside.",
            lane + 1
        ),
        (Pattern::Nest, false) => {
            "A nest: three eggs in one lane, then mud. Catch, then step aside.".to_string()
        }
    }
}

/// What a run that scored nothing says: a 0 goes on no board.
pub const ZERO_SCORE: &str = "Score 0, so nothing goes on the board.";

/// The score and what saving it meant, in a few words: "Score 61, saved."
/// A relaxed run's best is its relaxed best.
pub fn short_score_line(score: u64, r: ScoreResult, pace: Pace) -> String {
    let word = pace.best_word();
    match r {
        ScoreResult::First => format!("Score {score}, saved."),
        ScoreResult::NewBest { .. } => format!("Score {score}, a new {word}."),
        ScoreResult::Short { best } => format!("Score {score}, {word} {best}."),
    }
}

/// What saving a score meant, on the board for `pace`.
pub fn score_line(r: ScoreResult, pace: Pace) -> String {
    let word = pace.best_word();
    match r {
        ScoreResult::First if pace.separate() => {
            "Your first relaxed score here is saved.".to_string()
        }
        ScoreResult::First => "Your first score here is saved.".to_string(),
        ScoreResult::NewBest { previous } => {
            format!("A new {word}. The old {word} was {previous}.")
        }
        ScoreResult::Short { best } => format!("Your {word} is {best}."),
    }
}

impl State<Parlor> for RunScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        RUN_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        let progress = Progress::load(ctx);
        if self.run.mode == Mode::Practice {
            self.callouts = true;
        } else {
            self.callouts = !progress.quiet_calls;
            self.run.by_ear = progress.quiet_calls;
            for kind in Kind::ALL {
                if progress.has_met(kind) {
                    self.run.mark_introduced(kind);
                }
            }
        }
        self.run.introduce = true;
        self.run_clock = RunClock::start(ctx, ID);
        sound::meadow(ctx);
        self.announce_round(ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_loops(ctx);
        self.score.stop(ctx);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        self.run_clock.reset();
        if self.run.mode != Mode::Practice {
            // Settings, from the pause menu, may have changed the call-outs.
            // Switching them on ends the by-ear bonus for this run; switching
            // them off starts it only from the next run.
            let quiet = Progress::load(ctx).quiet_calls;
            self.callouts = !quiet;
            if !quiet {
                self.run.by_ear = false;
            }
        }
        if self.frozen {
            self.frozen = false;
            ctx.audio.resume_all();
            if let Some(kind) = self.introducing.take() {
                if let Some(f) = self.run.falling.iter().rev().find(|f| f.kind == kind) {
                    let text = format!(
                        "{} in lane {}, {}.",
                        sentence_start(kind.name()),
                        f.lane + 1,
                        self.run.relative(f.lane)
                    );
                    ctx.say(text);
                }
            }
            return;
        }
        if self.awaiting_next {
            self.awaiting_next = false;
            self.run.next_round();
            sound::meadow(ctx);
            self.announce_round(ctx);
            return;
        }
        let text = format!(
            "Back in the meadow. Lane {} of {}, basket {} of {}.",
            self.run.lane + 1,
            self.lanes(),
            self.run.basket.len(),
            self.run.capacity()
        );
        ctx.say(text);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() || self.done {
            return;
        }
        if event.pressed() == Some(Key::Escape) {
            let leave = if self.launch.mode.period().is_some() {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        let Some(action) = ctx.action_for(event) else {
            return;
        };
        self.run_clock.asked_by(Some(action));
        if matches!(
            action,
            ParlorAction::Secondary | ParlorAction::Up | ParlorAction::Down
        ) {
            // The cart's place and the sideways reminder are asked for too.
            self.run_clock.asked();
        }
        match action {
            ParlorAction::Left | ParlorAction::Right => {
                let delta = if action == ParlorAction::Left { -1 } else { 1 };
                match self.run.step(delta) {
                    Move::Moved { lane, at_cart } => {
                        ctx.audio.play(sound::STEP, Bus::EFFECTS);
                        sound::listen_from(ctx, lane, self.lanes());
                        if at_cart {
                            ctx.audio.play_at(
                                sound::AT_CART,
                                ground(lane, self.lanes()),
                                Bus::EFFECTS,
                            );
                            ctx.say(format!("Lane {}, the cart.", lane + 1));
                        } else {
                            ctx.say(format!("Lane {}.", lane + 1));
                        }
                    }
                    Move::Edge => {
                        ctx.audio.play(sound::EDGE, Bus::EFFECTS);
                        let side = if delta < 0 { "left" } else { "right" };
                        ctx.say(format!(
                            "The fence on the {side}. Lane {}.",
                            self.run.lane + 1
                        ));
                    }
                    Move::Idle => ctx.say("Wait for the next round."),
                }
            }
            ParlorAction::Primary if self.run.start_now() => {
                ctx.audio.play(sound::READY, Bus::EFFECTS);
                self.busy_until = self.clock;
                ctx.say("Go.");
            }
            ParlorAction::Primary => match self.run.bank() {
                Bank::Banked { eggs, points, full } => {
                    ctx.audio.play(sound::BANK, Bus::EFFECTS);
                    let mut text = format!(
                        "Banked {}, {points} points{}. Score {}.",
                        eggs_phrase(eggs),
                        if full {
                            ", with the full basket bonus"
                        } else {
                            ""
                        },
                        self.run.score
                    );
                    let needed = self.run.needed();
                    if needed > 0 {
                        let had = self.run.banked_round - eggs as u32;
                        if self.run.banked_round >= needed && had < needed {
                            text.push_str(" That is enough to clear the round.");
                        } else if self.run.banked_round < needed {
                            text.push_str(&format!(
                                " {} more to bank.",
                                needed - self.run.banked_round
                            ));
                        }
                    }
                    self.tell(ctx, text);
                }
                Bank::Empty => {
                    ctx.audio.play(sound::EMPTY, Bus::EFFECTS);
                    ctx.say("Your basket is empty.");
                }
                Bank::NotAtCart { lanes, left } => {
                    ctx.audio.play(sound::EMPTY, Bus::EFFECTS);
                    let way = if left { "left" } else { "right" };
                    ctx.say(format!(
                        "Cart {} {way}, lane {}.",
                        lanes_phrase(lanes),
                        self.run.cart_lane() + 1
                    ));
                }
                Bank::Idle => ctx.say("Wait for the next round."),
            },
            ParlorAction::Secondary => {
                let cart = self.run.cart_lane();
                ctx.audio
                    .play_at(sound::BEACON, ground(cart, self.lanes()), Bus::EFFECTS);
                let text = self.cart_text();
                ctx.say(text);
            }
            ParlorAction::Status => {
                let layers = self.run.status_layers();
                self.status_key.speak(ctx, layers);
            }
            ParlorAction::Repeat => {
                if self.last.is_empty() {
                    ctx.say("Nothing to repeat yet.");
                } else {
                    let text = self.last.clone();
                    ctx.say(text);
                }
            }
            ParlorAction::Hint => {
                let text = if matches!(self.run.phase, Phase::LeadIn(_)) {
                    format!(
                        "At the cart. {} starts the round.",
                        sentence_start(&ctx.control_name(ParlorAction::Primary))
                    )
                } else {
                    self.run.hint()
                };
                ctx.say(text);
            }
            ParlorAction::Up | ParlorAction::Down => {
                let left = ctx.control_name(ParlorAction::Left);
                let right = ctx.control_name(ParlorAction::Right);
                ctx.say(format!(
                    "Sideways only: {left} and {right}. Lane {}.",
                    self.run.lane + 1
                ));
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.done || self.frozen || self.awaiting_next {
            return;
        }
        if !self.intro_pending.is_empty() {
            let kind = self.intro_pending.remove(0);
            self.frozen = true;
            self.introducing = Some(kind);
            ctx.audio.pause_all();
            ctx.push(PlayPage::new(introduction(kind)));
            return;
        }
        // Speech is paced by real time.
        self.clock += dt;
        let game_dt = if matches!(self.run.phase, Phase::LeadIn(_)) {
            // The lead-in is timed to the round's announcement as it is
            // spoken, so it keeps real time, at every pace.
            self.run_clock.bypass();
            self.run_clock.game_dt(ctx, dt);
            dt
        } else {
            self.run_clock.game_dt(ctx, dt)
        };
        self.run.update(game_dt);
        for event in self.run.take_events() {
            self.on_rule_event(ctx, event);
        }
        if self.run.over().is_some() {
            self.finish(ctx);
            return;
        }
        self.sync_loops(ctx);
        let mood = self.mood();
        self.score.follow(ctx, mood, dt);
        if self.run.phase == Phase::Playing {
            self.beacon_in -= dt;
            if self.beacon_in <= 0.0 {
                self.beacon_in += BEACON_EVERY;
                let cart = self.run.cart_lane();
                ctx.audio
                    .play_at(sound::BEACON, ground(cart, self.lanes()), Bus::EFFECTS);
            }
        }
        sound::listen_from(ctx, self.run.lane, self.lanes());
        if !self.intro_pending.is_empty() {
            // Explained before it is heard falling any further.
            let kind = self.intro_pending.remove(0);
            self.frozen = true;
            self.introducing = Some(kind);
            ctx.audio.pause_all();
            ctx.push(PlayPage::new(introduction(kind)));
            return;
        }
        if self.run.phase == Phase::Between {
            self.stop_loops(ctx);
            self.score.stop(ctx);
            self.awaiting_next = true;
            let text = self
                .summary
                .take()
                .unwrap_or_else(|| "Round over.".to_string());
            self.last = text.clone();
            ctx.push(PlayPage::new(text));
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let r = &self.run;
        let phase = match r.phase {
            Phase::LeadIn(_) => "ready",
            Phase::Playing => "playing",
            Phase::Between => "between",
            Phase::Over { .. } => "over",
        };
        let mut lines = vec![
            format!("{}, round {}, {phase}", r.mode.label(), r.round),
            format!(
                "lane {} of {}, cart lane {}",
                r.lane + 1,
                r.lanes(),
                r.cart_lane() + 1
            ),
            format!(
                "basket {} of {}, score {}",
                r.basket.len(),
                r.capacity(),
                r.score
            ),
            format!(
                "banked {} of {}, baskets {}, time {:.1}",
                r.banked_round,
                r.needed(),
                r.lives,
                r.time_left()
            ),
        ];
        for f in &r.falling {
            lines.push(format!(
                "falling: {} {} {:.2}",
                f.kind.id(),
                f.lane + 1,
                f.left
            ));
        }
        lines
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let goal = match self.run.needed() {
            0 => String::new(),
            n => format!(
                " This round needs {n} eggs banked. You have {} so far.",
                self.run.banked_round
            ),
        };
        let rule = match (self.run.mode, self.run.twist) {
            (Mode::Daily, Some(t)) => format!(
                " Today's twist, {}: {}",
                t.name(),
                sentence_start(t.describe())
            ),
            (Mode::Festival, _) => format!(
                " Festival patterns are called as they start. An egg sweep crosses every lane: run along with it. A bee fence leaves one gap, and the gap holds a golden egg. A nest drops three eggs in one lane, then mud. Patterns caught cleanly so far: {}.",
                self.run.patterns_done
            ),
            (Mode::Weekly, _) => format!(
                " The marathon, round {} of {}.",
                self.run.round,
                crate::rules::WEEKLY_ROUNDS
            ),
            (Mode::Easter, _) => format!(
                " Easter morning, round {} of {}: golden eggs are worth double, there are no bees, and no round has an egg target.",
                self.run.round,
                crate::rules::EASTER_ROUNDS
            ),
            _ => String::new(),
        };
        format!(
            "{}.{rule} {}{goal} {}",
            self.run.mode.label(),
            controls_text(ctx),
            self.cart_text()
        )
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

/// A page that opens in the middle of play (a first-time explanation, a
/// round's summary). Enter, Escape and the repeat key work as on any page;
/// every other key, Space (the bank key) above all, is answered instead of
/// skipping the page, so a key pressed a moment late cannot throw it away
/// unheard.
pub struct PlayPage {
    inner: Pages,
}

impl PlayPage {
    pub fn new(text: impl Into<String>) -> PlayPage {
        PlayPage {
            inner: Pages::new([text.into()]),
        }
    }
}

impl State<Parlor> for PlayPage {
    fn name(&self) -> &'static str {
        "pages"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        self.inner.enter(ctx);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        self.inner.resume(ctx);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() {
            return;
        }
        let passes = matches!(
            event.pressed(),
            Some(Key::Return | Key::KpEnter | Key::Escape)
        ) || ctx.action_for(event) == Some(ParlorAction::Repeat);
        if passes {
            self.inner.handle_input(ctx, event);
        } else if event.pressed().is_some() {
            ctx.say(format!(
                "The meadow waits while you read. Enter goes on, {} hears it again.",
                ctx.control_name(ParlorAction::Repeat)
            ));
        }
    }

    fn lines(&self, ctx: &Ctx<Parlor>) -> Vec<String> {
        self.inner.lines(ctx)
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        format!(
            "Play pauses while a page is open: nothing falls and the clock is stopped. {}",
            self.inner.help(ctx)
        )
    }
}

/// After a run: what happened, and what next, on the shared results menu.
/// The whole result is the menu's title, so it is heard on entry.
pub fn results(launch: Launch, won: bool, title: String, lines: Vec<String>) -> Results {
    let back = match launch.mode.period() {
        Some(_) => Back::Challenges(CHALLENGES_NAME),
        None => Back::Title(TITLE_NAME),
    };
    let (again, help) = match launch.mode.period() {
        Some(period) => (
            labels::challenge_again(period),
            "The same drops again. Only your best result is kept.",
        ),
        None if won || launch.mode == Mode::Endless || launch.mode == Mode::Practice => (
            labels::PLAY_AGAIN.to_string(),
            "Another run of the same kind, with new drops.",
        ),
        None => (
            labels::TRY_AGAIN.to_string(),
            "Another run of the same kind, with new drops.",
        ),
    };
    Results::new(title, lines, back).again(again, help, move |ctx| {
        let launch = Launch {
            seed: if launch.mode.period().is_some() {
                launch.seed
            } else {
                launch.seed.map(|seed| seed.wrapping_add(1))
            },
            ..launch
        };
        start(ctx, launch, true)
    })
}

mod visuals_runscreen;
