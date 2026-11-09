//! Dustfall: hold five domes on a cold moon against raiders that find their
//! own way down to you.
//!
//! A turn-based tactics game. Raiders arrive on the horizon of a five by
//! six approach grid and pick their own route toward the domes, round
//! walls and past flak posts. On your turn you slide the rail gun along
//! the near edge, fire up its column, launch a missile or stop early to
//! charge the shield; then every raider takes its step. Between nights you
//! spend power, metal and food on buildings under the columns and works on
//! the grid, which change the routes the raiders will take. Twelve nights
//! to Relief.
//!
//! This crate holds the rules so far (`rules`); the screens, sounds and
//! music follow in their own changes.

pub mod rules;

/// The game's id: its sounds live under `sounds/dustfall/`.
pub const ID: &str = "dustfall";
