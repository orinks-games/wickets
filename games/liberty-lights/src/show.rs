//! A round in the sky: hear the show, then launch it back.

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, State};
use parlor_core::menus::{self, Leave};
use parlor_core::{Mood, Parlor, ParlorAction, Score};

use crate::controls::{
    aim_for, aim_line, held_pos, is_enter, keys_line, kind_for, on_pad, wheel_line, wheel_step,
    Pick, Wheel, KEYS_LINE, SHOW_PICKS,
};
use crate::festival::{Run, Twist};
use crate::independence::{self, NightRun};
use crate::results;
use crate::rules::{
    self, describe_show, generate, generate_finale, judge_with, level_size, tier_description,
    uses_positions, Cue, Kind, Launch, Phase, Round, HINT_COST, TOTAL_LEVELS,
};
use crate::sound;
use crate::HELP;

/// What a round is part of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    /// A campaign level, with the show points earned so far this run.
    Level { level: u32, total: u64 },
    /// A grand-finale round, with the points of the rounds before it.
    Finale { round: u32, total: u64 },
    /// The player's own saved freestyle show as a challenge.
    MyShow,
    /// A show of a daily, weekly or monthly challenge.
    Challenge(Box<Run>),
    /// An act of Independence night, on the Fourth of July.
    Independence(Box<NightRun>),
}

impl Mode {
    /// "Level 3", "Grand finale, round 2", "Your saved show".
    pub fn title(&self) -> String {
        match self {
            Mode::Level { level, .. } => format!("Level {level}"),
            Mode::Finale { round, .. } => format!("Grand finale, round {round}"),
            Mode::MyShow => "Your saved show".to_string(),
            Mode::Challenge(run) => {
                if run.plan.acts.len() == 1 {
                    run.challenge.title()
                } else {
                    format!("{}, {}", run.challenge.title(), run.progress())
                }
            }
            Mode::Independence(run) => format!("{}, {}", independence::NAME, run.progress()),
        }
    }

    /// The challenge's twist, if this is a challenge show with one.
    pub fn twist(&self) -> Option<Twist> {
        match self {
            Mode::Challenge(run) => run.plan.twist,
            _ => None,
        }
    }

    pub fn total(&self) -> u64 {
        match self {
            Mode::Level { total, .. } | Mode::Finale { total, .. } => *total,
            Mode::MyShow => 0,
            Mode::Challenge(run) => run.total,
            Mode::Independence(run) => run.total,
        }
    }

    /// Whether this round's fireworks can go up anywhere but the center.
    pub fn placed(&self, show: &[Launch]) -> bool {
        match self {
            Mode::Level { level, .. } => uses_positions(*level),
            Mode::Finale { .. } => true,
            Mode::MyShow => show.iter().any(|l| l.pos != rules::Pos::Center),
            Mode::Challenge(run) => run.placed(),
            Mode::Independence(_) => true,
        }
    }
}

/// Opens the pause screen over a practice or freestyle screen, whose leave
/// row goes back to the title.
pub fn pause_to_title(ctx: &mut Ctx<Parlor>) {
    menus::pause(ctx, HELP, Leave::Title(crate::TITLE_NAME));
}

pub struct Show {
    pub mode: Mode,
    pub round: Round,
    /// What the main action launches while playing; Up and Down change it.
    pub wheel: Wheel,
    /// The band's music, following the round.
    pub score: Score,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
    /// Run by the tutorial, whose own step says what to do on the turn.
    pub lesson: bool,
    /// F1, in layers: the keys for now, then every key.
    help_key: parlor_core::StatusKey,
    /// The phase F1 was last pressed in: a new phase starts F1 over.
    help_phase: Option<Phase>,
}

impl Show {
    /// A new show for `mode`. A saved-show challenge must use `with_show`.
    pub fn new(mode: Mode) -> Show {
        let mut rng = rules::show_rng();
        let show = match &mode {
            Mode::Level { level, .. } => generate(*level, &mut rng),
            Mode::Finale { round, .. } => generate_finale(*round, &mut rng),
            Mode::MyShow => Vec::new(),
            Mode::Challenge(run) => run.act().show.clone(),
            Mode::Independence(run) => run.act().show.clone(),
        };
        Show::with_show(mode, show)
    }

