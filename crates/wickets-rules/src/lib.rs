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
