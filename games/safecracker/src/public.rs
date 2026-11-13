// Safecracker: Turn the dial and listen for the tumblers to open safe after safe.
//
// An original puzzle for the ear. The player turns a combination dial a
// number at a time and listens: each number clicks, every tenth clicks
// twice, and when the dial reaches the current wheel's number the tumbler
// gives a deep clunk. Setting the right number moves on to the next wheel;
// a wrong set springs back and costs a strike. Ten safes grow from two
// wheels to five, with false gates, building noises, quieter clunks, the
// real dial's direction rule and a night guard you must keep still for.
// Best times are kept per safe, and cracking the third safe opens time
// attack.

pub mod challenge;
pub mod crack;
pub mod events;
pub mod heist;
pub mod progress;
pub mod rules;
pub mod sound;
mod soundcheck;
pub mod tutorial;

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::screens::SettingsRow;
use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{pace, Category, Challenge, Challenges, GameInfo, Parlor, Period};

pub use crack::Crack;
pub use heist::{ChallengeHost, ChallengesMenu, Jobs, CHALLENGES_NAME};
pub use progress::Progress;
pub use soundcheck::SoundCheck;

use rules::{count_word, safe_spec, time_words, Mode, ATTACK_OPENS, SAFES};

/// The game's id: its sounds live under `sounds/safecracker/`.
pub const ID: &str = "safecracker";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a safe returns to.
pub const TITLE_NAME: &str = "safecracker";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Safecracker",
    category: Category::Puzzle,
    blurb: "Turn the dial to find each tumbler and open safe after safe.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(events::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <Safecracker as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[
            parlor_core::story::PartOfDay::Night,
            parlor_core::story::PartOfDay::SmallHours,
        ],
        seasons: &[],
    },
};

/// Opens the current `period` challenge, from the launcher's list of
/// today's challenges: its briefing, then the job. Leaving it returns to
/// the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// "best score 300, relaxed best score 250", leaving out a board with no
/// score, or "no score yet".
fn best_score(ctx: &mut Ctx<Parlor>, board: &str) -> String {
    pace::bests(ctx, ID, board, "best score").unwrap_or_else(|| "no score yet".to_string())
}

/// "best time 1 minute, relaxed best time 2 minutes", leaving out one
/// with no time.
fn best_times(word: &str, best: Option<f64>, relaxed: Option<f64>) -> Option<String> {
    pace::both(word, best.map(time_words), relaxed.map(time_words))
}

fn best_time(best: Option<f64>, relaxed: Option<f64>) -> String {
    best_times("best time", best, relaxed).unwrap_or_else(|| "not yet cracked".to_string())
}

/// A safe's features, in a few words.
pub fn safe_summary(n: u32) -> String {
    let spec = safe_spec(n);
    let mut parts = vec![format!(
        "{} wheels",
        sentence_start(count_word(spec.wheels))
    )];
    if spec.decoys > 0 {
        parts.push(format!(
            "{} false gate{} a wheel",
            spec.decoys,
            if spec.decoys == 1 { "" } else { "s" }
        ));
    }
    if spec.alternate {
        parts.push("direction rule".to_string());
    }
    if spec.guard.is_some() {
        parts.push("a night guard".to_string());
    }
    if spec.noises.is_some() {
        parts.push("building noises".to_string());
    }
    parts.push(match spec.hints {
        1 => "1 hint".to_string(),
        h => format!("{h} hints"),
    });
    parts.join(", ")
}

/// An event safe's features, in a few words.
pub fn event_summary(mode: Mode) -> String {
    match mode {
        Mode::Unlucky => "Four wheels, one always 13, a false gate a wheel, and a black cat who yowls at 13".to_string(),
        Mode::Blackout => "Three wheels, a false gate a wheel, and the power cut, so count from zero".to_string(),
        _ => String::new(),
    }
}

