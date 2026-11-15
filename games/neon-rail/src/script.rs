//! What comes down the rail: the challenge build's authored corridor and
//! guided route, note for note, and the sector builder a Black Sky Run
//! uses for the routes the player picks.

use crate::rules::{Kind, Lane, Route, Run, Twist, APPROACH, BOOST_COOLDOWN, PICKUP_APPROACH};

use Kind::*;
use Lane::{All, Center, Left, Right};

/// The original two-minute, six-sector corridor.
pub fn challenge(run: &mut Run) {
    run.push_sector(
        0.0,
        20.0,
        "Sector 1. Departure rail",
        "Calm commuter flow. Learn the route before congestion starts stacking.",
    );
    run.push_sector(
        20.0,
        40.0,
        "Sector 2. Freight merge",
        "Fast traffic begins to cut through the corridor.",
    );
    run.push_sector(
        40.0,
        60.0,
        "Sector 3. Signal tunnel",
        "Priority traffic arrives. Pressure rises faster now.",
    );
    run.push_sector(
        60.0,
        80.0,
        "Sector 4. Scrap spill",
        "Heavy blockers and unstable vehicles contest the lanes.",
    );
    run.push_sector(
        80.0,
        100.0,
        "Sector 5. Security cordon",
        "Dense mixed flow. Use emergency tools carefully.",
    );
    run.push_sector(
        100.0,
        120.0,
        "Sector 6. Extraction sprint",
        "Sustained overload conditions. Survive the final corridor.",
    );

    let p = PICKUP_APPROACH;
    let a = APPROACH;
    let script: &[(f64, Kind, Lane, i32, f64)] = &[
        (2.5, Flow, Center, 12, 1.7),
        (5.6, Salvage, Left, 60, p),
        (9.0, Debris, Right, 0, a),
        (12.6, FastPasser, Center, 18, 1.2),
        (16.8, Repair, Right, 16, p),
        (21.4, Drone, Left, 0, a),
        (24.8, Flow, Right, 14, 1.6),
        (28.2, FastPasser, Left, 22, 1.1),
        (31.6, Blockade, All, 0, 1.9),
        (35.1, HeavyBlocker, Center, 0, 2.1),
        (38.7, Salvage, Center, 75, p),
        (42.2, Priority, Right, 0, 1.3),
        (45.4, Debris, Left, 0, a),
        (48.6, Unstable, Center, 0, 1.8),
        (52.0, Flow, Left, 16, 1.5),
        (55.4, Repair, Center, 18, p),
        (58.6, FastPasser, Right, 24, 1.0),
        (62.0, HeavyBlocker, Left, 0, 2.2),
        (65.2, Drone, Right, 0, a),
        (68.4, Salvage, Center, 90, p),
        (71.6, Priority, Left, 0, 1.2),
        (74.8, Blockade, All, 0, 1.8),
        (78.0, Unstable, Right, 0, 1.7),
        (81.2, Flow, Right, 18, 1.5),
        (84.2, HeavyBlocker, Center, 0, 2.1),
        (87.2, FastPasser, Left, 26, 0.95),
        (90.2, Priority, Center, 0, 1.15),
        (93.4, Drone, Left, 0, a),
        (96.6, Repair, Right, 20, p),
        (99.5, Blockade, All, 0, 1.8),
        (102.2, Unstable, Left, 0, 1.6),
        (104.9, FastPasser, Right, 28, 0.9),
        (107.4, HeavyBlocker, Center, 0, 2.0),
        (110.3, Priority, Right, 0, 1.05),
        (113.0, Drone, Center, 0, a),
        (115.5, Salvage, Left, 110, p),
        (118.0, Blockade, All, 0, 1.7),
    ];
    for &(at, kind, lane, value, approach) in script {
        run.push_event(at, kind, lane, value, approach);
    }
}

/// The original guided route, with one blockade added so the boost chime
/// is taught before it is needed. The debris and the drone come down the
/// lane the earlier steps leave the player in.
pub fn tutorial(run: &mut Run) {
    run.push_sector(
        0.0,
        20.0,
        "Tutorial 1. Departure rail",
        "Learn lane movement and flow traffic.",
    );
    run.push_sector(
        20.0,
        40.0,
        "Tutorial 2. Tools and traffic",
        "Practice boost, rail gun, and pickups.",
    );
    run.push_sector(
        40.0,
        60.0,
        "Tutorial 3. Pressure recovery",
        "Priority traffic and stress recovery.",
    );
    let p = PICKUP_APPROACH;
    let script: &[(f64, Kind, Lane, i32, f64)] = &[
        (6.0, Flow, Center, 12, 2.2),
        (12.5, Debris, Center, 0, 2.4),
        (19.0, Repair, Left, 12, p),
        (26.0, Drone, Left, 0, 2.6),
        (33.5, HeavyBlocker, Left, 0, 2.6),
        (37.5, Blockade, All, 0, 2.0),
        (42.5, Priority, Right, 0, 2.2),
        (50.0, Salvage, Center, 80, p),
        (55.5, Unstable, Left, 0, 2.3),
    ];
    for &(at, kind, lane, value, approach) in script {
        run.push_event(at, kind, lane, value, approach);
    }
}

