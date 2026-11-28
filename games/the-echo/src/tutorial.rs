//! The tutorial: Theo's notes, by ear, on a rank of two pipes.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::play::Tuning;
use crate::rules::PRACTICE;
use crate::ID;

/// The tutorial's two pipes, as note and steps out: G4 four steps sharp,
/// then D5 three steps flat.
pub const LESSON_PIPES: [(i32, i32); 2] = [(0, 4), (7, -3)];

/// The rank the lesson tunes.
pub fn rank() -> Live<Tuning> {
    Live::new(Tuning::lesson_rank(PRACTICE, &LESSON_PIPES))
}

/// Seconds a line takes to say, near enough.
fn speech_secs(text: &str) -> f64 {
    0.4 + text.chars().count() as f64 / 15.0
}

/// Waits for what the game says about the move that finished a step, so
/// the lesson's next words do not cut it off. Only that move's lines are
/// waited for: a key pressed after it has already cut them off, so the
/// next step comes at once, and the player's presses never hold it back.
#[derive(Default)]
struct Heard {
    /// `ctx.said` when last looked; none before the step's first frame.
    said: Option<u64>,
    /// When the finishing move's lines should be done, once it is made.
    until: Option<f64>,
}

impl Heard {
    /// Whether the step can end: `done` holds, and what the move that made
    /// it hold said has been heard or cut off by a later key.
    fn after(&mut self, done: bool, ctx: &Ctx<Parlor>) -> bool {
        if !done {
            self.said = Some(ctx.said);
            self.until = None;
            return false;
        }
        match self.until {
            None => {
                // The frame of the move: its lines start now. A step that
                // begins already done waits for its own prompt instead.
                let since = self.said.unwrap_or(ctx.said.saturating_sub(1));
                let new = ((ctx.said - since) as usize).min(ctx.transcript.len());
                let lines = &ctx.transcript[ctx.transcript.len() - new..];
                let until = lines
                    .iter()
                    .filter(|l| !l.starts_with("[event]"))
                    .fold(ctx.clock, |t, line| t + speech_secs(line));
                self.said = Some(ctx.said);
                self.until = Some(until);
                ctx.clock >= until
            }
            Some(until) => self.said.is_some_and(|s| ctx.said > s) || ctx.clock >= until,
        }
    }
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, rank())
}

/// The tutorial on `rank`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, rank: Live<Tuning>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (up, down, p, h, tab, right) = (
        k(ParlorAction::Up),
        k(ParlorAction::Down),
        k(ParlorAction::Primary),
        k(ParlorAction::Hint),
        k(ParlorAction::Secondary),
        k(ParlorAction::Right),
    );

    let first = rank.step(
        format!(
            "This pipe is out of tune with its target note, so it wobbles. {} raises the pipe and {down} lowers it. Find the direction that slows the wobble, keep going until it settles still, then press {p} to set the pipe.",
            sentence_start(&up)
        ),
        {
            let mut heard = Heard::default();
            move |t: &Tuning, ctx| heard.after(t.rank.tuned() >= 1, ctx).then(String::new)
        },
    );
    let second = rank.step(
        format!(
            "The next pipe is higher, and you tune it the same way. {} tells you which way it is out, and {tab} plays the true note on its own.",
            sentence_start(&h)
        ),
        {
            // The lesson ends once the Echo has played the rank back and
            // its words have been heard.
            let mut heard = Heard::default();
            move |t: &Tuning, ctx| {
                (heard.after(t.rank.finished(), ctx) && t.played_back()).then(String::new)
            }
        },
    );

    Lesson::new(
        ID,
        "The Echo",
        "Theo's notes lie open on the cellar bench: tune the Echo's pipes from low to high until every wobble settles still",
    )
    .step(first)
    .step(second)
    .outro(format!(
        "A full rank runs through several pipes from low to high, and {} steps up the rank by hand. In a scored rank, using a hint or pressing {p} too early costs a few points, though every tuned pipe still scores at least 2.",
        right
    ))
}
