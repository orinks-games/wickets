//! Cupid's Quiver: archery by ear over a rooftop sky.
//!
//! Paper hearts drift across the evening sky, each humming a soft note
//! from its place in stereo. The player turns the bow, holds to draw and
//! lets go at a full draw; a heart hit pops into a chime. Hearts come in
//! pairs that hum the same note, and hitting both one after the other is a
//! sweetheart pair. A round ends when the quiver is empty.
//!
//! Valentine's Day on the real calendar changes the sky: roses drift in
//! the week before, and on the day itself Valentine's night opens, with
//! golden hearts, a game for two and a keepsake for the year.

pub mod challenge;
pub mod field;
pub mod notes;
pub mod progress;
pub mod rules;
pub mod sound;
mod soundcheck;
pub mod tutorial;
pub mod valentine;

use hearken::menu::{Label, Menu, MenuItem};
use hearken::screens::{SettingsRow, SettingsScreen};
use hearken::text::{end_sentence, sentence_start};
use hearken::Ctx;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{
    Category, Challenge, Challenges, Date, GameInfo, Pages, Parlor, Period, Records,
};

pub use challenge::{ChallengeHost, ChallengesMenu, CHALLENGES_NAME};
pub use field::Field;
pub use progress::Progress;
pub use soundcheck::SoundCheck;

use rules::{Mode, ROOFTOPS};
use valentine::Gate;

/// The game's id: its sounds live under `sounds/cupids-quiver/`.
pub const ID: &str = "cupids-quiver";

/// The game's name.
pub const NAME: &str = "Cupid's Quiver";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a run returns to.
pub const TITLE_NAME: &str = "cupids_quiver";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: NAME,
    category: Category::Seasonal,
    blurb: "Archery over a rooftop sky: shoot drifting paper hearts, in pairs, for Valentine's Day.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <Cupid as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[parlor_core::story::PartOfDay::Evening],
        seasons: &[parlor_core::Season::Winter],
    },
};

/// The event the game holds on `date`: Valentine's night on the day,
/// roses in the sky in the week before.
pub fn event(date: Date) -> Option<parlor_core::Happening> {
    valentine::event(date)
}

/// Opens the current `period` challenge from the launcher's list.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// "best 840" or "not played yet".
pub fn best_words(ctx: &mut Ctx<Parlor>, mode: Mode) -> String {
    match mode.board().and_then(|b| Records::best(ctx, ID, b)) {
        Some(best) => format!("best {best}"),
        None => "not played yet".to_string(),
    }
}