/// Records: every best time and score, then the challenges, a page at a
/// time.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let progress = Progress::load(ctx);
    let mut pages = vec![format!(
        "Records. Safes cracked: {} of {SAFES}. Safes opened in all: {}. Practice wheels set: {}.",
        progress.cracked, progress.opened, progress.practice_sets
    )];
    let lines: Vec<String> = (1..=SAFES)
        .map(|n| {
            let spec = safe_spec(n);
            let times = best_times("best time", progress.best_time(n), progress.relaxed_time(n));
            let time = match times {
                Some(t) => format!("{t}, {}", best_score(ctx, &format!("safe {n}"))),
                None if n <= progress.open_safes() => "not yet cracked".to_string(),
                None => "locked".to_string(),
            };
            format!("Safe {n}, {}: {time}.", spec.name)
        })
        .collect();
    pages.push(lines[..5].join(" "));
    pages.push(lines[5..].join(" "));
    if progress.attack_open() {
        let attack: Vec<String> = (1..=progress.cracked.min(SAFES))
            .map(|n| {
                match best_times(
                    "best",
                    progress.attack_time(n),
                    progress.relaxed_attack_time(n),
                ) {
                    Some(t) => format!(
                        "safe {n}, {t}, {}",
                        best_score(ctx, &format!("time attack {n}"))
                    ),
                    None => format!("safe {n}, not yet beaten"),
                }
            })
            .collect();
        // Five a page, like the safes.
        for (i, chunk) in attack.chunks(5).enumerate() {
            let list = chunk.join("; ");
            pages.push(if i == 0 {
                format!("Time attack: {list}.")
            } else {
                format!("{}.", sentence_start(&list))
            });
        }
    } else {
        pages.push(format!(
            "Time attack opens once you crack safe {ATTACK_OPENS}."
        ));
    }
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            hearken::text::end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    let bonus: Vec<String> = [
        (Mode::Unlucky, events::UNLUCKY_NAME),
        (Mode::Blackout, events::BLACKOUT_NAME),
    ]
    .into_iter()
    .filter_map(|(mode, name)| {
        pace::bests(ctx, ID, &mode.board(), "best score").map(|b| format!("{name}, {b}."))
    })
    .collect();
    if !bonus.is_empty() {
        pages.push(format!("Bonus safes. {}", bonus.join(" ")));
    }
    pages.push(format!("Challenges. {}", challenges.join(" ")));
    pages
}

/// The game on the shared title builder: its rows, pages and screens.
pub struct Safecracker;

/// The game's title screen.
pub type Title = menus::Title<Safecracker>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(Safecracker)
}