    /// A round of exactly this show.
    pub fn with_show(mode: Mode, show: Vec<Launch>) -> Show {
        Show {
            status_key: Default::default(),
            mode,
            round: Round::new(show),
            wheel: Wheel::new(&SHOW_PICKS),
            score: Score::new(),
            lesson: false,
            help_key: Default::default(),
            help_phase: None,
        }
    }

    /// What the score hears: only the band's base while the show plays, so
    /// every firework stands clear; your own launches and later levels
    /// build it; the grand finale hurries it.
    pub fn mood(&self) -> Mood {
        let height = match &self.mode {
            Mode::Level { level, .. } => (*level as f32 - 1.0) / (TOTAL_LEVELS.max(2) as f32 - 1.0),
            _ => 1.0,
        }
        .clamp(0.0, 1.0);
        let launched = self.round.launches.len() as f32 / self.round.show.len().max(1) as f32;
        let intensity = match self.round.phase {
            Phase::Listening => 0.0,
            Phase::Ready => 0.15 + 0.2 * height,
            Phase::YourTurn => 0.25 + 0.2 * height,
            Phase::Playing => 0.4 + 0.3 * launched.min(1.0) + 0.3 * height,
        };
        let finale = match &self.mode {
            Mode::Finale { .. } => true,
            Mode::Independence(run) => run.stage + 1 >= run.acts.len(),
            _ => false,
        } && self.round.phase != Phase::Listening;
        Mood::new(intensity).with_flag(parlor_core::score::HURRY, finale)
    }

    fn intro(&self, ctx: &Ctx<Parlor>) -> String {
        let p = sentence_start(&ctx.control_name(ParlorAction::Primary));
        let n = self.round.show.len();
        match &self.mode {
            Mode::Level { level, .. } => format!(
                "Level {level}. {} Hear the show, then launch it back. {p} plays the show.",
                tier_description(*level)
            ),
            Mode::Finale { round, .. } => format!(
                "Grand finale, round {round}. {n} fireworks, all over the sky. {p} plays the show."
            ),
            Mode::MyShow => format!(
                "Your saved show, {n} fireworks. Hear it, then launch it back. {p} plays the show."
            ),
            Mode::Challenge(run) => {
                let several = run.plan.acts.len() > 1;
                let head = if several {
                    format!(
                        "{}, {}: {}.",
                        run.challenge.title(),
                        run.progress(),
                        run.act().name
                    )
                } else {
                    format!("{}: {}.", run.challenge.title(), run.act().name)
                };
                let twist = match (run.plan.twist, run.plan.festival) {
                    (Some(t), _) => format!(" Twist: {}.", t.name()),
                    (None, Some(f)) => {
                        format!(" {}s pay a bonus.", sentence_start(f.signature.name()))
                    }
                    _ => String::new(),
                };
                let total = if run.stage > 0 {
                    format!(" {} show points so far.", run.total)
                } else {
                    String::new()
                };
                format!("{head} {n} fireworks.{twist}{total} {p} plays the show.")
            }
            Mode::Independence(run) => {
                let total = if run.stage > 0 {
                    format!(" {} show points so far.", run.total)
                } else {
                    String::new()
                };
                format!(
                    "{}, {}: {}. {n} fireworks. Booms matched closely pay a bonus.{total} {p} plays the show.",
                    independence::NAME,
                    run.progress(),
                    run.act().name
                )
            }
        }
    }

    /// The show's last line before the turn, naming how to finish.
    fn finish_line(&self, ctx: &Ctx<Parlor>) -> String {
        if on_pad(ctx) {
            return format!(
                "When finished, choose Finish with {} and press {}.",
                ctx.control_name(ParlorAction::Up),
                ctx.control_name(ParlorAction::Primary)
            );
        }
        format!(
            "Press Enter when finished, or choose Finish with {}.",
            ctx.control_name(ParlorAction::Up)
        )
    }

