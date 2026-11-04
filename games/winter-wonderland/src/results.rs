//! The end of a run: its result pages, then the shared results menu.

use hearken::Ctx;
use parlor_core::menus::{labels, Back, Results};
use parlor_core::{Pages, Parlor};

use crate::challenge::{Plan, CHALLENGES_NAME};
use crate::count_run::CountRun;
use crate::mode::Mode;
use crate::night::{NightSpec, NIGHTS};
use crate::sort_run::SortRun;
use crate::title::story_run;
use crate::TITLE_NAME;

/// Replaces the run with its result pages, then the results menu.
pub fn show(ctx: &mut Ctx<Parlor>, pages: Vec<String>, results: Results) {
    let mut results = Some(results);
    ctx.replace(Pages::new(pages).then(move |ctx| {
        if let Some(results) = results.take() {
            ctx.replace(results);
        }
    }));
}

fn again(won: bool) -> &'static str {
    if won {
        labels::PLAY_AGAIN
    } else {
        labels::TRY_AGAIN
    }
}

/// After the story, or the blizzard story.
pub fn story(blizzard: bool, won: bool, summary: String, pages: Vec<String>) -> Results {
    let help = if blizzard {
        "The blizzard story again, from shift 4."
    } else {
        "The story again, from shift 1."
    };
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(again(won), help, move |ctx| {
        ctx.replace(story_run(blizzard))
    })
}

/// After a Gift Sort run from the title, which started at shift `first`.
pub fn gift_sort(first: u32, won: bool, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
        again(won),
        format!("Gift Sort again, from shift {first}."),
        move |ctx| ctx.replace(SortRun::new(Mode::Free, first)),
    )
}

/// After a night of Midnight Countdown from the title, or of practice.
pub fn countdown(
    level: u32,
    practice: bool,
    won: bool,
    summary: String,
    pages: Vec<String>,
) -> Results {
    let mode = if practice { Mode::Practice } else { Mode::Free };
    let mut results = Results::new(summary, pages, Back::Title(TITLE_NAME));
    if won && !practice && level < NIGHTS {
        let next = level + 1;
        let next_mode = mode.clone();
        results = results.next(
            format!("Next night: night {next}, {}", NightSpec::new(next).name),
            "A faster band with trickier rhythms.",
            move |ctx| ctx.replace(CountRun::new(next_mode.clone(), next)),
        );
    }
    let help = format!("Night {level}, {}, again.", NightSpec::new(level).name);
    results.again(again(won), help, move |ctx| {
        ctx.replace(CountRun::new(mode.clone(), level))
    })
}

/// After a challenge run: the same challenge again, or back to the
/// challenges.
pub fn challenge(plan: &Plan, summary: String, pages: Vec<String>) -> Results {
    let period = plan.period();
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same run, straight away. Only your best result is kept.",
        move |ctx| Plan::current(ctx, period).start(ctx),
    )
}
