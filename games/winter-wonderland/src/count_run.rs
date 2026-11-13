//! Midnight Countdown's screen: the intros, the music, and midnight.

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, State};
use parlor_core::{Parlor, ParlorAction};

use crate::mode::Mode;
use crate::night::{describe_call, Inst, Intro, Night, NightEvent, Stage, NIGHTS, PHRASES};
use crate::progress::{self, Progress};
use crate::sound::{self, cue, stinger};
use crate::{fresh_rng, pause, results};

/// Seconds of bells and fireworks before the results.
const CELEBRATION: f64 = 6.5;

/// One night of Midnight Countdown.
pub struct CountRun {
    mode: Mode,
    night: Night,
    told_early: bool,
    /// Seconds into the midnight celebration, once it has begun.
    celebrating: Option<f64>,
    popped: bool,
    /// Run by the tutorial: midnight starts the countdown over instead of
    /// ending in results.
    pub lesson: bool,
}

impl CountRun {
    /// Night `level` in `mode`.
    pub fn new(mode: Mode, level: u32) -> CountRun {
        let practice = mode == Mode::Practice;
        CountRun {
            night: Night::new(level, practice, fresh_rng()),
            mode,
            told_early: false,
            celebrating: None,
            popped: false,
            lesson: false,
        }
    }

    /// The night under way, for the tutorial and tests.
    pub fn night(&self) -> &Night {
        &self.night
    }

    /// A run of a night already built, such as a challenge's.
    pub fn with_night(mode: Mode, night: Night) -> CountRun {
        CountRun {
            night,
            mode,
            told_early: false,
            celebrating: None,
            popped: false,
            lesson: false,
        }
    }