    /// Whether the turn names every key: on level 1, on the first level
    /// where fireworks go up left and right, and on Independence night's
    /// first act, which aims from the start. Elsewhere it is short.
    fn full_turn_line(&self) -> bool {
        match &self.mode {
            Mode::Level { level, .. } => {
                *level == 1 || (uses_positions(*level) && !uses_positions(*level - 1))
            }
            Mode::Independence(run) => run.stage == 0,
            _ => false,
        }
    }

    fn turn_line(&self, ctx: &Ctx<Parlor>) -> String {
        let p = ctx.control_name(ParlorAction::Primary);
        if !self.full_turn_line() {
            return format!("Your turn. {} counts in.", sentence_start(&p));
        }
        let aim = if self.mode.placed(&self.round.show) {
            format!(" {}", aim_line(ctx))
        } else {
            String::new()
        };
        format!(
            "Your turn. Press {p} for your count-in, then launch each firework when it went up in the show. {}{aim} {}",
            keys_line(ctx),
            self.finish_line(ctx)
        )
    }

    /// What R says, by phase: the original's lines.
    fn prompt(&self, ctx: &Ctx<Parlor>) -> String {
        let p = ctx.control_name(ParlorAction::Primary);
        let t = ctx.control_name(ParlorAction::Secondary);
        match self.round.phase {
            Phase::Ready => {
                format!("Listen phase. Press {p} to hear the show, then launch it back.")
            }
            Phase::Listening => "The show is playing.".to_string(),
            Phase::YourTurn if self.mode.twist() == Some(Twist::OneShot) => {
                format!("Your turn. Press {p} for the count-in, then launch.")
            }
            Phase::YourTurn => format!(
                "Your turn. Press {p} for the count-in, then launch. {} hears the show again.",
                sentence_start(&t)
            ),
            Phase::Playing => {
                let aim = if self.mode.placed(&self.round.show) {
                    format!(" {}", aim_line(ctx))
                } else {
                    String::new()
                };
                format!(
                    "Play phase. {KEYS_LINE}{aim} Enter finishes. Backspace takes back the last firework. {} {p} now: {}",
                    wheel_line(ctx, true),
                    self.wheel.current().spoken()
                )
            }
        }
    }

    /// The status key, in layers. First: the level, its fireworks and
    /// where the round stands. A second press: the level's style, the
    /// twist or signature firework, points so far and a used hint.
    fn status(&self, ctx: &Ctx<Parlor>) -> Vec<String> {
        let n = self.round.show.len();
        let fireworks = if n == 1 {
            "1 firework".to_string()
        } else {
            format!("{n} fireworks")
        };
        let mut more = Vec::new();
        let head = match &self.mode {
            Mode::Level { level, .. } => {
                more.push(tier_description(*level).to_string());
                format!("Level {level} of {TOTAL_LEVELS}, {fireworks}.")
            }
            Mode::Finale { round, .. } => format!("Grand finale, round {round}, {fireworks}."),
            Mode::MyShow => format!("Your saved show, {fireworks}."),
            Mode::Challenge(run) => {
                match (run.plan.twist, run.plan.festival) {
                    (Some(t), _) => more.push(format!("Twist: {}.", t.name())),
                    (None, Some(f)) => {
                        more.push(format!("Signature firework: {}.", f.signature.name()))
                    }
                    _ => {}
                }
                format!("{}, {fireworks}.", self.mode.title())
            }
            Mode::Independence(run) => {
                more.push("Signature firework: boom.".to_string());
                format!(
                    "{}, {}, {}, {fireworks}.",
                    independence::NAME,
                    run.progress(),
                    run.act().name
                )
            }
        };
        let phase = match self.round.phase {
            Phase::Ready => format!(
                "The show has not played yet. {} plays it.",
                ctx.control_name(ParlorAction::Primary)
            ),
            Phase::Listening => "The show is playing.".to_string(),
            Phase::YourTurn => "Your turn, waiting for your count-in.".to_string(),
            Phase::Playing => {
                let ms = self.round.elapsed_ms();
                let clock = if ms < 0 {
                    "counting in".to_string()
                } else {
                    format!("{} since the go", rules::seconds(ms))
                };
                format!(
                    "Launched {} of {n}, {clock}. {} selected.",
                    self.round.launches.len(),
                    self.wheel.current().spoken().trim_end_matches('.')
                )
            }
        };
        match self.mode.total() {
            0 => {}
            t => more.push(format!("{t} show points so far.")),
        }
        if self.round.hint_used {
            more.push("Hint used.".to_string());
        }
        vec![
            parlor_core::status::sentences([head, phase]),
            parlor_core::status::sentences(more),
        ]
    }