impl TitleGame for Safecracker {
    const ID: &'static str = ID;
    const NAME: &'static str = "Safecracker";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![pace::row(ID)]
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let progress = Progress::load(ctx);
        let next = progress.next_safe();
        let spec = safe_spec(next);
        let play_help = format!(
            "{}. {}, {}.",
            safe_summary(next),
            sentence_start(&best_time(
                progress.best_time(next),
                progress.relaxed_time(next)
            )),
            best_score(ctx, &format!("safe {next}"))
        );
        let attack = if progress.attack_open() {
            MenuItem::new("Time attack", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(SafeSelect::new(true))
            })
            .help("Any safe you have cracked, against the clock, for its best time attack time.")
        } else {
            MenuItem::new(
                labels::locked("Time attack"),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.say(format!(
                        "Time attack opens once you crack safe {ATTACK_OPENS}, {}.",
                        safe_spec(ATTACK_OPENS).name
                    ))
                },
            )
            .select_sound(None)
            .help(format!("Crack safe {ATTACK_OPENS} to open time attack."))
        };
        let today = parlor_core::challenges::today(ctx);
        let open = events::open_modes(today);
        let events: Vec<TitleRow<Self>> = events::MODES
            .into_iter()
            .map(|mode| {
                let name = events::name(mode);
                if !open.contains(&mode) {
                    // Closed tonight: the row keeps its place and says
                    // when it opens.
                    return MenuItem::new(
                        labels::locked(name),
                        move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                            let today = parlor_core::challenges::today(ctx);
                            ctx.say(events::locked_line(mode, today))
                        },
                    )
                    .select_sound(None)
                    .help(events::locked_help(mode, today));
                }
                let help = format!(
                    "A bonus safe tonight. {}. {}.{}",
                    event_summary(mode),
                    sentence_start(&best_score(ctx, &mode.board())),
                    events::stays(mode, today)
                );
                MenuItem::new(name, move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.push(Crack::new(mode))
                })
                .help(help)
            })
            .collect();
        let mut rows = vec![MenuItem::new(
            format!("{}, safe {next}: {}", labels::PLAY, spec.name),
            move |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(Crack::new(Mode::Safe(next))),
        )
        .help(play_help)];
        rows.extend([
            attack,
            MenuItem::new("Safe select", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(SafeSelect::new(false))
            })
            .help(format!(
                "Play any safe you have reached. {} of {SAFES} open.",
                progress.open_safes()
            )),
        ]);
        rows.extend(events);
        rows
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new("Practice dial", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(Crack::new(Mode::Practice))
            })
            .help("Endless single wheels with nothing to lose."),
        )
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your best times and scores for every safe, and your challenge results.".to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "A daily job, a weekly vault and a monthly heist, the same for every player. {}",
            daily.resets()
        )
    }

    /// A player who has opened a safe is no longer new, tutorial or not.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        let progress = Progress::load(ctx);
        !menus::tutorial_read(ctx, ID)
            && progress.cracked == 0
            && progress.opened == 0
            && progress.practice_sets == 0
    }
}

/// Choose a safe: any reached safe, or for time attack any cracked one.
pub struct SafeSelect {
    menu: MenuCore<Self, Parlor>,
    attack: bool,
}

impl SafeSelect {
    pub fn new(attack: bool) -> Self {
        let title = if attack { "Time attack" } else { "Safe select" };
        SafeSelect {
            menu: MenuCore::new(title)
                .with_intro_help("Up and Down choose a safe, Enter plays it, Escape goes back."),
            attack,
        }
    }
}

impl Menu for SafeSelect {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        if self.attack {
            "safecracker_attack"
        } else {
            "safecracker_safes"
        }
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let progress = Progress::load(ctx);
        let open = if self.attack {
            progress.cracked.min(SAFES)
        } else {
            progress.open_safes()
        };
        let mut items: Vec<MenuItem<Self, Parlor>> = Vec::new();
        for n in 1..=SAFES {
            let spec = safe_spec(n);
            if n > open {
                let why = if self.attack {
                    format!("Crack safe {n} to race it against the clock.")
                } else {
                    format!("Crack safe {} to open safe {n}.", n - 1)
                };
                items.push(
                    MenuItem::new(
                        format!("Safe {n}, locked"),
                        move |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.say(why.clone()),
                    )
                    .select_sound(None),
                );
                continue;
            }
            let (mode, time, relaxed, board) = if self.attack {
                (
                    Mode::TimeAttack(n),
                    progress.attack_time(n),
                    progress.relaxed_attack_time(n),
                    format!("time attack {n}"),
                )
            } else {
                (
                    Mode::Safe(n),
                    progress.best_time(n),
                    progress.relaxed_time(n),
                    format!("safe {n}"),
                )
            };
            let label = match best_times("best", time, relaxed) {
                Some(t) => format!("Safe {n}, {}, {t}", spec.name),
                None => format!("Safe {n}, {}", spec.name),
            };
            let mut help = format!(
                "{}. {}.",
                safe_summary(n),
                sentence_start(&best_score(ctx, &board))
            );
            if self.attack {
                help.push_str(&format!(
                    " {} on the clock.",
                    sentence_start(&time_words(spec.limit))
                ));
            }
            items.push(
                MenuItem::new(label, move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.push(Crack::new(mode))
                })
                .help(help),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Safecracker title."),
        );
        items
    }
}
