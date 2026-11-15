//! The Wish Desk's calendar.
//!
//! - Muddled post, the fourth Wednesday of every month: the letters come
//!   from the youngest children, and every box is packed in straw, so each
//!   one needs two shakes.
//! - Late post, every evening by the player's clock: once an evening, a
//!   small extra sack of two letters arrives at the end of a stack, worth
//!   double.
//!
//! Both touch the Wish Desk from the title and the story, never a
//! challenge, which stays the same for every player.

use hearken::Ctx;
use parlor_core::almanac::{Week, Window};
use parlor_core::story::PartOfDay;
use parlor_core::{Date, Happening, Parlor};

use crate::progress::Progress;

/// Muddled post's row and title.
pub const MUDDLED_NAME: &str = "Muddled post";
/// The fourth Wednesday of every month.
pub const MUDDLED: Window = Window::Monthly(Week::Fourth, 2);
/// The best-score board for Muddled post.
pub const MUDDLED_BOARD: &str = "muddled post";
/// The stack Muddled post plays.
pub const MUDDLED_STACK: u32 = 3;

pub fn muddled(date: Date) -> bool {
    MUDDLED.is_open(date)
}

/// What the launcher names while Muddled post is on.
pub fn event(date: Date) -> Option<Happening> {
    muddled(date).then(|| {
        Happening::new(
            MUDDLED_NAME,
            "Letters from the youngest children at the Wish Desk, every box packed in straw.",
        )
    })
}

/// The Muddled post row's help while it is open.
pub fn muddled_help(best: Option<u64>) -> String {
    format!(
        "One stack of letters from the youngest children. Every box is packed in straw, so shake it twice.{}",
        best_words(best)
    )
}

/// What Enter on the locked Muddled post row says.
pub fn muddled_locked_line(today: Date) -> String {
    format!("Muddled post comes {}.", MUDDLED.next_words(today))
}

/// The locked Muddled post row's help.
pub fn muddled_locked_help(today: Date, best: Option<u64>) -> String {
    format!(
        "Comes {}, {}: letters from the youngest children, every box packed in straw.{}",
        MUDDLED.every(),
        MUDDLED.next_words(today),
        best_words(best)
    )
}

fn best_words(best: Option<u64>) -> String {
    best.map_or(String::new(), |b| format!(" Your best is {b}."))
}

/// A date as the progress file keeps it: "2026-11-25".
fn key(date: Date) -> String {
    format!("{}-{:02}-{:02}", date.year, date.month, date.day)
}

/// Whether it is evening by the player's clock: 5 to 9.
pub fn evening(ctx: &Ctx<Parlor>) -> bool {
    let now = parlor_core::story::now(ctx);
    PartOfDay::of_hour(now.hour) == PartOfDay::Evening
}

/// Whether tonight's late post has still to come.
pub fn late_post_due(ctx: &mut Ctx<Parlor>) -> bool {
    if !evening(ctx) {
        return false;
    }
    let today = key(parlor_core::challenges::today(ctx));
    Progress::load(ctx).late_post != today
}

/// Marks tonight's late post as come.
pub fn late_post_came(ctx: &mut Ctx<Parlor>) {
    let today = key(parlor_core::challenges::today(ctx));
    let mut p = Progress::load(ctx);
    p.late_post = today;
    p.store(ctx);
}
