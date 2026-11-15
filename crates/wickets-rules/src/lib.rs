//! Game rules with no screen, sound or clock, shared by the games and by
//! the server that replays a posted run to work out its score.
//!
//! What lives here must give the same result from the same seed and inputs
//! on every computer and in WebAssembly: randomness only from [`Rng`], time
//! only as fixed steps, and no `sin`, `cos`, `hypot`, `atan2`, `exp` or
//! `powf` from the standard library in anything that decides a score: each
//! computer's maths library rounds them its own way. The [`libm`] crate's
//! are pure Rust and the same everywhere.

pub mod fixed_step;
mod period;
mod rng;
pub mod text;

pub use fixed_step::FixedStep;
pub use period::Period;
pub use rng::Rng;

/// Maths the same to the last bit everywhere, for rule files to use.
pub use libm;

#[cfg(wickets_secrets)]
include!("../../../secrets/rules/lib.rs");

// Without the private overlay no game is ranked, but a release still builds
// a rules module, so it exports one set of rules that ranks nothing.
#[cfg(not(wickets_secrets))]
pub mod unranked {
    use hearken_replay::{InputCode, Replayable};

    /// No input: there is nothing to play.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Nothing {}

    impl InputCode for Nothing {
        fn to_code(&self) -> (u16, i32) {
            match *self {}
        }

        fn from_code(_: u16, _: i32) -> Option<Self> {
            None
        }
    }

    /// Rules that end before they begin.
    pub struct Unranked;

    impl Replayable for Unranked {
        const RULES: &'static str = "unranked";
        const MAX_TICKS: u64 = 1;
        type Input = Nothing;
        type Context = ();
        type Outcome = ();

        fn new(_: u64, _: &()) -> Self {
            Unranked
        }

        fn step(&mut self, _: &[Nothing]) {}

        fn finished(&self) -> bool {
            true
        }

        fn outcome(&self) {}
    }
}

#[cfg(not(wickets_secrets))]
hearken_replay::export_rules!(unranked::Unranked);
