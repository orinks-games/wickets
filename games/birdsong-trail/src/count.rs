//! The migration count: the special walk of the spring and autumn
//! equinoxes, and the monthly challenge. Flocks pass overhead, each bird
//! calling once, and you count each flock.

use hearken::text::sentence_start;
use hearken::{Ctx, InputEvent, Key, State, Vec3};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{challenges, Challenge, Pages, Parlor, ParlorAction, Period, Records};

use crate::birds::Note;
use crate::challenge::{self, CHALLENGES_NAME};
use crate::guide::{self, Guide};
use crate::rules::{best_possible, run_seed, Count, CountEv, CountPhase};
use crate::walk::END_PAUSE;
use crate::{birds::Kind, sound, HELP, ID, TITLE_NAME};

/// The count screen's `State::name`.
pub const COUNT_NAME: &str = "birdsong_trail_count";
/// The records board of the migration count: the best score.
pub const COUNT_BOARD: &str = "count";

pub struct CountScreen {
    pub count: Count,
    challenge: Option<Challenge>,
    last: String,
    asked_once: bool,
    ending: Option<f64>,
    new_birds: u32,
}

impl CountScreen {
    /// Today's count: north at the spring equinox, south in autumn.
    pub fn today(ctx: &mut Ctx<Parlor>) -> CountScreen {
        let today = challenges::today(ctx);
        let north = today.month <= 6;
        CountScreen::new(Count::equinox(north, run_seed()))
    }

    pub fn new(count: Count) -> CountScreen {
        CountScreen {
            count,
            challenge: None,
            last: String::new(),
            asked_once: false,
            ending: None,
            new_birds: 0,
        }
    }

    pub fn challenge(ch: Challenge, count: Count) -> CountScreen {
        let mut s = CountScreen::new(count);
        s.challenge = Some(ch);
        s
    }

    fn name(&self) -> String {
        match &self.challenge {
            Some(ch) => challenge::name(ch.period).to_string(),
            None if self.count.north => "Spring migration".to_string(),
            None => "Autumn migration".to_string(),
        }
    }

    fn say(&mut self, ctx: &mut Ctx<Parlor>, text: impl Into<String>) {
        let text = text.into();
        self.last = text.clone();
        ctx.say(text);
    }

    pub fn controls(ctx: &Ctx<Parlor>) -> String {
        let k = |a| ctx.control_name(a);
        format!(
            "Each bird calls once as it flies over: count the calls. When the flock has passed, {} and {} set your count and {} gives it. {} plays the flock again before you answer.",
            k(ParlorAction::Up),
            k(ParlorAction::Down),
            k(ParlorAction::Primary),
            sentence_start(&k(ParlorAction::Repeat)),
        )
    }

    pub fn status(&self) -> String {
        let c = &self.count;
        let n = c.flocks.len();
        let at = (c.at + 1).min(n);
        let mut line = format!("{}, flock {at} of {n}, {} points", self.name(), c.score());
        if c.phase == CountPhase::Asking {
            line.push_str(&format!(", your count {}", c.guess));
        }
        line.push('.');
        line
    }