/// How often each kind turns up on a route.
fn weights(route: Route) -> &'static [(Kind, f64)] {
    match route {
        Route::Departure => &[
            (Flow, 3.0),
            (Salvage, 1.5),
            (Debris, 1.5),
            (FastPasser, 1.0),
            (Repair, 0.8),
            (Drone, 0.8),
        ],
        Route::Salvage => &[
            (Salvage, 4.0),
            (Repair, 2.0),
            (Flow, 2.0),
            (Debris, 1.5),
            (FastPasser, 1.0),
            (Drone, 1.0),
            (HeavyBlocker, 0.5),
            (Priority, 0.5),
        ],
        Route::Ambush => &[
            (Drone, 4.0),
            (Unstable, 3.0),
            (Debris, 2.0),
            (HeavyBlocker, 1.5),
            (Blockade, 1.0),
            (Flow, 1.0),
            (Salvage, 0.5),
            (Repair, 0.5),
        ],
        Route::Storm => &[
            (FastPasser, 3.0),
            (Flow, 2.0),
            (Priority, 2.0),
            (Debris, 2.0),
            (Unstable, 1.5),
            (HeavyBlocker, 1.0),
            (Blockade, 1.0),
            (Repair, 0.5),
        ],
        Route::Relay => &[
            (Flow, 3.0),
            (Priority, 2.0),
            (Salvage, 1.0),
            (Repair, 1.0),
            (Debris, 1.0),
        ],
        Route::Convoy => &[
            (Priority, 3.0),
            (Flow, 2.0),
            (Debris, 1.5),
            (Drone, 1.5),
            (FastPasser, 1.2),
            (HeavyBlocker, 1.0),
            (Unstable, 1.0),
            (Salvage, 0.6),
            (Repair, 0.6),
            (Blockade, 0.5),
        ],
        Route::Extraction => &[
            (Flow, 1.5),
            (FastPasser, 2.0),
            (HeavyBlocker, 1.5),
            (Priority, 2.0),
            (Unstable, 1.5),
            (Debris, 1.5),
            (Drone, 1.5),
            (Blockade, 1.0),
            (Repair, 0.7),
            (Salvage, 0.7),
        ],
        Route::Standard | Route::Scripted => &[
            (Flow, 3.0),
            (FastPasser, 2.0),
            (Debris, 2.0),
            (Drone, 2.0),
            (HeavyBlocker, 1.5),
            (Priority, 1.5),
            (Unstable, 1.0),
            (Salvage, 1.0),
            (Repair, 0.7),
            (Blockade, 0.6),
        ],
    }
}

/// Space between arrivals, and warning time, relative to a standard rush.
fn pacing(route: Route) -> (f64, f64) {
    match route {
        Route::Departure => (1.15, 1.1),
        Route::Salvage => (1.1, 1.0),
        Route::Relay => (1.25, 1.05),
        Route::Storm => (0.9, 0.8),
        Route::Ambush => (0.95, 0.95),
        Route::Extraction => (0.85, 0.9),
        Route::Convoy => (1.05, 1.0),
        Route::Standard | Route::Scripted => (1.0, 1.0),
    }
}

/// The original's warning time for each kind.
fn base_approach(kind: Kind) -> f64 {
    match kind {
        Debris | Drone => APPROACH,
        Blockade => 1.9,
        Repair | Salvage => PICKUP_APPROACH,
        Flow => 1.6,
        FastPasser => 1.1,
        HeavyBlocker => 2.1,
        Priority => 1.25,
        Unstable => 1.7,
    }
}