/// The Records pages.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let progress = Progress::load(ctx);
    let mut lines = Vec::new();
    for mode in [Mode::Rooftops, Mode::Moonlight, Mode::Night, Mode::ForTwo] {
        let best = best_words(ctx, mode);
        lines.push(format!("{}: {best}.", mode.name()));
    }
    lines.push(format!(
        "Furthest on the rooftops: {} of {}.",
        progress.best_roofs,
        ROOFTOPS.len()
    ));
    if progress.moon_round > 0 {
        lines.push(format!(
            "Furthest moonlight round: {}.",
            progress.moon_round
        ));
    }
    let totals = format!(
        "In all: {}, {}, {} and {}, over {}. {} of {} love notes kept.",
        counted(progress.hearts, "heart", "hearts"),
        counted(progress.pairs, "sweetheart pair", "sweetheart pairs"),
        counted(progress.roses, "rose", "roses"),
        counted(progress.golden, "golden heart", "golden hearts"),
        counted(progress.runs, "evening", "evenings"),
        progress.notes_found(),
        notes::NOTES.len()
    );
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    vec![
        format!("Records. {}", lines.join(" ")),
        totals,
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

/// The love notes kept, as pages.
/// "1 heart", "4 hearts".
fn counted(n: u32, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

pub fn notes_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let progress = Progress::load(ctx);
    let n = progress.notes_found();
    let total = notes::NOTES.len();
    if n == 0 {
        return vec![format!(
            "Love notes: none kept yet, of {total}. A folded note drifts slowly over the rooftops now and then, with a flutter of paper. Hit it to keep it."
        )];
    }
    let mut pages = vec![format!(
        "Love notes: {n} of {total} kept, in the order found."
    )];
    for (i, &at) in progress.notes.iter().enumerate() {
        pages.push(format!("Note {}. {}", i + 1, notes::NOTES[at as usize]));
    }
    pages
}

/// The keepsake box, as pages.
pub fn keepsake_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let progress = Progress::load(ctx);
    let today = parlor_core::challenges::today(ctx);
    if progress.keepsakes.is_empty() {
        return vec![format!(
            "The keepsake box is empty. Play Valentine's night or the game for two on Valentine's Day, and a keepsake for that year goes in the box. {}",
            valentine::countdown(today)
        )];
    }
    let mut pages = vec![format!(
        "The keepsake box: {} Valentine's {}.",
        progress.keepsakes.len(),
        if progress.keepsakes.len() == 1 {
            "Day"
        } else {
            "Days"
        }
    )];
    for (year, k) in &progress.keepsakes {
        let mut parts = vec![format!("{year}: {}", valentine::keepsake_item(*year))];
        if let Some(best) = k.night_best {
            parts.push(format!("Valentine's night best {best}"));
        }
        if let Some(best) = k.two_best {
            parts.push(format!("for two best {best}"));
        }
        parts.push(match k.plays {
            1 => "played once".to_string(),
            n => format!("played {n} times"),
        });
        if k.pairs > 0 {
            parts.push(format!("{} sweetheart pairs", k.pairs));
        }
        if k.golden > 0 {
            parts.push(format!("{} golden hearts", k.golden));
        }
        pages.push(format!("{}.", parts.join(", ")));
    }
    pages
}

/// The Settings row that turns the spoken aim on and off.
pub const AIM_CALLS: &str = "Spoken aim";

/// The Settings row that turns the spoken name of each new target on and off.
pub const TARGET_CALLS: &str = "Spoken targets";

/// "Spoken aim: on".
pub fn call_out_label(name: &str, on: bool) -> String {
    format!("{name}: {}", if on { "on" } else { "off" })
}

/// A Settings row for one call-out: Enter, Left or Right flips it.
/// `quiet` picks the saved flag, which is true when the call-out is off.
fn call_out_row(
    id: &str,
    name: &'static str,
    quiet: fn(&mut Progress) -> &mut bool,
    help: &str,
) -> SettingsRow<Parlor> {
    let flip = move |ctx: &mut Ctx<Parlor>| {
        let mut progress = Progress::load(ctx);
        let q = quiet(&mut progress);
        *q = !*q;
        progress.store(ctx);
    };
    SettingsRow::new(
        id,
        MenuItem::new(
            Label::dynamic(move |_s: &SettingsScreen<Parlor>, ctx: &Ctx<Parlor>| {
                let mut progress = Progress::peek(ctx);
                call_out_label(name, !*quiet(&mut progress))
            }),
            move |s: &mut SettingsScreen<Parlor>, ctx: &mut Ctx<Parlor>| {
                flip(ctx);
                Menu::refresh(s, ctx, true);
                s.speak_current(ctx);
            },
        )
        .help(help)
        .adjustable(
            move |_s: &mut SettingsScreen<Parlor>, ctx: &mut Ctx<Parlor>, _delta| flip(ctx),
        ),
    )
}

/// The game on the shared title builder.
pub struct Cupid;

/// The game's title screen.
pub type Title = menus::Title<Cupid>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(Cupid)
}

fn today(ctx: &Ctx<Parlor>) -> Date {
    parlor_core::challenges::today(ctx)
}

/// The help for a Valentine's row: what it holds, or how long to wait.
fn valentine_help(ctx: &mut Ctx<Parlor>, mode: Mode, open: bool) -> String {
    let what = match mode {
        Mode::Night => "Four rooftops under a full moon, with golden hearts and roses, and a keepsake for this year",
        _ => "Two players take turns with one bow and share the score, with golden hearts and a keepsake",
    };
    if open {
        format!("{what}. {}.", sentence_start(&best_words(ctx, mode)))
    } else {
        format!("{what}. {}", valentine::locked_help(today(ctx)))
    }
}

