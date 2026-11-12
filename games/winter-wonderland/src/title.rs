//! The title, on the shared `parlor_core::menus::Title`: the story, Gift
//! Sort, the Wish Desk and Muddled post, Midnight Countdown, the
//! challenges and rhythm practice, then the shared rows.

use hearken::menu::{Label, MenuItem};
use hearken::screens::SettingsRow;
use hearken::text::end_sentence;
use hearken::Ctx;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{Challenge, Challenges, Pages, Parlor, Period};

use crate::challenge::ChallengesMenu;
use crate::count_run::CountRun;
use crate::desk::STACKS;
use crate::desk_run::DeskRun;
use crate::events;
use crate::gifts::SHIFTS;
use crate::mode::{story_intro, Mode};
use crate::night::{NightSpec, NIGHTS};
use crate::progress::{self, Progress};
use crate::sort_run::SortRun;
use crate::sound;
use crate::soundcheck::SoundCheck;
use crate::{HELP, ID, TITLE_NAME};

/// The game on the shared title builder, with the shift and night the
/// player has chosen on the title.
pub struct Winter {
    /// The shift Gift Sort starts from.
    pub shift: u32,
    /// The night Midnight Countdown and rhythm practice play.
    pub night: u32,
    /// The stack the Wish Desk plays.
    pub stack: u32,
    progress: Progress,
}

impl Default for Winter {
    fn default() -> Self {
        Winter {
            shift: 1,
            night: 1,
            stack: 1,
            progress: Progress::default(),
        }
    }
}

/// The game's title screen.
pub type Title = menus::Title<Winter>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(Winter::default())
}

/// "best 420" or "not played yet", for the end of a row's label.
fn best_words(ctx: &mut Ctx<Parlor>, board: &str) -> String {
    progress::best_text(ctx, board).to_lowercase()
}

/// The story's opening pages, which then start its first shift. Escape
/// goes back to the title without starting.
pub fn story_briefing(blizzard: bool) -> Pages {
    Pages::briefing(story_intro(blizzard)).then(move |ctx| ctx.replace(story_run(blizzard)))
}

/// The story's first Gift Sort shift.
pub fn story_run(blizzard: bool) -> SortRun {
    SortRun::new(
        Mode::Story { blizzard, gifts: 0 },
        if blizzard { 4 } else { 1 },
    )
}

/// Records: every best, how far each part is open, then the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let p = Progress::load(ctx);
    let story = best_words(ctx, progress::STORY_BOARD);
    let blizzard = if p.story {
        best_words(ctx, progress::BLIZZARD_BOARD)
    } else {
        "locked".to_string()
    };
    let gifts = best_words(ctx, progress::GIFT_BOARD);
    let desk = best_words(ctx, progress::DESK_BOARD);
    let muddled = best_words(ctx, events::MUDDLED_BOARD);
    let count = best_words(ctx, progress::COUNT_BOARD);
    let mut pages = vec![format!(
        "Records. Story mode: {story}. Blizzard story: {blizzard}. Gift Sort: {gifts}, {} of {SHIFTS} shifts open. Wish Desk: {desk}, {} of {STACKS} stacks open. Muddled post: {muddled}. Midnight Countdown: {count}, {} of {NIGHTS} nights open.",
        p.shifts_open(),
        p.stacks_open(),
        p.nights_open()
    )];
    let stacks: Vec<String> = (1..=STACKS)
        .map(|n| {
            let best = if n <= p.stacks_open() {
                best_words(ctx, &progress::stack_board(n))
            } else {
                "locked".to_string()
            };
            format!("Stack {n}: {best}.")
        })
        .collect();
    pages.push(format!("Wish Desk by stack. {}", stacks.join(" ")));
    let nights: Vec<String> = (1..=NIGHTS)
        .map(|n| {
            let best = if n <= p.nights_open() {
                best_words(ctx, &progress::night_board(n))
            } else {
                "locked".to_string()
            };
            format!("Night {n}, {}: {best}.", NightSpec::new(n).name)
        })
        .collect();
    pages.push(format!("Midnight Countdown by night. {}", nights.join(" ")));
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    pages.push(format!("Challenges. {}", challenges.join(" ")));
    pages
}

