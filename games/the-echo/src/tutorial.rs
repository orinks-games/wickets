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

/// Waits for what the game says to be heard before a step ends, so the
/// lesson's next words do not cut it off.
#[derive(Default)]
struct Heard {
    /// `ctx.said` when last looked.
    said: u64,
    /// When the lines said so far should be done.
    until: f64,
    /// Whether it has looked yet: what was said before the step began is
    /// the last step's business.
    started: bool,
}

impl Heard {
    /// Whether everything said since `armed` first held has been heard.
    /// Lines said in the frame that arms it count.
    fn after(&mut self, armed: bool, ctx: &Ctx<Parlor>) -> bool {
        // On the first look, what was said before the step began is the
        // last step's business; a step armed from the start is done.
        if !armed || !self.started {
            self.started = true;
            self.said = ctx.said;
            self.until = ctx.clock;
            return armed;
        }
        let new = ((ctx.said - self.said) as usize).min(ctx.transcript.len());
        let lines = &ctx.transcript[ctx.transcript.len() - new..];
        for line in lines.iter().filter(|l| !l.starts_with("[event]")) {
            self.until = self.until.max(ctx.clock) + speech_secs(line);
        }
        self.said = ctx.said;
        ctx.clock >= self.until
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
            "Two tones are playing: the pipe, and the note it should hold. While the pipe is out, they wobble against each other. {} raises the pipe and {down} lowers it. Find the way that slows the wobble, keep going until it is still, then {p} sets the pipe.",
            sentence_start(&up)
        ),
        {
            let mut heard = Heard::default();
            move |t: &Tuning, ctx| heard.after(t.rank.tuned() >= 1, ctx).then(String::new)
        },
    );
    let second = rank.step(
        format!(
            "The next pipe is higher. Tune it the same way. {} says which way it is out, and {tab} plays the true note alone.",
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
        "Theo's notes are open on the cellar bench: tune the Echo's pipes low to high, and listen for the wobble to go still",
    )
    .step(first)
    .step(second)
    .outro(format!(
        "A rank has several pipes, low to high, and {} moves up it by hand. In a scored rank, a hint or {p} too soon costs a little, but nothing fails.",
        right
    ))
}
