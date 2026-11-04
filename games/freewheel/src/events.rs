//! Rides for special days.
//!
//! From 17 October to the end of November the title has a Leaf season
//! ride row: a kilometre of autumn Downtown, the same all season for every
//! player, with leaf piles in the bike lane. Each pile rustles ahead of
//! you, and riding through it crunches, muffles the freewheel for a moment
//! and adds 1000 ride points. Leaf piles are never hazards. On the third
//! Sunday of every month the Car-free Sunday row opens: a course made from
//! that Sunday's date, on streets closed to cars, where birds sing and a
//! church bell rings far off instead of the city. On other days that row
//! stays in its place, locked, and says when it opens. Each row keeps its
//! own best time and ride points for each bike, like the time trials.
//!
//! Everything here is a plain function of the date, so tests pin it.

use parlor_core::almanac::{Week, Window};
use parlor_core::{Date, Happening};

use crate::bike::Bike;
use crate::course::{Course, Mix, START_X};

/// The leaf season row's name.
pub const LEAF_SEASON: &str = "Leaf season ride";
/// Its boards' stem: each bike keeps its own, as [`board`] names them.
pub const LEAF_BOARD: &str = "leaf season";
/// When leaf season is open: 17 October to 30 November, the fall of the
/// leaves.
pub const LEAF_WINDOW: Window = Window::Range((10, 17), (11, 30));
/// The leaf season course's length, metres.
pub const LEAF_LENGTH: f64 = 1000.0;
/// Ride points for each leaf pile ridden through.
pub const LEAF_POINTS: u64 = 1000;
/// Leaf piles lie only in gaps this long between hazards and turns, in
/// the middle, so a pile's sounds never meet a hazard's warning.
pub const LEAF_GAP: f64 = 60.0;
/// Where a leaf pile lies across the road: the middle of the bike lane.
pub const LEAF_X: f64 = START_X + 0.2;

/// The Car-free Sunday row's name.
pub const SUNDAY: &str = "Car-free Sunday";
/// Its boards' stem: each bike keeps its own, as [`board`] names them.
/// The key is older than the row's name.
pub const SUNDAY_BOARD: &str = "new moon";
/// When it is open: the third Sunday of every month.
pub const SUNDAY_WINDOW: Window = Window::Monthly(Week::Third, 6);
/// The Car-free Sunday course's length, metres.
pub const SUNDAY_LENGTH: f64 = 1200.0;

/// The board an event ride on `bike` is kept on: `<stem>/<bike>`, so
/// each bike has its own best time and ride points, like the time trials.
pub fn board(stem: &str, bike: Bike) -> String {
    format!("{stem}/{}", bike.id())
}

/// What Play again says once an event ride has closed.
pub const RIDE_CLOSED: &str = "That ride has closed for now. It opens again on its next day.";

/// Par seconds for a course of `length` metres, worth 100,000 points.
pub fn par(length: f64) -> f64 {
    length / 7.5
}

/// Whether leaf season is on.
pub fn leaf_open(date: Date) -> bool {
    LEAF_WINDOW.is_open(date)
}

/// This year's leaf season course, the same for every player all season,
/// and its leaf piles: metres along and across.
pub fn leaf_course(date: Date) -> (Course, Vec<(f64, f64)>) {
    let seed = 0x4c45_4146_0000 ^ date.year as u64;
    let mix = Mix {
        gap: (LEAF_GAP, 90.0),
        ..Mix::DOWNTOWN
    };
    let course = Course::generate(LEAF_SEASON, seed, LEAF_LENGTH, mix);
    let piles = leaf_piles(&course);
    (course, piles)
}

/// Leaf piles in the middle of every long gap between the start, the
/// hazards, the turns and the finish.
pub fn leaf_piles(course: &Course) -> Vec<(f64, f64)> {
    let mut marks: Vec<f64> = vec![0.0, course.length];
    marks.extend(course.hazards.iter().map(|h| h.s));
    marks.extend(course.junctions.iter().map(|j| j.s));
    marks.sort_by(f64::total_cmp);
    marks
        .windows(2)
        .filter(|w| w[1] - w[0] >= LEAF_GAP)
        .map(|w| ((w[0] + w[1]) / 2.0, LEAF_X))
        .collect()
}

/// Whether Car-free Sunday is open on `date`.
pub fn sunday_open(date: Date) -> bool {
    SUNDAY_WINDOW.is_open(date)
}

/// Today's Car-free Sunday course, when it is open: seeded by the date, so
/// every rider gets one course, with no cars.
pub fn sunday_course(date: Date) -> Option<Course> {
    if !sunday_open(date) {
        return None;
    }
    let seed = 0x4e45_574d_0000_0000 ^ date.days() as u64;
    Some(Course::generate(SUNDAY, seed, SUNDAY_LENGTH, Mix::DOWNTOWN).quiet())
}

/// What Enter on the locked Car-free Sunday row says.
pub fn sunday_locked_line(date: Date) -> String {
    format!("Car-free Sunday opens {}.", SUNDAY_WINDOW.next_words(date))
}

/// What Enter on the locked Leaf season row says.
pub fn leaf_locked_line(date: Date) -> String {
    format!("Leaf season opens {}.", LEAF_WINDOW.next_words(date))
}

/// The locked Leaf season row's help.
pub fn leaf_locked_help(date: Date) -> String {
    format!(
        "Opens {}, until the end of November: autumn Downtown, with leaf piles in the bike lane.",
        LEAF_WINDOW.next_words(date)
    )
}

/// The locked Car-free Sunday row's help.
pub fn sunday_locked_help(date: Date) -> String {
    format!(
        "Opens {}, {}: Downtown with the streets closed to cars, and birdsong instead of the city.",
        SUNDAY_WINDOW.every(),
        SUNDAY_WINDOW.next_words(date)
    )
}

/// What the launcher names while a row is open.
pub fn event(date: Date) -> Option<Happening> {
    if sunday_open(date) {
        return Some(Happening::new(
            SUNDAY,
            "Ride today's course on streets closed to cars, to birdsong and a far church bell.",
        ));
    }
    leaf_open(date).then(|| {
        Happening::new(
            LEAF_SEASON,
            "Ride through crunchy leaf piles in the bike lane for 1000 ride points each, until the end of November.",
        )
    })
}

/// " Best 1 minute 20 seconds." style words come from the title; this is
/// the row's help without them.
pub fn leaf_help() -> String {
    format!(
        "Leaf season: {} of autumn Downtown, with leaf piles in the bike lane. Each one rustles ahead of you. Ride through it for a crunch and {LEAF_POINTS} ride points. Leaf piles are never hazards. Open until the end of November.",
        parlor_core::units::route(LEAF_LENGTH)
    )
}

/// The Car-free Sunday row's help.
pub fn sunday_help() -> String {
    format!(
        "Today's Car-free Sunday course, {}, the same for every player: the streets are closed to cars, and birds sing with a far church bell instead of the city. Open all day.",
        parlor_core::units::route(SUNDAY_LENGTH)
    )
}