/// A row that opens on Valentine's Day only.
fn valentine_row(ctx: &mut Ctx<Parlor>, mode: Mode) -> TitleRow<Cupid> {
    let open = valentine::night_open(today(ctx));
    let help = valentine_help(ctx, mode, open);
    let label = if open {
        mode.name().to_string()
    } else {
        labels::locked(mode.name())
    };
    let said = help.clone();
    MenuItem::new(label, move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
        if valentine::night_open(today(ctx)) {
            ctx.push(Field::mode(mode));
        } else {
            ctx.audio.play(sound::NOPE, hearken::Bus::UI);
            ctx.say(said.clone());
        }
    })
    .help(help)
}

impl TitleGame for Cupid {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let progress = Progress::load(ctx);
        let gate = valentine::gate(today(ctx));
        let season = match gate {
            Gate::Season { .. } => " Roses drift in the sky this week.",
            Gate::Day => " It is Valentine's Day: roses drift in the sky.",
            Gate::Off { .. } => "",
        };
        let rooftops = format!(
            "Six rooftops at dusk, each round busier. Hit enough hearts to go on. {}, furthest {} of {}.{season}",
            sentence_start(&best_words(ctx, Mode::Rooftops)),
            progress.best_roofs,
            ROOFTOPS.len()
        );
        let moon_open = progress.moonlight_open();
        let moon_help = if moon_open {
            format!(
                "Endless rounds under the moon, faster each time. {}.",
                sentence_start(&best_words(ctx, Mode::Moonlight))
            )
        } else {
            "Clear all six rooftops in one evening to open moonlight, endless rounds under the moon.".to_string()
        };
        let moon_said = moon_help.clone();
        vec![
            MenuItem::new(
                format!("{}, the rooftops", labels::PLAY),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(Field::mode(Mode::Rooftops)),
            )
            .help(rooftops),
            MenuItem::new(
                if moon_open {
                    Mode::Moonlight.name().to_string()
                } else {
                    labels::locked(Mode::Moonlight.name())
                },
                move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    if Progress::load(ctx).moonlight_open() {
                        ctx.push(Field::mode(Mode::Moonlight));
                    } else {
                        ctx.audio.play(sound::NOPE, hearken::Bus::UI);
                        ctx.say(moon_said.clone());
                    }
                },
            )
            .help(moon_help),
            valentine_row(ctx, Mode::Night),
            valentine_row(ctx, Mode::ForTwo),
        ]
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new(
                Mode::Practice.name(),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(Field::mode(Mode::Practice)),
            )
            .help("One slow heart at a time, endless arrows and nothing to lose. Escape pauses."),
        )
    }

    fn extras(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let progress = Progress::load(ctx);
        let notes_label = format!(
            "Love notes, {} of {}",
            progress.notes_found(),
            notes::NOTES.len()
        );
        vec![
            MenuItem::new(notes_label, |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                let pages = notes_pages(ctx);
                ctx.push(Pages::new(pages));
            })
            .help("The love notes your arrows have brought down, read a page at a time."),
            MenuItem::new("Keepsake box", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                let pages = keepsake_pages(ctx);
                ctx.push(Pages::new(pages));
            })
            .help("One keepsake for each Valentine's Day you played Valentine's night."),
        ]
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![
            call_out_row(
                "aim_call_outs",
                AIM_CALLS,
                |p| &mut p.quiet_aim,
                "When on, turning the bow speaks its aim, such as 15 degrees right. The tick always sounds.",
            ),
            call_out_row(
                "target_call_outs",
                TARGET_CALLS,
                |p| &mut p.quiet,
                "When on, each arriving target is named, such as pink heart, from the left. Target hums always sound.",
            ),
        ]
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Best scores, the furthest rooftop, totals and love notes, with your challenge results."
            .to_string()
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
            "A daily sky with a twist, three rooftops each week, and a monthly full moon, the same for every player. {}",
            daily.resets()
        )
    }

    /// A player who has finished a run is no longer new.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && Progress::load(ctx).runs == 0
    }
}
