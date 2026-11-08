//! How the meal went, what it scored, and where to go next, on the shared
//! results menu.

use hearken::menu::Menu;
use hearken::text::{end_sentence, sentence_start};
use hearken::{Bus, Ctx, InputEvent, State};
use parlor_core::menus::{labels, Back, Results};
use parlor_core::{Parlor, ParlorAction, Records, ScoreResult};

use crate::kitchen::{Kitchen, Mode};
use crate::progress::{self, Progress};
use crate::rules::{course, sentence, stars_phrase, Grade, Outcome, COURSES};
use crate::sound;
use crate::thanksgiving::{self, Keepsakes, Kept};
use crate::{ID, TITLE_NAME};

fn record_line(result: ScoreResult, what: &str) -> String {
    match result {
        ScoreResult::First => format!("First score saved for {what}."),
        ScoreResult::NewBest { previous } => format!("New best for {what}, up from {previous}."),
        ScoreResult::Short { best } => format!("Your best for {what} is {best}."),
    }
}

/// "Perfect" for a dish quality, for the breakdown.
fn quality_word(q: u32) -> &'static str {
    match Grade::for_score(q) {
        Grade::Perfect => "perfect",
        Grade::Good => "good",
        // Late, not "overdone": the whipped cream never saw the heat.
        Grade::Late => "late",
        Grade::Ruined => "ruined",
    }
}

/// Each dish again, with its station, for the second results page.
pub fn dish_page(mode: &Mode, outcome: &Outcome, points: u64) -> String {
    let dishes: Vec<String> = outcome
        .qualities
        .iter()
        .map(|(k, q)| format!("{}, {}, quality {q}", sentence(k.name()), k.station().on()))
        .collect();
    format!(
        "{}. {points} points, average {}, {}. {}.",
        mode.title(),
        outcome.average,
        stars_phrase(outcome.stars),
        dishes.join(". ")
    )
}

/// The shared results menu after a meal, with two things of the kitchen's
/// own: it waits for the dinner bell's line instead of cutting it off, and
/// the repeat key reads the results again while the status key reads each
/// dish. Everything else is the shared menu's.
pub struct Served {
    results: Results,
    summary: String,
    dishes: String,
}

impl Served {
    pub fn new(results: Results, summary: String, dishes: String) -> Served {
        Served {
            results,
            summary,
            dishes,
        }
    }
}

impl State<Parlor> for Served {
    fn name(&self) -> &'static str {
        Menu::name(&self.results)
    }
    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        let results = &mut self.results;
        ctx.queued_speech(|ctx| Menu::enter(results, ctx));
    }
    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        Menu::exit(&mut self.results, ctx)
    }
    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        Menu::resume(&mut self.results, ctx)
    }
    fn resume_quiet(&mut self, ctx: &mut Ctx<Parlor>) {
        Menu::resume_quiet(&mut self.results, ctx)
    }
    fn resume_with(&mut self, ctx: &mut Ctx<Parlor>, result: &str) {
        Menu::resume_with(&mut self.results, ctx, result)
    }
    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        let action = if event.is_repeat() {
            None
        } else {
            ctx.action_for(event)
        };
        match action {
            Some(ParlorAction::Repeat) => {
                ctx.say(self.summary.clone());
                let current = self.results.current_text(ctx);
                ctx.say_queued(current);
            }
            Some(ParlorAction::Status) => ctx.say(self.dishes.clone()),
            _ => Menu::handle_input(&mut self.results, ctx, event),
        }
    }
    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        Menu::update(&mut self.results, ctx, dt)
    }
    fn lines(&self, ctx: &Ctx<Parlor>) -> Vec<String> {
        Menu::lines(&self.results, ctx)
    }
    /// F1: the row's help, then how to hear the results again, naming the
    /// player's own controls.
    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let keys = format!(
            "{} repeats the results, {} reads each dish again.",
            sentence_start(&ctx.control_name(ParlorAction::Repeat)),
            ctx.control_name(ParlorAction::Status)
        );
        match self.results.current_help(ctx) {
            Some(row) => format!("{} {keys}", end_sentence(&row)),
            None => format!("The meal's results. {keys}"),
        }
    }
}

