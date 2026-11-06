//! Game rules with no screen, sound or clock, shared by the games and by
//! the server that replays a posted run to work out its score.
//!
//! What lives here must give the same result from the same seed and inputs
//! on every computer and in WebAssembly: randomness only from [`Rng`], time
//! only as fixed steps, and no `sin`, `cos`, `exp` or `powf` in anything
//! that decides a score.

mod period;
mod rng;
pub mod text;

pub use period::Period;
pub use rng::Rng;

#[cfg(wickets_secrets)]
include!("../../../secrets/rules/lib.rs");
