//! The tutorial: one landing at the real table, with the dice set to
//! teach.

use std::cell::Cell;
use std::rc::Rc;

use hearken::text::sentence_start;
use hearken::{Ctx, InputEvent};
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::rules::{Dice, Night, Roll, LANDINGS, ONE_ROLLS, SALOON};
use crate::table::TableScreen;
use crate::ID;

/// The table the lesson plays, with its dice set: two good rolls, then a
/// 1 that ends the landing.
pub fn table() -> Live<TableScreen> {
    let dice = Dice::scripted([
        Roll::new(3, 4),
        Roll::new(5, 6),
        Roll::new(2, 3),
        Roll::new(6, 1),
    ]);
    Live::new(TableScreen::with_night(Night::with_dice(SALOON, dice)).lesson())
}

/// Keeps the landing on the lesson's script: two rolls, a bank, then the
/// 1 while you stand. `first` holds while the first step is open: once
/// its roll is in, the table waits for the next step.
fn guard(
    s: &TableScreen,
    ctx: &mut Ctx<Parlor>,
    event: &InputEvent,
    first: bool,
) -> Option<String> {
    let action = ctx.action_for(event)?;
    if event.is_repeat() || !matches!(action, ParlorAction::Primary | ParlorAction::Secondary) {
        return None;
    }
    let n = &s.table.night;
    let key = |a| sentence_start(&ctx.control_name(a));
    if first && !n.history.is_empty() && !s.table.busy() {
        Some("One moment.".to_string())
    } else if n.landing > 1 {
        Some("The landing is over.".to_string())
    } else if !n.you().seated {
        Some("Wait for the 1.".to_string())
    } else if action == ParlorAction::Secondary && n.history.len() < 2 {
        Some(format!("Not yet. {} rolls.", key(ParlorAction::Primary)))
    } else if action == ParlorAction::Primary && n.history.len() >= 2 && !s.table.busy() {
        Some(format!("Bank now. {} banks.", key(ParlorAction::Secondary)))
    } else {
        None
    }
}

/// Seconds a line takes to say, near enough.
fn speech_secs(text: &str) -> f64 {
    0.4 + text.chars().count() as f64 / 15.0
}

/// Waits for what the table says about the move that finished a step, so
/// the lesson's next words do not cut off the dealer. His lines are waited
/// for as they come, until he is quiet. After that, only a key can say
/// more, and a key ends the wait at once: the player's presses never hold
/// the next step back.
#[derive(Default)]
struct Heard {
    /// `ctx.said` when last looked.
    said: u64,
    /// When the lines said so far should be done.
    until: f64,
    /// Whether the step's move has been made.
    armed: bool,
    /// `ctx.said` when the table went quiet after the move.
    quiet_at: Option<u64>,
}

impl Heard {
    /// Whether the step can end: `done` holds, the table is `quiet`, and
    /// what was said since the move has been heard or cut off by a later
    /// key. Lines said in the frame of the move count.
    fn after(&mut self, done: bool, quiet: bool, ctx: &Ctx<Parlor>) -> bool {
        if !done {
            *self = Heard {
                said: ctx.said,
                until: ctx.clock,
                ..Heard::default()
            };
            return false;
        }
        self.armed = true;
        if self.quiet_at.is_some_and(|s| ctx.said > s) {
            return true;
        }
        let new = ((ctx.said - self.said) as usize).min(ctx.transcript.len());
        let lines = &ctx.transcript[ctx.transcript.len() - new..];
        for line in lines.iter().filter(|l| !l.starts_with("[event]")) {
            self.until = self.until.max(ctx.clock) + speech_secs(line);
        }
        self.said = ctx.said;
        if quiet && self.quiet_at.is_none() {
            self.quiet_at = Some(ctx.said);
        }
        quiet && ctx.clock >= self.until
    }
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, table())
}

/// The tutorial on `table`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, table: Live<TableScreen>) -> Lesson {
    // Whether the first step is still open.
    let first = Rc::new(Cell::new(true));
    let open = Rc::clone(&first);
    let table = table.guard(move |s, ctx, event| guard(s, ctx, event, open.get()));
    let k = |a| ctx.control_name(a);
    let (l, u, p, t, h) = (
        k(ParlorAction::Left),
        k(ParlorAction::Up),
        k(ParlorAction::Primary),
        k(ParlorAction::Secondary),
        k(ParlorAction::Hint),
    );

    // Each step ends once the dealer has said his piece and it has been
    // heard.
    let mut heard = Heard::default();
    let roll = table.step(
        format!(
            "Press {p} to call a roll. The dealer shakes two dice, calls the faces and their sum, and puts that sum on the table. Go ahead and call a roll."
        ),
        move |s: &TableScreen, ctx| {
            let rolled = !s.table.night.history.is_empty();
            heard.after(rolled, !s.table.busy(), ctx).then(String::new)
        },
    );

    let mut heard = Heard::default();
    let bank = table.step(
        format!(
            "Press {p} to stay in for another roll and grow the pot, or press {t} to bank what the table holds and stand until the next landing. On your first night, press {t} twice to confirm your first bank. Roll once more, then bank."
        ),
        move |s: &TableScreen, ctx| {
            let banked = s.table.night.you().banked.is_some();
            if banked && !heard.armed {
                // The table rolls on by itself: hold it until the next
                // step has been read.
                s.hold(f64::INFINITY);
            }
            // The held table says nothing more by itself.
            heard.after(banked, true, ctx).then(String::new)
        },
    )
    .start(move |_ctx| first.set(false));

    let one_prompt = format!(
        "A 1 on either die ends the landing, and everyone still seated loses what the table holds. {ONE_ROLLS} of 36 rolls show a 1, and {u} gives the odds anytime. You already banked, so wait for the 1."
    );
    let wait = speech_secs(&one_prompt);
    let held = table.clone();
    let mut heard = Heard::default();
    let one = table
        .step(one_prompt, move |s: &TableScreen, ctx| {
            let n = &s.table.night;
            let ended = n.last.as_ref().is_some_and(|r| r.end.is_some());
            heard.after(ended, !s.table.busy(), ctx).then(|| {
                let kept = n.you().night;
                if kept > 0 {
                    format!("Your banked {kept} is safe.")
                } else {
                    "The landing is over.".to_string()
                }
            })
        })
        .start(move |_ctx| held.with_mut(|s| s.hold(wait)));

    Lesson::new(
        ID,
        "Riverboat Dice",
        format!(
            "Pull up a chair at a paddle steamer dice table with three fellow passengers. Whoever banks the most over {LANDINGS} landings wins. Try one landing here to get the feel of it"
        ),
    )
    .step(roll)
    .step(bank)
    .step(one)
    .outro(format!(
        "Press {l} to check each passenger and their total, or {h} to ask what a careful player would do. Play starts a full night."
    ))
}
