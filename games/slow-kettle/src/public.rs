// Slow Kettle: keep a small tea house at your own pace.
//
// Guests come in one at a time and say what they feel like. The player
// chooses a tea from the shelf and a cup, heats the kettle by ear (its
// song rises from a rumble to a hiss to a whistle), pours, and lifts the
// leaves when the steep tastes right. Nothing fails: every cup pleases,
// a perfect one earns a note and a tip, and tips buy teas, cups and a
// music box. Regulars come back with stories; each season has its blend,
// and certain holidays bring a special guest, a special brew and a
// keepsake (see `days`).

pub mod challenge;
pub mod days;
pub mod guests;
pub mod play;
pub mod rules;
pub mod save;
pub mod shop;
pub mod sound;
mod soundcheck;
pub mod talk;
pub mod teas;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::screens::SettingsRow;
use hearken::text::{end_sentence, sentence_start};
use hearken::Ctx;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{
    challenges, Category, Challenge, Challenges, Date, GameInfo, Happening, Pages, Parlor, Period,
    Records,
};

pub use challenge::{ChallengeHost, ChallengesMenu, CHALLENGES_NAME};
pub use play::{Counter, COUNTER_NAME};
pub use save::House;
pub use shop::Shop;
pub use soundcheck::{SoundCheck, SOUNDS_NAME};

use rules::{Mode, EVENING_AFTER};
use talk::coins_words;

/// The game's id: its sounds live under `sounds/slow-kettle/`.
pub const ID: &str = "slow-kettle";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a sitting returns to.
pub const TITLE_NAME: &str = "slow_kettle";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Slow Kettle",
    category: Category::Cozy,
    blurb: "Keep a small tea house at your own pace, brewing each guest's cup.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <SlowKettle as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[
            parlor_core::story::PartOfDay::Afternoon,
            parlor_core::story::PartOfDay::Evening,
        ],
        seasons: &[parlor_core::Season::Autumn, parlor_core::Season::Winter],
    },
};

/// The holiday guest the tea house is expecting on `date`, if any.
pub fn event(date: Date) -> Option<Happening> {
    days::event(date)
}

/// Opens the current `period` challenge from the launcher's list.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// What today holds, for the Play row's help: the day's guest, or the
/// coming holiday.
pub fn today_words(today: Date) -> String {
    if let Some(o) = days::occasion_on(today) {
        return format!("Today: {}", o.line(o.year(today)));
    }
    let blend = days::blend(today);
    let mut out = format!("{} is this season's blend.", blend.name);
    if let Some((o, _)) = days::coming(today) {
        if let Some(c) = o.countdown(today) {
            out.push_str(&format!(" {c}"));
        }
    }
    out
}

/// Records: the house's counts, then the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let house = House::load(ctx);
    let mut first = if house.sessions() == 0 {
        "Records. The tea house has not opened yet.".to_string()
    } else {
        format!(
            "Records. {} and {} played. {} served, {} perfect, {} earned in all.",
            plural(house.afternoons, "afternoon"),
            plural(house.evenings, "evening"),
            plural(house.cups_served, "cup"),
            house.perfect,
            coins_words(house.coins_earned)
        )
    };
    for mode in [Mode::Afternoon, Mode::Evening] {
        if let Some(best) = mode.board().and_then(|b| Records::best(ctx, ID, b)) {
            let what = match mode {
                Mode::Evening => "evening",
                _ => "afternoon",
            };
            first.push_str(&format!(" Best {what}: {}.", coins_words(best as u32)));
        }
    }
    first.push_str(&format!(
        " {} discovered. {} kept.",
        sentence_start(&house.tasted_words()),
        sentence_start(&plural(house.keepsakes.len() as u32, "keepsake"))
    ));
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    vec![first, format!("Challenges. {}", challenges.join(" "))]
}

/// "1 cup" or "3 cups".
pub fn plural(n: u32, word: &str) -> String {
    match n {
        1 => format!("1 {word}"),
        n => format!("{n} {word}s"),
    }
}

fn start(ctx: &mut Ctx<Parlor>, mode: Mode) {
    let counter = Counter::new(ctx, mode);
    ctx.push(counter);
}

