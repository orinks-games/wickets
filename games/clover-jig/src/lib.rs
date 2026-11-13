//! Clover Jig: dance to a jig or a reel on the caller's word.
//!
//! A fiddle, a tin whistle and a bodhran play; a caller calls each step a
//! beat ahead, "left", "right", "hop", "turn", and the player dances it on
//! the beat with the matching key. Steps are judged spot on, good or close
//! by how near the beat they land. Four sets of three tunes, each set a
//! little faster, open one after another; the practice room plays a tune
//! that waits for each step. Around Saint Patrick's Day the village keeps
//! the feast: shamrocks hide in the tunes in the week before, and on the
//! day ceili night opens with the full band, a set dance, a four-leaf
//! clover and a keepsake for the year.

pub mod challenge;
pub mod picker;
pub mod play;
pub mod progress;
pub mod rules;
pub mod season;
pub mod sound;
pub mod soundcheck;
pub mod tunes;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::screens::SettingsRow;
use hearken::text::{end_sentence, sentence_start};
use hearken::{Ctx, UiSound};
use parlor_core::challenges::today;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{
    Category, Challenge, Challenges, Date, GameInfo, Happening, Pages, Parlor, Period, Records,
};

pub use challenge::{ChallengeHost, ChallengesMenu, CHALLENGES_NAME};
pub use play::{Floor, Mode};
pub use progress::Progress;
pub use soundcheck::SoundCheck;

use rules::{luck_words, steps_words};
use tunes::SETS;

/// The game's id: its sounds live under `sounds/clover-jig/`.
pub const ID: &str = "clover-jig";

/// The game's name.
pub const NAME: &str = "Clover Jig";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a run returns to.
pub const TITLE_NAME: &str = "clover_jig";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: NAME,
    category: Category::Seasonal,
    blurb: "Dance a jig or reel on the beat after each call, and join the village ceili.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <CloverJig as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[parlor_core::story::PartOfDay::Evening],
        seasons: &[parlor_core::Season::Spring],
    },
};

/// The event Clover Jig holds on `date`: shamrock week, then ceili night.
pub fn event(date: Date) -> Option<Happening> {
    season::event(date)
}

/// Opens the current `period` challenge from the launcher's list of
/// today's challenges: its briefing, then the dance. Leaving it returns
/// to the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

fn call_outs_key() -> String {
    format!("{ID}/call_outs")
}

/// Whether the screen reader also speaks each call.
pub fn call_outs(ctx: &mut Ctx<Parlor>) -> bool {
    Records::load(ctx, &call_outs_key())
}

/// The call-outs as last read, for a Settings row.
fn call_outs_peek(ctx: &Ctx<Parlor>) -> bool {
    Records::peek(ctx, &call_outs_key())
}

/// Saves the call-outs, from their Settings row.
fn set_call_outs(ctx: &mut Ctx<Parlor>, on: bool) {
    Records::store(ctx, &call_outs_key(), &on);
}

/// Records: stars and bests, the tallies, then the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let p = Progress::load(ctx);
    let open = SETS.iter().filter(|s| p.set_open(s.number)).count();
    let sets: Vec<String> = SETS
        .iter()
        .filter(|s| p.set_open(s.number))
        .map(|s| {
            let stars: u32 = s.tunes().iter().map(|t| p.stars_for(t.id)).sum();
            let best = match Records::best(ctx, ID, &progress::set_board(s.number)) {
                Some(b) => format!("best {b}"),
                None => "not danced yet".to_string(),
            };
            format!("{}: {best}, {stars} of 9 stars.", s.title())
        })
        .collect();
    let ceili = match Records::best(ctx, ID, progress::CEILI_BOARD) {
        Some(b) => format!(" Ceili night: best {b}."),
        None => String::new(),
    };
    let tunes = match p.tunes_danced {
        1 => "1 tune".to_string(),
        n => format!("{n} tunes"),
    };
    let set_count = match p.sets_danced {
        1 => "1 set".to_string(),
        n => format!("{n} sets"),
    };
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    vec![
        format!(
            "Records. Stars: {} of 36. Sets open: {open} of {}.{ceili}",
            p.total_stars(),
            SETS.len()
        ),
        sets.join(" "),
        format!(
            "Danced so far: {set_count}, {tunes}, {} in time, {} of them spot on. Longest run in time: {}. Practice room: {}.",
            steps_words(p.steps),
            p.spot_on,
            steps_words(p.best_streak),
            steps_words(p.practice_steps)
        ),
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

/// The keepsakes: luck, clovers, and each year's ceili.
pub fn keepsake_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let p = Progress::load(ctx);
    let date = today(ctx);
    let clovers = match p.clovers {
        0 => "no four-leaf clover yet".to_string(),
        1 => "1 four-leaf clover found".to_string(),
        n => format!("{n} four-leaf clovers found"),
    };
    let shamrocks = match p.shamrocks_heard {
        1 => "1 shamrock".to_string(),
        n => format!("{n} shamrocks"),
    };
    let mut pages = vec![format!(
        "Keepsakes. {}, from {shamrocks} heard. {}.",
        sentence_start(&luck_words(p.luck)),
        sentence_start(&clovers)
    )];
    if p.keepsakes.is_empty() {
        pages.push(format!(
            "No ceili keepsakes yet. The village ceili opens on Saint Patrick's Day, March 17. {}",
            season::countdown(date)
        ));
    } else {
        for (year, k) in p.keepsakes.iter().rev() {
            let nights = match k.nights {
                1 => "danced once".to_string(),
                n => format!("danced {n} times"),
            };
            let clover = if k.clover {
                "four-leaf clover found"
            } else {
                "no clover"
            };
            pages.push(format!(
                "Saint Patrick's Day {year}: ceili night {nights}, best {}, {clover}, {}.",
                k.best,
                luck_words(k.luck)
            ));
        }
        if !season::ceili_open(date) {
            pages.push(season::countdown(date));
        }
    }
    pages
}