fn briefing(route: Route, pick: u32) -> &'static str {
    let lines: &[&str] = match route {
        Route::Departure => {
            &["Calm commuter flow. Learn the route before congestion starts stacking."]
        }
        Route::Standard => &[
            "Freight and commuter lines merge. Mixed traffic, steady pressure.",
            "A signal tunnel under the old transit spire. Expect a bit of everything.",
        ],
        Route::Salvage => &[
            "A scrap spill from a derailed hauler. Salvage everywhere, threats thin.",
            "The salvage unions left this line open. Pick it clean.",
        ],
        Route::Ambush => &[
            "Transit Authority hunters waiting in the dark. Drones inbound.",
            "Corporate security has armed the crossings. Keep the rail gun hot.",
        ],
        Route::Storm => &[
            "Ion storm over the partition wall. Warnings come late.",
            "The storm rail runs fast and blind. Short windows all the way.",
        ],
        Route::Relay => &[
            "A dead data relay stands beside the rail. Light traffic.",
            "The unknown signal is strongest here. Priority traffic carries it.",
        ],
        Route::Extraction => &["Sustained overload conditions. Survive the final corridor."],
        Route::Convoy => &[
            "The evacuation trains are moving under clean codes. Keep their lanes open.",
            "Hunters shadow the convoy through the dark districts. Every train you clear is a carriage of people out.",
            "The partition gates are open for ninety seconds. Get the trains through.",
        ],
        Route::Scripted => &["The corridor."],
    };
    lines[pick as usize % lines.len()]
}

/// Builds sector `index` along `route` from `start`, adding its events to
/// the run. Gives the sector's end, name and briefing.
pub fn generate(run: &mut Run, index: usize, start: f64, route: Route) -> (f64, String, String) {
    // Difficulty climbs with every sector. Past the sixth or so (the
    // endless rail) it keeps climbing, more slowly, toward a floor a
    // practised ear can still read.
    let level = index + run.depth;
    let d = level as f64 * 1.2;
    let deep = (d - 7.0).max(0.0);
    let length = run
        .sector_length
        .unwrap_or_else(|| (30.0 + 4.0 * level as f64).min(48.0));
    let end = start + length;
    let (mut gap_mul, mut approach_mul) = pacing(route);
    let no_boost = run.twist == Some(Twist::NoBoost);
    if let Some(twist) = run.twist {
        gap_mul *= twist.gap_factor();
        approach_mul *= twist.warning_factor();
    }
    let gap = (3.4 - 0.3 * d).max(1.3 - 0.03 * deep).max(1.0) * gap_mul;
    let warn = (1.0 - 0.05 * d).max(0.65 - 0.01 * deep).max(0.5) * approach_mul;
    // The least warning anything gets: enough to hear a short call-out
    // and change lanes.
    let mut shortest = if deep > 0.0 { 0.8 } else { 0.95 };
    if run.twist == Some(Twist::Fog) {
        shortest *= 0.8;
    }
    let table = weights(route);
    let total: f64 = table.iter().map(|(_, w)| w).sum();

    let mut spawn = start + 3.0;
    let mut last_resolve = start;
    let mut last_blockade = f64::NEG_INFINITY;
    loop {
        let mut roll = run.rng().unit() * total;
        let mut kind = table[0].0;
        for &(k, w) in table {
            if roll < w {
                kind = k;
                break;
            }
            roll -= w;
        }
        if kind == Blockade && spawn - last_blockade < BOOST_COOLDOWN + 2.0 {
            kind = Flow;
        }
        if kind == Blockade && no_boost {
            kind = HeavyBlocker;
        }
        let approach = (base_approach(kind) * warn).max(shortest);
        let mut resolve = spawn + approach;
        if resolve < last_resolve + 0.95 {
            spawn += last_resolve + 0.95 - resolve;
            resolve = spawn + approach;
        }
        if resolve > end - 0.6 {
            break;
        }
        let lane = if kind == Blockade {
            All
        } else {
            Lane::from_index(run.rng().below(3) as usize)
        };
        let step = level as i32;
        let value = match kind {
            Flow => 12 + 2 * step,
            FastPasser => 18 + 2 * step,
            Salvage => 60 + 15 * step,
            Repair => 16 + 2 * step,
            _ => 0,
        };
        if kind == Blockade {
            last_blockade = spawn;
        }
        run.push_event(spawn, kind, lane, value, approach);
        last_resolve = resolve;
        spawn += gap * (0.8 + 0.45 * run.rng().unit());
    }
    if route == Route::Extraction
        && !no_boost
        && end - last_resolve > 2.5
        && end - 2.6 - last_blockade > 7.0
    {
        run.push_event(end - 2.6, Blockade, All, 0, 1.8);
    }

    let pick = run.rng().below(8);
    let name = format!("Sector {}. {}", index + 1, route.name());
    (end, name, briefing(route, pick).to_string())
}