/// Saves the call-outs, from their Settings row.
fn set_call_outs(ctx: &mut Ctx<Parlor>, on: bool) {
    let mut house = House::load(ctx);
    house.quiet = !on;
    house.store(ctx);
}

fn music_box_label(house: &House) -> String {
    if !house.has_music_box() {
        return "Music box: not bought yet".to_string();
    }
    match house.playing() {
        Some(t) => format!("Music box: {}", t.name),
        None => "Music box: off".to_string(),
    }
}

/// The music box's next tune, from its Settings row. Before it is bought,
/// says where to buy it.
fn cycle_music_box(ctx: &mut Ctx<Parlor>, _delta: i32) -> Option<String> {
    let mut house = House::load(ctx);
    if !house.has_music_box() {
        sound::play(ctx, sound::NOPE);
        return Some("The music box is in the shop, for 15 coins.".to_string());
    }
    house.next_tune();
    house.store(ctx);
    None
}

/// The game on the shared title builder.
pub struct SlowKettle;

/// The game's title screen.
pub type Title = menus::Title<SlowKettle>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(SlowKettle)
}

impl TitleGame for SlowKettle {
    const ID: &'static str = ID;
    const NAME: &'static str = "Slow Kettle";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let house = House::load(ctx);
        let today = challenges::today(ctx);
        let day = today_words(today);
        let afternoon = format!(
            "Four guests, one at a time, at your own pace. You have {}. {day}",
            coins_words(house.coins)
        );
        let evening_open = play::evening_open(&house);
        let evening = if evening_open {
            MenuItem::new("Long evening", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                start(ctx, Mode::Evening)
            })
            .help(format!(
                "Seven guests on a rainy evening, three of them regulars. {day}"
            ))
        } else {
            let left = EVENING_AFTER - house.afternoons.min(EVENING_AFTER);
            let more = match left {
                1 => "1 more afternoon".to_string(),
                n => format!("{n} more afternoons"),
            };
            let said = format!("The long evening opens after {more}.");
            MenuItem::new(
                labels::locked("Long evening"),
                move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    sound::play(ctx, sound::NOPE);
                    ctx.say(said.clone());
                },
            )
            .help(format!(
                "Seven guests on a rainy evening. It opens after {more}."
            ))
        };
        vec![
            MenuItem::new(
                format!("{}, open the tea house", labels::PLAY),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| start(ctx, Mode::Afternoon),
            )
            .help(afternoon),
            evening,
        ]
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new("Practice brewing", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                start(ctx, Mode::Practice)
            })
            .help("Brew any tea on your shelf for yourself, as often as you like, with the exact heat and steep read back. There are no guests and no coins."),
        )
    }

    fn extras(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let house = House::load(ctx);
        vec![
            MenuItem::new("Shop", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(Shop::new())
            })
            .help(format!(
                "Spend your coins on teas, cups, the music box and the sand timer. You have {}.",
                coins_words(house.coins)
            )),
            MenuItem::new("Keepsakes", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                let house = House::load(ctx);
                ctx.push(Pages::new(shop::keepsake_pages(&house)))
            })
            .help("Gifts from holiday guests, one for each special day you open on."),
            MenuItem::new("Guest book", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                let house = House::load(ctx);
                ctx.push(Pages::new(shop::guest_book_pages(&house)))
            })
            .help("Your regulars, and their stories so far."),
        ]
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![
            menus::settings::toggle(
                "slow_kettle_call_outs",
                labels::call_outs,
                |ctx| !House::peek(ctx).quiet,
                set_call_outs,
                |_ctx, _on| None,
                "When on, the kettle's song and the steeping clock are also spoken as they change.",
            ),
            menus::settings::setting(
                "slow_kettle_music_box",
                |ctx| music_box_label(&House::peek(ctx)),
                cycle_music_box,
                "Which tune the music box plays in the tea house, or off.",
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
        "Cups served, perfect cups, teas discovered, your best afternoon, and your challenge results."
            .to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Plays the kettle's song, every cue and every cup.".to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "A daily tasting, a busy week and a monthly tasting society, the same guests for every player. {}",
            daily.resets()
        )
    }

    /// A player who has opened the tea house is no longer new.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && House::load(ctx).sessions() == 0
    }
}