    /// The fanfare at midnight.
    fn fanfare(&self) -> &'static str {
        sound::WIN
    }

    fn key(ctx: &Ctx<Parlor>, inst: Inst) -> String {
        ctx.control_name(match inst {
            Inst::Bell => ParlorAction::Primary,
            Inst::Drum => ParlorAction::Down,
            Inst::Horn => ParlorAction::Up,
        })
    }

    fn intro_text(&self, ctx: &Ctx<Parlor>, intro: Intro) -> String {
        let primary = ctx.control_name(ParlorAction::Primary);
        match intro {
            Intro::Bells if self.night.gala => format!(
                "Midnight Countdown: {}. Ten minutes to midnight, and the full orchestra is here from the first call. The band plays a one-bar call; answer it in the next bar, on the same beats. Bells are {primary}, the drum is {}, the horn is {}. The party starts at full energy; misses drain it. Press {primary} to start. A bar counts you in.",
                self.night.spec.name,
                Self::key(ctx, Inst::Drum),
                Self::key(ctx, Inst::Horn)
            ),
            Intro::Bells if self.mode == Mode::Free && self.night.spec.level > 1 => format!(
                "Night {}: {}. {} starts.",
                self.night.spec.level,
                self.night.spec.name,
                sentence_start(&primary)
            ),
            Intro::Bells => {
                let practice = if self.night.practice {
                    " This is practice: the party never fizzles, and nothing is saved."
                } else {
                    ""
                };
                format!(
                    "Midnight Countdown, night {}: {}. Ten minutes to midnight. The band plays a one-bar call on bells; answer it in the next bar, on the same beats, with {primary}. Good answers lift the party's energy; misses drain it.{practice} Press {primary} to start. A bar counts you in.",
                    self.night.spec.level,
                    self.night.spec.name
                )
            }
            Intro::Drums => format!(
                "Seven minutes to midnight. The drums join in. Answer a drum with {}. Bells are still {primary}. Press {primary} to play on.",
                Self::key(ctx, Inst::Drum)
            ),
            Intro::Horns => format!(
                "Four minutes to midnight. The horns join in. Answer a horn with {}. Press {primary} to play on.",
                Self::key(ctx, Inst::Horn)
            ),
            Intro::Orchestra => format!(
                "The final seconds. The full orchestra plays one long two-bar call with bells, drums and horns. Answer all of it, for double points, to ring in midnight. Press {primary} when you are ready."
            ),
        }
    }

    fn mapping(&self, ctx: &Ctx<Parlor>) -> String {
        let parts: Vec<String> = self
            .night
            .known()
            .iter()
            .map(|&i| format!("{} for the {}", Self::key(ctx, i), i.name()))
            .collect();
        format!(
            "Answer in the bar after the call, on the same beats: {}.",
            parts.join(", ")
        )
    }

    fn status(&self) -> String {
        let n = &self.night;
        let when = if n.phrase >= PHRASES {
            "The final seconds before midnight".to_string()
        } else {
            match n.minutes_left() {
                1 => "One minute to midnight".to_string(),
                m => format!("{m} minutes to midnight"),
            }
        };
        format!(
            "{when}. Party energy {} percent. Score {}, combo {}, times {}.",
            n.energy,
            n.score,
            n.combo,
            n.multiplier()
        )
    }

    fn play(&mut self, ctx: &mut Ctx<Parlor>, inst: Inst) {
        ctx.audio.play(sound::inst(inst), Bus::EFFECTS);
        let events = self.night.press(inst);
        self.after(ctx, events);
    }

    fn voice(&mut self, ctx: &mut Ctx<Parlor>, event: NightEvent) {
        match event {
            NightEvent::Tick { beat_in_bar, layer } => {
                let key = if beat_in_bar == 0 {
                    sound::TOCK
                } else {
                    sound::TICK
                };
                // The clock keeps time for the player: a cue, not music.
                ctx.audio.play(key, Bus::EFFECTS);
                if layer >= 2 && (beat_in_bar == 0 || (layer >= 4 && beat_in_bar == 2)) {
                    ctx.audio.play(sound::BASS, Bus::MUSIC);
                }
                if layer >= 3 && beat_in_bar % 2 == 0 {
                    ctx.audio.play(sound::CHORD, Bus::MUSIC);
                }
            }
            NightEvent::Call(inst) => {
                ctx.audio.play(sound::inst(inst), Bus::EFFECTS);
            }
            NightEvent::Hit { .. } => {}
            NightEvent::Wrong { .. } => {
                cue(ctx, sound::OOPS);
            }
            NightEvent::Miss(_) => {
                cue(ctx, sound::MISS);
            }
            NightEvent::Stray => {
                cue(ctx, sound::STRAY);
            }
            NightEvent::Early => {
                cue(ctx, sound::STRAY);
                if !self.told_early {
                    self.told_early = true;
                    ctx.say("Wait for the call to finish.");
                }
            }
            NightEvent::ComboUp(m) => {
                cue(ctx, sound::COMBO);
                ctx.say(format!("Times {m}."));
            }
            NightEvent::PhraseDone { clean, .. } => {
                if clean {
                    cue(ctx, sound::CLEAN);
                }
            }
            NightEvent::EnergyLow => {
                cue(ctx, sound::LOW);
                ctx.say("The party is fading!");
            }
            NightEvent::Intro(intro) => {
                let text = self.intro_text(ctx, intro);
                ctx.say(text);
            }
            NightEvent::Midnight { .. } if self.lesson => {
                cue(ctx, sound::TOLL);
                let level = self.night.spec.level;
                self.night = Night::new(level, true, fresh_rng());
                ctx.say(format!(
                    "Midnight, but the lesson is not over. The band starts again from ten minutes to midnight. {} starts the music.",
                    sentence_start(&ctx.control_name(ParlorAction::Primary))
                ));
            }
            NightEvent::Midnight { .. } => {
                self.celebrating = Some(0.0);
                cue(ctx, sound::TOLL);
                ctx.say("Midnight.");
            }
            NightEvent::Fizzled => {}
        }
    }

    fn after(&mut self, ctx: &mut Ctx<Parlor>, events: Vec<NightEvent>) {
        for e in events {
            self.voice(ctx, e);
        }
        if self.night.stage == Stage::Done(false) {
            self.finish(ctx, false);
        }
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>, won: bool) {
        let n = &self.night;
        let level = n.spec.level;
        let score = n.score;
        let stats = format!(
            "Accuracy {} percent, best combo {}: {} perfect, {} good, {} missed, {} wrong.",
            (n.accuracy() * 100.0).round(),
            n.best_combo,
            n.perfects,
            n.goods,
            n.misses,
            n.wrongs
        );
        let head = if won {
            format!("Happy New Year from {}!", n.spec.name)
        } else {
            stinger(ctx, sound::LOSE);
            let at = if n.phrase >= PHRASES {
                "in the final seconds before midnight".to_string()
            } else {
                match n.minutes_left() {
                    1 => "at one minute to midnight".to_string(),
                    m => format!("at {m} minutes to midnight"),
                }
            };
            format!("The party fizzled out {at}.")
        };
        match self.mode.clone() {
            Mode::Practice => {
                let pages = vec![format!(
                    "{head} Practice score {score}. {stats} Nothing is saved from practice."
                )];
                let summary = format!("Practice over. Score {score}");
                results::show(
                    ctx,
                    pages.clone(),
                    results::countdown(level, true, won, summary, pages),
                );
            }
            _ => {
                let line = progress::record(ctx, progress::COUNT_BOARD, score);
                progress::record(ctx, &progress::night_board(level), score);
                let mut extra = String::new();
                if won {
                    if level < NIGHTS && Progress::open_night(ctx, level + 1) {
                        extra = format!(
                            " Night {} is now open: {}.",
                            level + 1,
                            crate::night::NightSpec::new(level + 1).name
                        );
                    } else if level == NIGHTS && Progress::mark(ctx, |p| &mut p.all_nights) {
                        extra = " You have rung in midnight at every party.".to_string();
                    }
                }
                let pages = vec![format!("{head} Score {score}. {stats} {line}{extra}")];
                let summary = if won {
                    format!("Midnight. Score {score}")
                } else {
                    format!("The party fizzled. Score {score}")
                };
                results::show(
                    ctx,
                    pages.clone(),
                    results::countdown(level, false, won, summary, pages),
                );
            }
        }
    }

    fn celebrate(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let Some(t) = self.celebrating.as_mut() else {
            return;
        };
        *t += dt;
        let t = *t;
        if !self.popped && t >= 4.4 {
            self.popped = true;
            cue(ctx, sound::POP);
            cue(ctx, sound::CHEER);
            cue(ctx, sound::FIREWORKS);
            let fanfare = self.fanfare();
            stinger(ctx, fanfare);
        }
        if t >= CELEBRATION {
            self.celebrating = None;
            self.finish(ctx, true);
        }
    }
}