    fn hint(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.mode.twist() == Some(Twist::OneShot) {
            ctx.say("No hints tonight: the twist is one shot.");
            return;
        }
        if self.round.listens == 0 {
            ctx.say(format!(
                "Hear the show first: press {}.",
                ctx.control_name(ParlorAction::Primary)
            ));
            return;
        }
        ctx.audio.play(sound::HINT, Bus::UI);
        let cost = if self.round.hint_used {
            String::new()
        } else {
            format!(" That hint costs {HINT_COST} points.")
        };
        self.round.hint_used = true;
        ctx.say(format!("{}{cost}", describe_show(&self.round.show)));
    }

    fn listen(&mut self, ctx: &mut Ctx<Parlor>) {
        self.round.listen();
        ctx.say("The show starts.");
        let cues = self.round.update(0.0);
        self.play_cues(ctx, cues);
    }

    fn play_cues(&mut self, ctx: &mut Ctx<Parlor>, cues: Vec<Cue>) {
        for cue in cues {
            match cue {
                Cue::Tick(_) => {
                    ctx.audio.play(sound::TICK, Bus::EFFECTS);
                }
                Cue::Go => {
                    ctx.audio.play(sound::GO, Bus::EFFECTS);
                }
                Cue::Burst(l) => {
                    if self.mode.twist() == Some(Twist::NightFog) {
                        sound::fogged(ctx, l);
                    } else {
                        sound::launch(ctx, l);
                    }
                }
                Cue::TurnBegins => {
                    ctx.audio.play(sound::FUSE, Bus::EFFECTS);
                    if !self.lesson {
                        let line = self.turn_line(ctx);
                        ctx.say(line);
                    }
                }
            }
        }
    }

    /// Launches `kind` now, or says why not.
    fn fire(&mut self, ctx: &mut Ctx<Parlor>, kind: Kind) {
        let pos = held_pos(ctx);
        match self.round.phase {
            Phase::Playing => {
                if let Some(l) = self.round.launch(kind, pos) {
                    sound::launch(ctx, l);
                }
            }
            Phase::YourTurn => ctx.say(format!(
                "Press {} for the count-in first, then launch.",
                ctx.control_name(ParlorAction::Primary)
            )),
            Phase::Ready => ctx.say(format!(
                "Play the show first: press {}.",
                ctx.control_name(ParlorAction::Primary)
            )),
            Phase::Listening => ctx.say("Wait for the show to finish."),
        }
    }

    /// The main action: hear the show, start the count-in, then do
    /// whatever the wheel has chosen.
    fn primary(&mut self, ctx: &mut Ctx<Parlor>) {
        match self.round.phase {
            Phase::Ready => self.listen(ctx),
            Phase::Listening => ctx.say("The show is still playing."),
            Phase::YourTurn => {
                // The turn instructions must not talk over the count-in.
                ctx.stop_speech();
                self.round.play();
                let cues = self.round.update(0.0);
                self.play_cues(ctx, cues);
            }
            Phase::Playing => match self.wheel.current() {
                Pick::Fire(kind) => self.fire(ctx, kind),
                Pick::TakeBack => self.take_back(ctx),
                Pick::Finish => self.finish(ctx),
            },
        }
    }

