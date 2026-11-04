// Riverboat Dice: stay at the dice table or bank, before a 1 comes up.
//
// A push-your-luck dice game on a paddle steamer. Each landing, the
// dealer rolls two dice and adds their sum to the table's total for
// everyone still seated. Stand up and bank it, or stay for another roll;
// a 1 on either die ends the landing and everyone seated gets nothing.
// Three passengers play against you, each by one plain rule, drawing on
// the same dice you do. The highest night total after five landings wins.

pub mod challenge;
pub mod events;
pub mod progress;
pub mod rules;
pub mod sound;
mod soundcheck;
pub mod table;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::text::end_sentence;
use hearken::Ctx;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Records};

pub use challenge::{ChallengeHost, ChallengesMenu, Nights, CHALLENGES_NAME};
pub use progress::Progress;
pub use soundcheck::SoundCheck;
pub use table::TableScreen;

use rules::{Opponent, LANDINGS};

/// The game's id: its sounds live under `sounds/riverboat-dice/`.
pub const ID: &str = "riverboat-dice";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a night returns to.
pub const TITLE_NAME: &str = "riverboat_dice";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Riverboat Dice",
    category: Category::Table,
    blurb: "Stay at the dice table or bank your winnings, before a 1 comes up.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(events::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <RiverboatDice as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[
            parlor_core::story::PartOfDay::Evening,
            parlor_core::story::PartOfDay::Night,
        ],
        seasons: &[parlor_core::Season::Summer],
    },
};

/// Opens the current `period` challenge, from the launcher's list of
/// today's challenges: its briefing, then the night. Leaving it returns to
/// the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// Records: the saloon's bests, then the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let progress = Progress::load(ctx);
    let first = if progress.nights == 0 {
        "Records. No saloon night played yet.".to_string()
    } else {
        let best = Progress::best(ctx).unwrap_or(0);
        format!(
            "Records. Best night: {best}. Nights won: {} of {}.",
            progress.wins, progress.nights
        )
    };
    let stay = match progress.longest_stay {
        0 => "Longest stay: nobody has stayed in for a roll yet.".to_string(),
        1 => "Longest stay: 1 roll in one landing.".to_string(),
        n => format!("Longest stay: {n} rolls in one landing."),
    };
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    let mut first = format!("{first} {stay}");
    if let Some(b) = Records::best(ctx, ID, events::MOON_BOARD) {
        first.push_str(&format!(" {}: best {b}.", events::MOON_NAME));
    }
    vec![first, format!("Challenges. {}", challenges.join(" "))]
}

/// The game on the shared title builder.
pub struct RiverboatDice;

/// The game's title screen.
pub type Title = menus::Title<RiverboatDice>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(RiverboatDice)
}

impl TitleGame for RiverboatDice {
    const ID: &'static str = ID;
    const NAME: &'static str = "Riverboat Dice";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let best = match Progress::best(ctx) {
            Some(b) if Progress::load(ctx).nights > 0 => format!(" Best night {b}."),
            _ => String::new(),
        };
        let field: Vec<&str> = rules::SALOON
            .field
            .iter()
            .map(|o: &Opponent| o.name())
            .collect();
        let mut rows = vec![MenuItem::new(
            format!("{}, Saloon", labels::PLAY),
            |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(TableScreen::new()),
        )
        .help(format!(
            "{LANDINGS} landings against {}.{best}",
            rules::and_list(&field)
        ))];
        let today = parlor_core::challenges::today(ctx);
        if events::moon_open(today) {
            let help = events::row_help(Records::best(ctx, ID, events::MOON_BOARD));
            rows.push(
                MenuItem::new(
                    events::MOON_NAME,
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(TableScreen::moon()),
                )
                .help(help),
            );
        } else {
            // Closed today: the row keeps its place and says when it opens.
            rows.push(
                MenuItem::new(
                    labels::locked(events::MOON_NAME),
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        let today = parlor_core::challenges::today(ctx);
                        ctx.play_ui(hearken::UiSound::Error);
                        ctx.say(events::locked_line(today))
                    },
                )
                .select_sound(None)
                .help(events::locked_help(today)),
            );
        }
        rows
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your best night, nights won and the longest stay, with your challenge results.".to_string()
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
            "A daily table, a weekly snag week and a monthly Captain's table, the same dice for every player. {}",
            daily.resets()
        )
    }

    /// A player who has finished a night is no longer new, tutorial or not.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && Progress::load(ctx).nights == 0
    }
}
