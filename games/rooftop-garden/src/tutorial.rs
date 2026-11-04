//! The tutorial: a small practice garden, planted, watered and tended. It is
//! never saved over your own.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction, Rng};

use crate::rules::{Garden, Plant, Species, Stage};
use crate::screens::RunScreen;
use crate::ID;

/// The planter with the aphids: the squash, row 1, planter 1.
const SQUASH: usize = 0;

/// The practice garden: the starting rooftop, with aphids on the squash.
pub fn plot() -> Live<RunScreen> {
    let mut garden = Garden::home(Rng::seeded(1));
    let mut squash = Plant::at(Species::Squash, Stage::Growing);
    squash.pest = true;
    garden.plots[SQUASH] = Some(squash);
    Live::new(RunScreen::practice(garden))
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, plot())
}

/// The tutorial on `plot`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, plot: Live<RunScreen>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (u, d, l, r, w, t, s, h) = (
        k(ParlorAction::Up),
        k(ParlorAction::Down),
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Primary),
        k(ParlorAction::Secondary),
        k(ParlorAction::Status),
        k(ParlorAction::Hint),
    );

    let walk = plot.step(
        format!(
            "{}, {d}, {l} and {r} step between planters. Row 1 is the far row; planter 1 is on the left. A hollow knock means empty soil. Step onto empty soil.",
            sentence_start(&u)
        ),
        // The game has just said "…: empty soil.", so the step adds nothing.
        |s: &RunScreen, _| s.garden().plant_here().is_none().then(String::new),
    );

    // Planting means a new plant, not stepping onto the tomato already here.
    let count = |s: &RunScreen| s.garden().plots.iter().flatten().count();
    let planted = plot.with(count);
    let sow = plot.step(
        format!(
            "{} opens your seed packet: {u} and {d} choose, Enter plants.",
            sentence_start(&w)
        ),
        move |s: &RunScreen, _| {
            (count(s) > planted && s.garden().plant_here().is_some())
                .then(|| "Seeds are silent; sprouts hum.".to_string())
        },
    );

    let water = plot.step(
        format!(
            "{} also waters. Dry soil crackles, and a thirsty plant droops in pitch. Water your seed.",
            sentence_start(&w)
        ),
        |s: &RunScreen, _| {
            s.garden()
                .plant_here()
                .is_some_and(|p| p.water >= 1.0)
                .then(|| "Wait for the crackle before watering again: too much stops growth for a while.".to_string())
        },
    );

    let tend = plot.step(
        format!(
            "A buzz means aphids, and the plant stops growing. The squash in row 1, planter 1 has them: {t} picks them off there."
        ),
        |s: &RunScreen, _| {
            s.garden().plots[SQUASH]
                .as_ref()
                .is_some_and(|p| !p.pest)
                .then(String::new)
        },
    );

    Lesson::new(
        ID,
        "Rooftop Garden",
        "Grow a rooftop garden whose plants make music together. Learn to plant seeds, water them, and remove pests in this practice garden",
    )
    .step(walk)
    .step(sow)
    .step(water)
    .step(tend)
    .outro(format!(
        "A sparkle means ripe: {t} harvests. Enter sits on the bench to pass time faster. {} says what needs you; {h} points to the nearest job.",
        sentence_start(&s)
    ))
}