    fn take_back(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.round.phase != Phase::Playing {
            ctx.say("Nothing to take back yet.");
        } else if let Some(l) = self.round.undo() {
            ctx.audio.play(sound::UNDO, Bus::UI);
            ctx.say(format!("Last firework removed: {}.", l.kind.name()));
        } else {
            ctx.say("Nothing to take back.");
        }
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        match self.round.phase {
            Phase::Playing => self.submit(ctx),
            Phase::YourTurn => ctx.say(format!(
                "Press {} for the count-in and launch your show first.",
                ctx.control_name(ParlorAction::Primary)
            )),
            _ => ctx.say("Play the show first."),
        }
    }

    fn submit(&mut self, ctx: &mut Ctx<Parlor>) {
        let judgement = match &self.mode {
            Mode::Challenge(run) => judge_with(
                &self.round.show,
                &self.round.launches,
                self.round.hint_used,
                run.plan.scoring(),
            ),
            Mode::Independence(run) => judge_with(
                &self.round.show,
                &self.round.launches,
                self.round.hint_used,
                run.scoring(),
            ),
            _ => self.round.judge(),
        };
        let results = results::score(
            self.mode.clone(),
            self.round.show.clone(),
            self.round.launches.clone(),
            judgement,
            ctx,
        );
        ctx.replace(results);
    }
}

