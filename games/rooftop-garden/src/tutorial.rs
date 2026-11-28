//! The tutorial: a small practice garden, planted, watered and tended. It is
//! never saved over your own.

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
            "Use {u}, {d}, {l}, and {r} to step between planters. Row 1 is the far row, and planter 1 is on the left. Step onto an empty planter."
        ),
        // The game has just said "…: empty soil.", so the step adds nothing.
        |s: &RunScreen, _| s.garden().plant_here().is_none().then(String::new),
    );

    // Planting means a new plant, not stepping onto the tomato already here.
    let count = |s: &RunScreen| s.garden().plots.iter().flatten().count();
    let planted = plot.with(count);
    let sow = plot.step(
        format!(
            "Press {w} to open your seed packet, pick a seed with {u} and {d}, and press Enter to plant it."
        ),
        move |s: &RunScreen, _| {
            (count(s) > planted && s.garden().plant_here().is_some())
                .then(|| "Seeds sprout after part of a day.".to_string())
        },
    );

    let water = plot.step(
        format!("Press {w} on a planted planter to water it. Give your new seed a drink."),
        |s: &RunScreen, _| {
            s.garden()
                .plant_here()
                .is_some_and(|p| p.water >= 1.0)
                .then(|| {
                    "Wait to water again until a plant gets thirsty, because watering moist soil waterlogs it and pauses growth for a while."
                        .to_string()
                })
        },
    );

    let tend = plot.step(
        format!(
            "Aphids stop a plant from growing. The squash in row 1, planter 1 has aphids right now: step over there and press {t} to pick them off."
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
        "Turn a city rooftop into a garden whose plants play music together. Learn how to plant seeds, water soil, and pick off pests in this practice garden",
    )
    .step(walk)
    .step(sow)
    .step(water)
    .step(tend)
    .outro(format!(
        "When a plant ripens, press {t} to harvest it. Harvesting beside a different blooming or ripe plant can cross them into new seeds. Press Enter to sit on the bench and pass time faster, {s} to check what needs you, or {h} to point to the nearest job."
    ))
}