/// Scores the meal: saves bests, stars and progress, plays the stars and a
/// stinger, and builds the results menu.
pub fn score(
    mode: Mode,
    outcome: Outcome,
    by_ear: bool,
    hints: u32,
    ctx: &mut Ctx<Parlor>,
) -> Served {
    score_paced(mode, outcome, by_ear, hints, false, ctx)
}

/// Scores using the pace captured when cooking began, not a changed setting.
pub fn score_paced(
    mode: Mode,
    outcome: Outcome,
    by_ear: bool,
    hints: u32,
    relaxed: bool,
    ctx: &mut Ctx<Parlor>,
) -> Served {
    let relaxed = relaxed && matches!(mode, Mode::Course(_));
    let o = &outcome;
    let passed = o.passed();
    let family = match &mode {
        Mode::Thanksgiving { year } => Some(thanksgiving::family(*year)),
        _ => None,
    };
    let favourite = family.as_ref().map_or(0, |f| f.bonus(o));
    let points = o.points(by_ear) + favourite;
    let mut progress = Progress::load(ctx);
    let was_feast_served = progress.helpings_open();
    let mut summary = String::new();
    let mut total = 0;
    let mut feast = false;
    let dishes: Vec<String> = o
        .qualities
        .iter()
        .map(|(kind, q)| format!("{} {q}, {}", sentence(kind.name()), quality_word(*q)))
        .collect();
    let head = if passed {
        format!("Served. {}.", sentence(&stars_phrase(o.stars)))
    } else {
        "The guests push their plates away. No stars.".to_string()
    };
    summary.push_str(&format!(
        "{head} Average quality {}. {}.",
        o.average,
        dishes.join(". ")
    ));
    if o.ruined > 0 || o.fumbles > 0 {
        summary.push_str(&format!(
            " {} ruined, {} fumbled.",
            steps(o.ruined),
            steps(o.fumbles)
        ));
    }
    summary.push_str(&format!(" {points} points"));
    if by_ear {
        summary.push_str(", with the by-ear bonus");
    }
    summary.push('.');
    if let Some(f) = &family {
        let quality = favourite / 10;
        summary.push_str(&format!(
            " {} loved the {}: quality {quality}, counted twice.",
            f.guests[0],
            f.favourite.name()
        ));
    }
    if hints > 0 {
        summary.push_str(&format!(
            " {} used.",
            if hints == 1 {
                "1 hint".to_string()
            } else {
                format!("{hints} hints")
            }
        ));
    }
    match &mode {
        Mode::Course(n) => {
            if relaxed {
                summary
                    .push_str(" Relaxed pace. This score is kept separately from standard pace.");
            }
            let r = Records::record(ctx, ID, &progress::paced_course_board(*n, relaxed), points);
            summary.push_str(&format!(" {}", record_line(r, &format!("course {n}"))));
            let opened = passed && !progress.served(*n);
            progress.earn(*n, o.stars);
            if passed {
                if *n < COURSES {
                    if opened {
                        summary.push_str(&format!(
                            " Course {} is open: {}.",
                            n + 1,
                            course(n + 1).name
                        ));
                    }
                } else {
                    feast = true;
                    summary.push_str(" The whole feast is on the table. Happy Thanksgiving!");
                    if !was_feast_served {
                        summary.push_str(" Second helpings is open on the title.");
                    }
                }
            } else {
                summary.push_str(" One star serves the course: try again.");
            }
        }
        Mode::Helpings {
            round,
            total: before,
        } => {
            total = before + if passed { points } else { 0 };
            if passed {
                summary.push_str(&format!(" Round {round} served. Run total {total}."));
                if *round > progress.best_rounds {
                    progress.best_rounds = *round;
                }
            } else {
                let rounds = round - 1;
                summary.push_str(&format!(
                    " Second helpings ends after {} served, {total} points.",
                    if rounds == 1 {
                        "1 round".to_string()
                    } else {
                        format!("{rounds} rounds")
                    }
                ));
            }
            let r = Records::record(ctx, ID, progress::HELPINGS_BOARD, total);
            if !passed || matches!(r, ScoreResult::NewBest { .. } | ScoreResult::First) {
                summary.push_str(&format!(" {}", record_line(r, "second helpings")));
            }
        }
        Mode::Thanksgiving { year } => {
            feast = passed;
            Records::record(ctx, ID, thanksgiving::BOARD, points);
            let mut keep = Keepsakes::load(ctx);
            let kept = keep.keep(*year, points, o.stars, (favourite / 10) as u32);
            keep.store(ctx);
            let line = match kept {
                Kept::First => format!("Your keepsake for {year} is saved."),
                Kept::Better { previous } => {
                    format!("A new best for {year}, up from {previous}.")
                }
                Kept::Short { best } => format!("Your best for {year} is {best}."),
            };
            let toast = family.as_ref().map_or("", |f| f.toast);
            summary.push_str(&format!(" {line} {toast} Happy Thanksgiving."));
        }
        // A challenge meal is scored by `challenge::finish` instead.
        Mode::Challenge(_) => {}
    }
    progress.store(ctx);

    if let Some(key) = sound::stars(o.stars) {
        ctx.audio.play(key, Bus::UI);
    }
    let stinger = if feast {
        sound::FEAST
    } else if passed {
        sound::WIN
    } else {
        sound::LOSE
    };
    ctx.audio.set_bed(None);
    ctx.audio.play(stinger, Bus::MUSIC);

    let dishes = dish_page(&mode, o, points);
    let pages = vec![summary.clone(), dishes.clone()];
    let results = Results::new(summary.clone(), pages, Back::Title(TITLE_NAME));
    let results = match mode {
        Mode::Course(n) => {
            let mut results = results;
            if passed && n < COURSES {
                results = results.next(
                    format!("Next course: {}", course(n + 1).name),
                    course(n + 1).blurb,
                    move |ctx| ctx.replace(Kitchen::new(Mode::Course(n + 1))),
                );
            }
            if passed && n >= COURSES && progress.helpings_open() {
                results = results.next(
                    "Second helpings",
                    "An endless run of random meals, faster every round.",
                    |ctx| ctx.replace(Kitchen::new(Mode::Helpings { round: 1, total: 0 })),
                );
            }
            results.again(
                if passed {
                    labels::PLAY_AGAIN
                } else {
                    labels::TRY_AGAIN
                },
                format!(
                    "Course {n} again, from the top. Your best score and star rating are kept."
                ),
                move |ctx| ctx.replace(Kitchen::new(Mode::Course(n))),
            )
        }
        Mode::Helpings { round, .. } if passed => results.next(
            "Next round",
            "Another random meal, faster. The run goes on until a meal earns no stars.",
            move |ctx| {
                ctx.replace(Kitchen::new(Mode::Helpings {
                    round: round + 1,
                    total,
                }))
            },
        ),
        Mode::Helpings { .. } => results.again(
            labels::TRY_AGAIN,
            "A new second-helpings run from round 1.",
            |ctx| ctx.replace(Kitchen::new(Mode::Helpings { round: 1, total: 0 })),
        ),
        Mode::Thanksgiving { year } => results.again(
            if passed {
                labels::PLAY_AGAIN
            } else {
                labels::TRY_AGAIN
            },
            "The whole dinner again, from the top. Your best for the year is kept.",
            move |ctx| ctx.replace(Kitchen::new(Mode::Thanksgiving { year })),
        ),
        Mode::Challenge(_) => results,
    };
    Served::new(results, summary, dishes)
}

fn steps(n: u32) -> String {
    if n == 1 {
        "1 step".to_string()
    } else {
        format!("{n} steps")
    }
}