impl State<Parlor> for Show {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        "liberty_lights_show"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::night(ctx);
        let mood = self.mood();
        self.score.start(ctx, &sound::SCORE, mood);
        match &self.mode {
            Mode::Challenge(_) => {
                ctx.audio.play(sound::TWIST, Bus::UI);
            }
            Mode::Independence(run) if run.stage == 0 => {
                ctx.audio.play(sound::SALUTE, Bus::UI);
            }
            Mode::Independence(_) => {
                ctx.audio.play(sound::TWIST, Bus::UI);
            }
            _ => {}
        }
        let intro = self.intro(ctx);
        ctx.say(intro);
    }

    /// Back from the pause menu or a help page: the sky picks up where it
    /// was.
    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::night(ctx);
        let prompt = self.prompt(ctx);
        ctx.say(format!("{}. {prompt}", self.mode.title()));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() {
            return;
        }
        if event.pressed() == Some(Key::Escape) {
            let leave = if matches!(self.mode, Mode::Challenge(_)) {
                Leave::Challenge(crate::CHALLENGES_NAME)
            } else {
                Leave::Title(crate::TITLE_NAME)
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        if let Some(kind) = kind_for(event) {
            self.wheel.select(kind);
            self.fire(ctx, kind);
            return;
        }
        // Actions before Enter: on a controller A is both the main action
        // and Enter, and here it must be the main action.
        let action = ctx.action_for(event);
        if action == Some(ParlorAction::Primary) {
            self.primary(ctx);
            return;
        }
        if let Some(step) = wheel_step(ctx, event) {
            let pick = self.wheel.step(step);
            ctx.audio.play(sound::SELECT, Bus::UI);
            ctx.say(pick.spoken());
            return;
        }
        if let Some(pos) = aim_for(ctx, event) {
            sound::aim(ctx, pos);
            return;
        }
        if is_enter(event) {
            self.finish(ctx);
            return;
        }
        if event.pressed() == Some(Key::Backspace) {
            self.take_back(ctx);
            return;
        }
        match action {
            Some(ParlorAction::Secondary) => match self.round.phase {
                Phase::Listening => ctx.say("The show is already playing."),
                _ if self.mode.twist() == Some(Twist::OneShot) && self.round.listens > 0 => {
                    ctx.say("Tonight's twist is one shot: the show plays only once.")
                }
                _ => self.listen(ctx),
            },
            Some(ParlorAction::Repeat) => {
                let prompt = self.prompt(ctx);
                ctx.say(prompt);
            }
            Some(ParlorAction::Status) => {
                let layers = self.status(ctx);
                self.status_key.speak(ctx, layers);
            }
            Some(ParlorAction::Hint) => self.hint(ctx),
            _ => {}
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.score.stop(ctx);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let cues = self.round.update(dt);
        if !cues.is_empty() {
            self.play_cues(ctx, cues);
        }
        let mood = self.mood();
        // The base drops away fast as the show starts, not over a beat.
        if self.round.phase == Phase::Listening && self.score.mood().intensity > 0.0 {
            self.score.jump(ctx, mood);
        } else {
            self.score.follow(ctx, mood, dt);
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        vec![
            self.mode.title(),
            format!("{:?}", self.round.phase),
            format!(
                "Launched {} of {}",
                self.round.launches.len(),
                self.round.show.len()
            ),
        ]
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let p = sentence_start(&ctx.control_name(ParlorAction::Primary));
        let t = ctx.control_name(ParlorAction::Secondary);
        let r = ctx.control_name(ParlorAction::Repeat);
        let s = ctx.control_name(ParlorAction::Status);
        let h = ctx.control_name(ParlorAction::Hint);
        let size = match &self.mode {
            Mode::Level { level, .. } => level_size(*level),
            _ => self.round.show.len(),
        };
        let rule = match &self.mode {
            Mode::Challenge(run) => format!(" {}", run.plan.rule()),
            Mode::Independence(_) => format!(
                " Independence night: four acts, each played whatever it scores. On an act that clears, every boom matched near-perfectly adds {} bonus points.",
                rules::SIGNATURE_BONUS
            ),
            _ => String::new(),
        };
        let aim = if self.mode.placed(&self.round.show) {
            format!(" {}", aim_line(ctx))
        } else {
            " Every firework in this level goes up in the center.".to_string()
        };
        // One shot refuses a second hearing and the hint, so neither is offered.
        let extras = if self.mode.twist() == Some(Twist::OneShot) {
            format!(
                "{} repeats the instructions, {s} gives the status, twice for more.",
                sentence_start(&r)
            )
        } else {
            format!(
                "{} plays the show again, {r} repeats the instructions, {s} gives the status, twice for more, {h} describes the show in words for {HINT_COST} points.",
                sentence_start(&t)
            )
        };
        format!(
            "{}: {size} to match.{rule} {p} plays the show after a count of three, then, on your turn, starts your own count-in; the go is time zero. {KEYS_LINE}{aim} Backspace takes back the last firework, Enter finishes and scores. {} {extras} Escape pauses.",
            self.mode.title(),
            wheel_line(ctx, true),
        )
    }

    /// F1 says the keys for this moment first; F1 again says every key.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let p = ctx.control_name(ParlorAction::Primary);
        let t = ctx.control_name(ParlorAction::Secondary);
        // A challenge's twist is named; its rules are in the second layer,
        // with every key.
        let twist = match &self.mode {
            Mode::Challenge(run) => run
                .plan
                .twist
                .map(|t| format!(" Twist: {}.", t.name()))
                .unwrap_or_default(),
            _ => String::new(),
        };
        let now = match self.round.phase {
            Phase::Ready => format!(
                "{}.{twist} {} plays the show.",
                self.mode.title(),
                sentence_start(&p)
            ),
            Phase::Listening => "The show is playing.".to_string(),
            Phase::YourTurn if self.mode.twist() == Some(Twist::OneShot) => {
                format!("Your turn. {} counts in, then launch.", sentence_start(&p))
            }
            Phase::YourTurn => format!(
                "Your turn. {} counts in, then launch. {} plays the show again.",
                sentence_start(&p),
                sentence_start(&t)
            ),
            Phase::Playing => {
                let aim = if self.mode.placed(&self.round.show) {
                    format!(" {}", aim_line(ctx))
                } else {
                    String::new()
                };
                format!(
                    "{}{aim} Backspace takes back the last firework. {}",
                    keys_line(ctx),
                    self.finish_line(ctx)
                )
            }
        };
        if self.help_phase.replace(self.round.phase) != Some(self.round.phase) {
            self.help_key.reset();
        }
        let short = format!("{now} F1 again for every key.");
        let full = self.help(ctx);
        self.help_key.speak(ctx, vec![short, full]);
    }
}

mod visuals;