impl TitleGame for Winter {
    const ID: &'static str = ID;
    const NAME: &'static str = "Winter Wonderland";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(Some(sound::TITLE_MUSIC), true);
    }

    /// Pace sets Gift Sort's clocks; the desk has none and Midnight
    /// Countdown keeps the band's beat.
    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![parlor_core::pace::row_for(ID, "Gift Sort")]
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        self.progress = Progress::load(ctx);
        self.shift = self.shift.clamp(1, self.progress.shifts_open());
        self.night = self.night.clamp(1, self.progress.nights_open());
        self.stack = self.stack.clamp(1, self.progress.stacks_open());
        let stacks: Vec<String> = (1..=STACKS)
            .map(|n| best_words(ctx, &progress::stack_board(n)))
            .collect();
        let today = parlor_core::challenges::today(ctx);
        let muddled_best = parlor_core::Records::best(ctx, ID, events::MUDDLED_BOARD);
        let muddled_row = if events::muddled(today) {
            MenuItem::new(
                events::MUDDLED_NAME,
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(DeskRun::new(Mode::Muddled, 1)),
            )
            .help(events::muddled_help(muddled_best))
        } else {
            // Closed today: the row keeps its place and says when it opens.
            MenuItem::new(
                labels::locked(events::MUDDLED_NAME),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    let today = parlor_core::challenges::today(ctx);
                    ctx.play_ui(hearken::UiSound::Error);
                    ctx.say(events::muddled_locked_line(today))
                },
            )
            .select_sound(None)
            .help(events::muddled_locked_help(today, muddled_best))
        };
        let late = if events::late_post_due(ctx) {
            " Late post tonight: two more letters after your next stack."
        } else {
            ""
        };
        let story = best_words(ctx, progress::STORY_BOARD);
        let blizzard = best_words(ctx, progress::BLIZZARD_BOARD);
        let gifts = best_words(ctx, progress::GIFT_BOARD);
        let nights: Vec<String> = (1..=NIGHTS)
            .map(|n| best_words(ctx, &progress::night_board(n)))
            .collect();

        let blizzard_row = if self.progress.story {
            MenuItem::new(
                format!("Blizzard story, {blizzard}"),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(story_briefing(true)),
            )
            .help("The story again, harder: shifts 4 to 6, then a bigger stack at the desk.")
        } else {
            MenuItem::new(
                labels::locked("Blizzard story"),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.say("The blizzard story opens once you finish story mode.")
                },
            )
            .select_sound(None)
            .help("Finish story mode once to open the blizzard story.")
        };
        vec![
            MenuItem::new(
                format!("{}, story mode, {story}", labels::PLAY),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(story_briefing(false)),
            )
            .help("Three Gift Sort shifts, then a stack of letters at the Wish Desk, scored together."),
            blizzard_row,
            MenuItem::new(
                Label::dynamic(move |s: &Title, _| {
                    format!("Gift Sort, shift {}, {gifts}", s.game.shift)
                }),
                |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.push(SortRun::new(Mode::Free, s.game.shift))
                },
            )
            .adjustable(|s: &mut Title, _ctx: &mut Ctx<Parlor>, delta: i32| {
                let open = s.game.progress.shifts_open() as i32;
                s.game.shift = (s.game.shift as i32 + delta).clamp(1, open) as u32;
            })
            .help(format!(
                "Sort gifts by their sound, shift after shift, up to shift {SHIFTS}. Left and Right choose the starting shift once later shifts are open."
            )),
            MenuItem::new(
                Label::dynamic(move |s: &Title, _| {
                    let best = stacks
                        .get(s.game.stack as usize - 1)
                        .cloned()
                        .unwrap_or_default();
                    format!(
                        "Wish Desk, stack {}, {} open, {best}",
                        s.game.stack,
                        s.game.progress.stacks_open()
                    )
                }),
                |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.push(DeskRun::new(Mode::Free, s.game.stack))
                },
            )
            .adjustable(|s: &mut Title, _ctx: &mut Ctx<Parlor>, delta: i32| {
                let open = s.game.progress.stacks_open() as i32;
                s.game.stack = (s.game.stack as i32 + delta).clamp(1, open) as u32;
            })
            .help(format!(
                "Read children's letters, find each gift on the shelf by shaking, and wrap it. No clock. Left and Right choose the stack, up to {STACKS}.{late}"
            )),
            muddled_row,
            MenuItem::new(
                Label::dynamic(move |s: &Title, _| {
                    let best = nights
                        .get(s.game.night as usize - 1)
                        .cloned()
                        .unwrap_or_default();
                    format!(
                        "Midnight Countdown, night {}: {}, {} open, {best}",
                        s.game.night,
                        NightSpec::new(s.game.night).name,
                        s.game.progress.nights_open()
                    )
                }),
                |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.push(CountRun::new(Mode::Free, s.game.night))
                },
            )
            .adjustable(|s: &mut Title, _ctx: &mut Ctx<Parlor>, delta: i32| {
                let open = s.game.progress.nights_open() as i32;
                s.game.night = (s.game.night as i32 + delta).clamp(1, open) as u32;
            })
            .help(format!(
                "A rhythm game for New Year's Eve: answer the band on the beat to midnight. Reaching midnight opens the next night, up to {NIGHTS}. Left and Right choose the night."
            )),
        ]
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new(
                Label::dynamic(|s: &Title, _| {
                    format!(
                        "Practice rhythm, night {}: {}",
                        s.game.night,
                        NightSpec::new(s.game.night).name
                    )
                }),
                |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.push(CountRun::new(Mode::Practice, s.game.night))
                },
            )
            .adjustable(|s: &mut Title, _ctx: &mut Ctx<Parlor>, delta: i32| {
                let open = s.game.progress.nights_open() as i32;
                s.game.night = (s.game.night as i32 + delta).clamp(1, open) as u32;
            })
            .help("A night of Midnight Countdown where the party never fizzles and nothing is saved. Left and Right choose any night you have opened."),
        )
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your best story, Gift Sort, Wish Desk and Midnight Countdown scores, each stack and night, your holiday keepsakes and your challenge results.".to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Hear every gift's shake and every cue, each with its name.".to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "A daily run with a twist, the weekly holiday double and the monthly midwinter post, the same for every player. {}",
            daily.resets()
        )
    }

    /// A player who has saved a score is no longer new, tutorial or not.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && Progress::load(ctx) == Progress::default()
    }

    fn intro_help(&self) -> Option<String> {
        Some("Up and Down choose, Left and Right choose the shift, stack or night, Enter starts, Escape goes back to the parlor.".to_string())
    }
}