impl State<Parlor> for CountRun {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        "winter_wonderland_countdown"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        if let Stage::Waiting(intro) = self.night.stage {
            let text = self.intro_text(ctx, intro);
            ctx.say(text);
        }
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        match self.night.stage {
            Stage::Waiting(intro) => {
                let text = self.intro_text(ctx, intro);
                ctx.say(text);
            }
            Stage::Playing => {
                self.night.replay_call();
                let s = self.status();
                ctx.say(format!("Back to the party. {s} The call plays again."));
            }
            Stage::Done(_) => {
                let s = self.status();
                ctx.say(format!("Back to the party. {s}"));
            }
        }
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() {
            return;
        }
        let action = ctx.action_for(event);
        if self.celebrating.is_some() {
            if action.is_some() || event.pressed().is_some() {
                self.celebrating = None;
                if !self.popped {
                    self.popped = true;
                    let fanfare = self.fanfare();
                    stinger(ctx, fanfare);
                }
                self.finish(ctx, true);
            }
            return;
        }
        let Some(action) = action else {
            match event.pressed() {
                Some(Key::Escape) => pause(ctx, &self.mode),
                Some(_) => {
                    cue(ctx, sound::STRAY);
                    if matches!(self.night.stage, Stage::Waiting(_)) {
                        let primary = ctx.control_name(ParlorAction::Primary);
                        ctx.say(format!("{} starts the music.", sentence_start(&primary)));
                    }
                }
                None => {}
            }
            return;
        };
        match self.night.stage {
            Stage::Waiting(intro) => match action {
                ParlorAction::Primary => {
                    ctx.stop_speech();
                    self.night.begin();
                }
                ParlorAction::Repeat | ParlorAction::Secondary => {
                    let text = self.intro_text(ctx, intro);
                    ctx.say(text);
                }
                ParlorAction::Status => {
                    let s = self.status();
                    ctx.say(s);
                }
                ParlorAction::Hint => {
                    let m = self.mapping(ctx);
                    ctx.say(m);
                }
                ParlorAction::Down => {
                    ctx.audio.play(sound::inst(Inst::Drum), Bus::EFFECTS);
                }
                ParlorAction::Up => {
                    ctx.audio.play(sound::inst(Inst::Horn), Bus::EFFECTS);
                }
                ParlorAction::Left | ParlorAction::Right => {
                    cue(ctx, sound::STRAY);
                    let primary = ctx.control_name(ParlorAction::Primary);
                    ctx.say(format!("{} starts the music.", sentence_start(&primary)));
                }
            },
            Stage::Playing => match action {
                ParlorAction::Primary => self.play(ctx, Inst::Bell),
                ParlorAction::Down => self.play(ctx, Inst::Drum),
                ParlorAction::Up => self.play(ctx, Inst::Horn),
                ParlorAction::Status => {
                    let s = self.status();
                    ctx.say(s);
                }
                ParlorAction::Repeat => {
                    let call = describe_call(&self.night.last_call);
                    ctx.say(format!("The last call: {call}."));
                }
                ParlorAction::Hint => {
                    let m = self.mapping(ctx);
                    ctx.say(m);
                }
                ParlorAction::Left | ParlorAction::Right | ParlorAction::Secondary => {
                    cue(ctx, sound::STRAY);
                }
            },
            Stage::Done(_) => {}
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.celebrating.is_some() {
            self.celebrate(ctx, dt);
            return;
        }
        let events = self.night.update(dt);
        self.after(ctx, events);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let n = &self.night;
        let stage = match n.stage {
            Stage::Waiting(_) => "waiting",
            Stage::Playing if n.in_call() => "call",
            Stage::Playing => "answer",
            Stage::Done(true) => "midnight",
            Stage::Done(false) => "fizzled",
        };
        vec![
            format!(
                "Midnight Countdown, night {} ({}), {} minutes left, {stage}, energy {}, score {}, combo {}",
                n.spec.level,
                n.spec.name,
                n.minutes_left(),
                n.energy,
                n.score,
                n.combo
            ),
            format!("Last call: {}", describe_call(&n.last_call)),
        ]
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let n = |a| ctx.control_name(a);
        if self.celebrating.is_some() {
            return "Midnight, and the town is celebrating. Press any key to skip to the results.".to_string();
        }
        match self.night.stage {
            Stage::Waiting(_) => format!(
                "Midnight Countdown, between parts. {} starts the music. {} repeats what was just said, {} speaks the time and energy, {} lists your instruments. Try the drum with {} and the horn with {}. Escape pauses.",
                sentence_start(&n(ParlorAction::Primary)),
                sentence_start(&n(ParlorAction::Repeat)),
                n(ParlorAction::Status),
                n(ParlorAction::Hint),
                n(ParlorAction::Down),
                n(ParlorAction::Up)
            ),
            _ => format!(
                "Midnight Countdown. After the band's call, answer it in the next bar on the same beats. {} plays the bell, {} the drum, {} the horn. {} says the last call in words, {} speaks the time, energy and combo. Escape pauses.",
                sentence_start(&n(ParlorAction::Primary)),
                n(ParlorAction::Down),
                n(ParlorAction::Up),
                sentence_start(&n(ParlorAction::Repeat)),
                sentence_start(&n(ParlorAction::Status))
            ),
        }
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

mod visuals_countrun;
