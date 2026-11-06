//! The stream's calendar: a duck parade and a lantern float, once a month
//! each. Both touch your own streams and the endless river, never a
//! challenge.
//!
//! - Duck parade, the second Monday: more ducks paddle across every
//!   stream, and each one you pass is worth points.
//! - Lantern float, the third Friday: lanterns float down every stream,
//!   worth five petals each.

use parlor_core::almanac::{Week, Window};
use parlor_core::{Date, Happening};

/// The second Monday of every month.
pub const DUCK_PARADE: Window = Window::Monthly(Week::Second, 0);
/// The third Friday of every month.
pub const LANTERN_FLOAT: Window = Window::Monthly(Week::Third, 4);

pub fn duck_parade(date: Date) -> bool {
    DUCK_PARADE.is_open(date)
}

pub fn lantern_float(date: Date) -> bool {
    LANTERN_FLOAT.is_open(date)
}

/// What the launcher names while an event is on.
pub fn event(date: Date) -> Option<Happening> {
    if duck_parade(date) {
        return Some(Happening::new(
            "Duck parade",
            "Ducks paddle across every stream today, and each one you pass is worth points.",
        ));
    }
    if lantern_float(date) {
        return Some(Happening::new(
            "Lantern float",
            "Lanterns float down every stream today, worth five petals each.",
        ));
    }
    None
}

/// Today's touch for a run's opening line, if any.
pub fn opening(date: Date) -> Option<&'static str> {
    if duck_parade(date) {
        Some("Duck parade today.")
    } else if lantern_float(date) {
        Some("Lantern float today.")
    } else {
        None
    }
}