    fn voice(&mut self, ctx: &mut Ctx<Parlor>, evs: Vec<CountEv>) {
        let mut lines: Vec<String> = Vec::new();
        for ev in evs {
            match ev {
                CountEv::Coming(_) => {
                    sound::play_vol(ctx, sound::WINGS, 0.7);
                    lines.push("A flock is coming.".to_string());
                }
                CountEv::Call { species, pos } => {
                    sound::bird_note(
                        ctx,
                        species,
                        Note::Mid,
                        Some(Vec3::new(pos.0, pos.1, pos.2)),
                    );
                }
                CountEv::Ask => {
                    if self.asked_once {
                        lines.push(format!("How many? {}.", self.count.guess));
                    } else {
                        self.asked_once = true;
                        let k = |a| ctx.control_name(a);
                        lines.push(format!(
                            "How many? {} and {} set your count, {} gives it. Count: {}.",
                            sentence_start(&k(ParlorAction::Up)),
                            k(ParlorAction::Down),
                            k(ParlorAction::Primary),
                            self.count.guess
                        ));
                    }
                }
                CountEv::Guess(n) => lines.push(format!("{n}.")),
                CountEv::Judged {
                    size,
                    guess,
                    points,
                } => {
                    let species = self.count.flocks[self.count.at - 1].species;
                    let mut line = match points {
                        3 => {
                            sound::play(ctx, sound::FOUND);
                            format!(
                                "{size}, exactly right. A flock of {size} {}.",
                                plural(species.name)
                            )
                        }
                        1 => {
                            sound::play(ctx, sound::AGAIN);
                            format!(
                                "Close: there were {size}, and you counted {guess}. {}.",
                                sentence_start(&plural(species.name))
                            )
                        }
                        _ => {
                            sound::play(ctx, sound::AGAIN);
                            format!(
                                "There were {size}, and you counted {guess}. {}.",
                                sentence_start(&plural(species.name))
                            )
                        }
                    };
                    if points == 3 {
                        let mut g = Guide::load(ctx);
                        let learned = g.learn(species, 0, guide::special_where(Kind::Flock));
                        g.store(ctx);
                        if learned.new_bird {
                            self.new_birds += 1;
                            sound::play(ctx, sound::NEW_SONG);
                            line.push_str(" New in your field guide.");
                        }
                    }
                    lines.push(line);
                }
                CountEv::Finished => {
                    sound::play(ctx, sound::ARRIVE);
                    lines.push(format!(
                        "The last flock has gone. {} of {} points.",
                        self.count.score(),
                        best_possible(self.count.flocks.len())
                    ));
                    self.ending = Some(END_PAUSE);
                }
            }
        }
        if !lines.is_empty() {
            let text = lines.join(" ");
            self.say(ctx, text);
        }
    }

    pub fn summary(&self) -> String {
        let c = &self.count;
        format!(
            "{}: {} of {} points, {} of {} flocks exactly right.",
            self.name(),
            c.score(),
            best_possible(c.flocks.len()),
            c.exact(),
            c.flocks.len()
        )
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_bed(None);
        self.count.finished = true;
        let summary = self.summary();
        let tally: Vec<String> = self
            .count
            .flocks
            .iter()
            .zip(&self.count.guesses)
            .map(|(f, g)| format!("{} {}, you counted {g}", f.size, plural(f.species.name)))
            .collect();
        let mut pages = vec![summary.clone()];
        if !tally.is_empty() {
            pages.push(format!("{}.", sentence_start(&tally.join(". "))));
        }
        if let Some(ch) = self.challenge.take() {
            pages.extend(challenge::finish_count(ctx, &ch, &self.count));
            let summary = challenge::scored(&summary, challenge::count_points(&self.count));
            let mut results = Some(after_challenge(ch.period, summary, pages.clone()));
            ctx.replace(Pages::new(pages).then(move |ctx| {
                if let Some(r) = results.take() {
                    ctx.replace(r);
                }
            }));
            return;
        }
        let mut g = Guide::load(ctx);
        g.counts += 1;
        g.flocks_exact += self.count.exact();
        g.flock_birds += self
            .count
            .flocks
            .iter()
            .take(self.count.guesses.len())
            .map(|f| f.size)
            .sum::<u32>();
        g.store(ctx);
        let best = match Records::record(ctx, ID, COUNT_BOARD, self.count.score() as u64) {
            parlor_core::ScoreResult::First => "Your first migration count.".to_string(),
            parlor_core::ScoreResult::NewBest { previous } => {
                format!("A new best count, beating {previous}.")
            }
            parlor_core::ScoreResult::Short { best } => format!("Your best count is {best}."),
        };
        let new = match self.new_birds {
            0 => String::new(),
            1 => " 1 bird is new in your field guide.".to_string(),
            n => format!(" {n} birds are new in your field guide."),
        };
        pages.push(format!("{best}{new}"));
        let mut results = Some(
            Results::new(summary, pages.clone(), Back::Title(TITLE_NAME)).again(
                labels::PLAY_AGAIN,
                "Another count, with new flocks.",
                |ctx| {
                    let s = CountScreen::today(ctx);
                    ctx.replace(s)
                },
            ),
        );
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(r) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(r);
            }
        }));
    }
}

