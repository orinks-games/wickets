//! Graveyard Shuffle: lead the monsters' new dance craze by ear.
//!
//! Doctor Volt's newest creation sat up on the slab and started to
//! shuffle, and now the whole graveyard wants to learn it. The graveyard
//! band plays; each monster comes up with its own sound, a groan, a rattle,
//! a howl, one beat before its move, and the player dances that move on the
//! beat with the monster's key. Moves are judged spot on, good or close by
//! how near the beat they land. Four nights of three songs, each a little
//! faster, open one after another; the practice room plays a song that
//! waits for each move. A holiday special: the launcher lists it in the
//! two weeks before Halloween, and all year once the player starts a dance or practice
//! session. Doctor Volt's laboratory, the last night, opens only in
//! Halloween season, even in a game kept all year.

pub mod challenge;
pub mod picker;
pub mod play;
pub mod progress;
pub mod rules;
pub mod songs;
pub mod sound;
pub mod soundcheck;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::screens::SettingsRow;
use hearken::text::end_sentence;
use hearken::Ctx;
use parlor_core::calendar::{self, Holiday, Nearness};
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{
    Category, Challenge, Challenges, Date, GameInfo, Happening, Parlor, Period, Records,
};

pub use challenge::{ChallengeHost, ChallengesMenu, CHALLENGES_NAME};
pub use play::{Floor, Mode};
pub use progress::Progress;
pub use soundcheck::SoundCheck;

use rules::steps_words;
use songs::NIGHTS;

/// The game's id: its sounds live under `sounds/graveyard-shuffle/`.
pub const ID: &str = "graveyard-shuffle";

/// The game's name.
pub const NAME: &str = "Graveyard Shuffle";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a run returns to.
pub const TITLE_NAME: &str = "graveyard_shuffle";

/// The holiday whose season the game turns up in.
pub const HOLIDAY: Holiday = Holiday::Halloween;

/// Where the launcher lists the game all year, once it is kept.
pub const HOME: Category = Category::Action;

/// The blurb in season, until the game is kept.
pub const KEEP_BLURB: &str = "A Halloween special. Lead the monsters' new dance craze: hear who comes up, and dance their move on the beat. Play once, even in practice, to keep it all year.";

/// The blurb out of season, once the game is kept.
pub const YEAR_ROUND_BLURB: &str = "The monsters' dance craze, all year: hear who comes up, and dance their move on the beat. Doctor Volt's laboratory waits for Halloween.";

/// Said in the results when a night first keeps the game in season.
pub const KEPT_LINE: &str =
    "Graveyard Shuffle is yours to keep. After Halloween, it moves to the Action games.";

/// Said in the results when a night first keeps the game out of season,
/// when the laboratory is shut.
pub const KEPT_LINE_OUT_OF_SEASON: &str = "Graveyard Shuffle is yours to keep, in the Action games. Doctor Volt's laboratory waits for Halloween.";

/// Whether the ordinary game has been played or kept by an earlier save.
pub fn kept(ctx: &mut Ctx<Parlor>) -> bool {
    Progress::load(ctx).kept()
}

/// How the launcher lists the game in Halloween season, and all year once
/// it is kept.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: NAME,
    category: Category::Seasonal,
    blurb: "A Halloween special. Lead the monsters' new dance craze: hear who comes up, and dance their move on the beat.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <GraveyardShuffle as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[parlor_core::story::PartOfDay::Night],
        seasons: &[parlor_core::Season::Autumn],
    },
};

/// While the game is in the parlor, the launcher names it as an event.
pub fn event(date: Date) -> Option<Happening> {
    let line = match calendar::nearness(HOLIDAY, date)? {
        Nearness::Day => "The Halloween dance is on. The graveyard goes quiet after 1 November.",
        Nearness::Season { .. } => {
            "The graveyard dances until the night after Halloween, then it goes quiet for the year."
        }
    };
    Some(Happening::new("The Halloween special", line))
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

/// Whether the screen reader also names each monster as it comes up.
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
    let today = parlor_core::challenges::today(ctx);
    let open = NIGHTS
        .iter()
        .filter(|s| p.night_open(s.number, today))
        .count();
    let nights: Vec<String> = NIGHTS
        .iter()
        .filter(|s| p.night_open(s.number, today))
        .map(|s| {
            let stars: u32 = s.songs().iter().map(|t| p.stars_for(t.id)).sum();
            let best = match Records::best(ctx, ID, &progress::set_board(s.number)) {
                Some(b) => format!("best {b}"),
                None => "not danced yet".to_string(),
            };
            format!("{}: {best}, {stars} of 9 stars.", s.title())
        })
        .collect();
    let songs = match p.tunes_danced {
        1 => "1 song".to_string(),
        n => format!("{n} songs"),
    };
    let night_count = match p.sets_danced {
        1 => "1 night".to_string(),
        n => format!("{n} nights"),
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
            "Records. Stars: {} of 36. Nights open: {open} of {}.",
            p.total_stars(),
            NIGHTS.len()
        ),
        nights.join(" "),
        format!(
            "Danced so far: {night_count}, {songs}, {} in time, {} of them spot on. Longest streak: {}. Practice room: {}.",
            moves_words(p.steps),
            p.spot_on,
            moves_words(p.best_streak),
            moves_words(p.practice_steps)
        ),
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

/// "1 move" or "24 moves".
pub fn moves_words(n: u32) -> String {
    steps_words(n)
}

/// The game on the shared title builder.
pub struct GraveyardShuffle;

/// The game's title screen.
pub type Title = menus::Title<GraveyardShuffle>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(GraveyardShuffle)
}

impl TitleGame for GraveyardShuffle {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let p = Progress::load(ctx);
        let today = parlor_core::challenges::today(ctx);
        let n = p.current_set(today);
        let night = &NIGHTS[n as usize - 1];
        let names: Vec<&str> = night.songs().iter().map(|t| t.name).collect();
        let mut play_help = format!("Three songs in a row: {}.", names.join(", "));
        if let Some(w) = p.waiting(today).filter(|&w| w == n + 1) {
            play_help.push_str(&format!(" {}", picker::waits_words(w)));
        }
        vec![
            MenuItem::new(
                format!("{}, {}", labels::PLAY, night.title_in_sentence()),
                move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    let f = Floor::open(ctx, Mode::Set(n));
                    ctx.push(f)
                },
            )
            .help(play_help),
            MenuItem::new("Night select", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(picker::SetSelect::new())
            })
            .help("Dance any night you have opened, with the stars on each."),
        ]
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new("Practice room", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(picker::TuneSelect::new())
            })
            .help("Any song you have opened. The music holds for each monster until you dance its move."),
        )
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your stars, your best score on each night, your moves danced, and your challenge results."
            .to_string()
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![menus::settings::toggle(
            "graveyard_shuffle_call_outs",
            labels::call_outs,
            call_outs_peek,
            set_call_outs,
            |_ctx, on| {
                Some(
                    if on {
                        "Spoken call-outs: on. Each monster is also named."
                    } else {
                        "Spoken call-outs: off. The monsters are not named."
                    }
                    .to_string(),
                )
            },
            "When on, the game also names each monster as it comes up.",
        )]
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Every monster, move and cue in the game, with its name and the key that dances it."
            .to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "A daily song, a weekly night and a monthly long night, the same monsters for every player. {}",
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