/// The pages before ceili night, which then start it.
pub fn ceili_briefing(ctx: &mut Ctx<Parlor>) -> Pages {
    let date = today(ctx);
    let night = season::night_key(date);
    let clover = if progress::clover_due(ctx, &night) {
        "A four-leaf clover hides in the third or fourth tune, once a night: three sparkling notes with a call. Dance that step to find it."
    } else {
        "The four-leaf clover has been out once tonight already, so tonight's dances have shamrocks only."
    };
    let year = season::keepsake_year(date);
    let pages = vec![
        "Ceili night. The village hall is full for Saint Patrick's Day, and the full band is on the stage: fiddle, whistle, harp, accordion, bass and bodhran.".to_string(),
        "The band plays the set dance: four tunes in a row, and the caller names each figure between them. Every step is called, and shamrocks hide in every tune.".to_string(),
        format!("{clover} Each set dance you finish goes in your keepsake for {year}. Escape on this page goes back without dancing."),
    ];
    Pages::briefing(pages).then(|ctx| {
        let f = Floor::open(ctx, Mode::Ceili);
        ctx.replace(f);
    })
}

/// The game on the shared title builder.
pub struct CloverJig;

/// The game's title screen.
pub type Title = menus::Title<CloverJig>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(CloverJig)
}

impl TitleGame for CloverJig {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let p = Progress::load(ctx);
        let date = today(ctx);
        let n = p.current_set();
        let set = &SETS[n as usize - 1];
        let names: Vec<&str> = set.tunes().iter().map(|t| t.name).collect();
        let mut play_help = format!("Three tunes in a row: {}.", names.join(", "));
        if let Some(line) = season::shamrock_line(date) {
            play_help.push(' ');
            play_help.push_str(&line);
        }
        let open = season::ceili_open(date);
        let ceili_label = if open {
            "Ceili night, open tonight".to_string()
        } else {
            labels::locked("Ceili night")
        };
        vec![
            MenuItem::new(
                format!("{}, {}", labels::PLAY, set.title().to_lowercase()),
                move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    let f = Floor::open(ctx, Mode::Set(n));
                    ctx.push(f)
                },
            )
            .help(play_help),
            MenuItem::new("Set select", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(picker::SetSelect::new())
            })
            .help("Dance any set you have opened, with the stars on each."),
            MenuItem::new(ceili_label, |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                let date = today(ctx);
                if season::ceili_open(date) {
                    let pages = ceili_briefing(ctx);
                    ctx.push(pages);
                } else {
                    ctx.play_ui(UiSound::Error);
                    ctx.say(season::ceili_help(date));
                }
            })
            .help(season::ceili_help(date)),
        ]
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new("Practice room", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(picker::TuneSelect::new())
            })
            .help("Any tune you have opened. The music holds at each step until you dance it."),
        )
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your stars, your best score on each set, your steps danced, and your challenge results."
            .to_string()
    }

    fn extras(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        vec![
            MenuItem::new("Keepsakes", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                let pages = keepsake_pages(ctx);
                ctx.push(Pages::new(pages))
            })
            .help("Your luck points, four-leaf clovers, and a keepsake for each Saint Patrick's ceili."),
        ]
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![menus::settings::toggle(
            "clover_jig_call_outs",
            labels::call_outs,
            call_outs_peek,
            set_call_outs,
            |_ctx, on| {
                Some(
                    if on {
                        "Spoken call-outs: on. Each call is also spoken."
                    } else {
                        "Spoken call-outs: off. Only the caller calls the steps."
                    }
                    .to_string(),
                )
            },
            "When on, the game also speaks each step as the caller calls it.",
        )]
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Every call, step and cue in the game, with its name and the key that dances it."
            .to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "A daily tune, a weekly set and a monthly long set, the same calls for every player. {}",
            daily.resets()
        )
    }

    /// A player who has danced is no longer new, tutorial or not.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && !Progress::load(ctx).any()
    }

    fn intro_help(&self) -> Option<String> {
        Some("Up and Down move, Enter chooses, Escape goes back to the parlor.".to_string())
    }
}