/// "Canada geese", "sandhill cranes", "whooper swans".
pub fn plural(name: &str) -> String {
    if let Some(stem) = name.strip_suffix("goose") {
        format!("{stem}geese")
    } else {
        format!("{name}s")
    }
}

/// The results menu after the monthly challenge.
pub fn after_challenge(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same flocks, straight away. Only your best result is kept.",
        move |ctx| {
            let ch = Challenge::current(ctx, ID, period);
            challenge::start(ctx, ch, true)
        },
    )
}

impl State<Parlor> for CountScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        COUNT_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        sound::bed(ctx, sound::BED_HILL);
        let n = self.count.flocks.len();
        let way = if self.count.north { "north" } else { "south" };
        let from = if self.count.north {
            "from behind you to ahead"
        } else {
            "from ahead of you to behind"
        };
        let intro = match &self.challenge {
            Some(_) => format!(
                "{}. You stand on a hill as {n} flocks fly over, {from}.",
                self.name()
            ),
            None => format!(
                "{}. On the equinox you stand on a hill, and {n} flocks fly over on their way {way}, {from}.",
                self.name()
            ),
        };
        self.say(ctx, intro);
        ctx.say_queued(Self::controls(ctx));
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.count.finished {
            return;
        }
        sound::bed(ctx, sound::BED_HILL);
        let s = self.status();
        self.say(ctx, format!("Back on the hill. {s}"));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.count.finished || self.ending.is_some() {
            return;
        }
        let action = ctx.action_for(event);
        let asking = self.count.phase == CountPhase::Asking;
        if event.is_repeat()
            && !(asking && matches!(action, Some(ParlorAction::Up | ParlorAction::Down)))
        {
            return;
        }
        if action.is_none() && event.pressed() == Some(Key::Escape) {
            let leave = if self.challenge.is_some() {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        let evs = match action {
            Some(ParlorAction::Up | ParlorAction::Right) if asking => self.count.adjust(1),
            Some(ParlorAction::Down | ParlorAction::Left) if asking => self.count.adjust(-1),
            Some(ParlorAction::Primary) if asking => self.count.confirm(),
            Some(ParlorAction::Repeat) if asking => {
                self.say(ctx, "Once more, overhead.");
                self.count.replay()
            }
            Some(ParlorAction::Status) => {
                let s = self.status();
                self.say(ctx, s);
                Vec::new()
            }
            Some(ParlorAction::Hint) => {
                let h = if asking {
                    let r = sentence_start(&ctx.control_name(ParlorAction::Repeat));
                    format!("Each bird calls once. Count the calls, not the wingbeats. {r} plays the flock again.")
                } else {
                    "Each bird calls once. Count the calls, not the wingbeats.".to_string()
                };
                self.say(ctx, h);
                Vec::new()
            }
            Some(ParlorAction::Repeat) => {
                let l = if self.last.is_empty() {
                    self.status()
                } else {
                    self.last.clone()
                };
                ctx.say(l);
                Vec::new()
            }
            Some(ParlorAction::Secondary) => {
                self.say(ctx, "The flocks are still coming. Escape pauses.");
                Vec::new()
            }
            Some(_) => {
                self.say(ctx, "Count the calls.");
                Vec::new()
            }
            None => Vec::new(),
        };
        self.voice(ctx, evs);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if let Some(left) = self.ending.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                self.ending = None;
                self.finish(ctx);
            }
            return;
        }
        let evs = self.count.update(dt);
        self.voice(ctx, evs);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let c = &self.count;
        vec![
            self.name(),
            format!(
                "Flock: {} of {}",
                (c.at + 1).min(c.flocks.len()),
                c.flocks.len()
            ),
            format!("Points: {}", c.score()),
            format!("Count: {}", c.guess),
        ]
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let now = if self.count.phase == CountPhase::Asking {
            "Flock passed. How many birds?"
        } else {
            "The next flock is on its way."
        };
        format!("{now} {}", Self::controls(ctx))
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

mod visuals_countscreen;
