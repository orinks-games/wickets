//! Sound keys, where cues sit around the rider, and the sound check's
//! catalog. The keys are the asset contract: a recording of the same name
//! in `sounds/freewheel/` replaces a prototype with no code change.

use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::Parlor;

use crate::bike::Bike;
use crate::course::HazardKind;

pub const ROAD_ROLL: &str = "freewheel/bike/road/roll";
pub const ROAD_FREEHUB: &str = "freewheel/bike/road/freehub";
pub const ROAD_PEDAL: &str = "freewheel/bike/road/pedal";
pub const BMX_ROLL: &str = "freewheel/bike/bmx/roll";
pub const BMX_FREEHUB: &str = "freewheel/bike/bmx/freehub";
pub const BMX_PEDAL: &str = "freewheel/bike/bmx/pedal";
pub const MTB_ROLL: &str = "freewheel/bike/mtb/roll";
pub const MTB_FREEHUB: &str = "freewheel/bike/mtb/freehub";
pub const MTB_PEDAL: &str = "freewheel/bike/mtb/pedal";
pub const EBIKE_ROLL: &str = "freewheel/bike/ebike/roll";
pub const EBIKE_FREEHUB: &str = "freewheel/bike/ebike/freehub";
pub const EBIKE_PEDAL: &str = "freewheel/bike/ebike/pedal";
pub const MOPED_ROLL: &str = "freewheel/bike/moped/roll";
pub const MOPED_IDLE: &str = "freewheel/bike/moped/idle";
pub const MOPED_THROTTLE: &str = "freewheel/bike/moped/throttle";
pub const COBBLES: &str = "freewheel/road/cobbles";
pub const ONCOMING: &str = "freewheel/traffic/oncoming";
pub const ONCOMING_HORN: &str = "freewheel/traffic/oncoming_horn";
pub const PARCEL_CALL: &str = "freewheel/parcel/doorbell";
pub const PARCEL_GRAB: &str = "freewheel/parcel/grab";
pub const RAIN: &str = "freewheel/ambience/rain";
pub const OLD_TOWN: &str = "freewheel/ambience/old_town";
pub const WIND: &str = "freewheel/bike/wind";
pub const BRAKE: &str = "freewheel/bike/brake";
pub const STEER: &str = "freewheel/bike/steer";
pub const SPRINT: &str = "freewheel/bike/sprint";
pub const SPENT: &str = "freewheel/bike/spent";
pub const HOP: &str = "freewheel/bike/hop";
pub const LAND_CLEAN: &str = "freewheel/bike/land_clean";
pub const LAND_HARD: &str = "freewheel/bike/land_hard";
pub const BUMP: &str = "freewheel/bike/bump";
pub const FLAT: &str = "freewheel/bike/flat";
pub const TUBE: &str = "freewheel/bike/tube";
pub const CURB_STRIKE: &str = "freewheel/bike/curb_strike";
pub const CRASH: &str = "freewheel/bike/crash";
pub const UP: &str = "freewheel/bike/up";
pub const SKID: &str = "freewheel/bike/skid";
pub const EDGE: &str = "freewheel/road/edge_rumble";
pub const LINE: &str = "freewheel/road/line_thrum";
pub const DOOR_ZONE: &str = "freewheel/road/door_zone";
pub const MARKER: &str = "freewheel/hazard/marker";
pub const CURB: &str = "freewheel/hazard/curb";
pub const RAMP: &str = "freewheel/hazard/ramp";
pub const CAR: &str = "freewheel/traffic/car";
pub const DOOR_CLICK: &str = "freewheel/traffic/door_click";
pub const DOOR_OPEN: &str = "freewheel/traffic/door_open";
pub const HORN: &str = "freewheel/traffic/horn";
pub const TOOT: &str = "freewheel/traffic/toot";
pub const TURN_LEFT: &str = "freewheel/nav/turn_left";
pub const TURN_RIGHT: &str = "freewheel/nav/turn_right";
pub const TURNED: &str = "freewheel/nav/turned";
pub const BEACON: &str = "freewheel/nav/beacon";
pub const FINISH: &str = "freewheel/nav/finish";
pub const ACT_NOW: &str = "freewheel/ui/act_now";
pub const NOPE: &str = "freewheel/ui/nope";
pub const COUNT: &str = "freewheel/ui/count";
/// The clock's dry tick in its last seconds, against the clock.
pub const CLOCK_TICK: &str = "freewheel/ui/clock_tick";
pub const GO: &str = "freewheel/ui/go";
pub const LESSON: &str = "freewheel/ui/lesson";
pub const CITY: &str = "freewheel/ambience/city";
pub const LEAVES: &str = "freewheel/leaf/rustle";
pub const CRUNCH: &str = "freewheel/leaf/crunch";
pub const SUNDAY_BED: &str = "freewheel/ambience/sunday";
pub const TITLE_MUSIC: &str = "freewheel/music/title";
pub const RESULT_MUSIC: &str = "freewheel/music/finish";

/// A bike's rolling tyres, its coasting freehub and its pedalling chain.
pub fn roll(bike: Bike) -> &'static str {
    match bike {
        Bike::Road => ROAD_ROLL,
        Bike::Bmx => BMX_ROLL,
        Bike::Mountain => MTB_ROLL,
        Bike::Electric => EBIKE_ROLL,
        Bike::Moped => MOPED_ROLL,
    }
}

/// The sound while coasting: a freehub, or a moped's idling engine.
pub fn freehub(bike: Bike) -> &'static str {
    match bike {
        Bike::Road => ROAD_FREEHUB,
        Bike::Bmx => BMX_FREEHUB,
        Bike::Mountain => MTB_FREEHUB,
        Bike::Electric => EBIKE_FREEHUB,
        Bike::Moped => MOPED_IDLE,
    }
}

/// The sound while pedalling: a chain, the e-bike's motor, or a moped's throttle.
pub fn pedal(bike: Bike) -> &'static str {
    match bike {
        Bike::Road => ROAD_PEDAL,
        Bike::Bmx => BMX_PEDAL,
        Bike::Mountain => MTB_PEDAL,
        Bike::Electric => EBIKE_PEDAL,
        Bike::Moped => MOPED_THROTTLE,
    }
}

/// The tick that marks a hazard ahead.
pub fn marker(kind: HazardKind) -> &'static str {
    match kind {
        HazardKind::Door | HazardKind::Pothole => MARKER,
        HazardKind::Curb => CURB,
        HazardKind::Ramp => RAMP,
    }
}

/// Hazards ahead sound no further off than this, so their side stays
/// easy to hear; their loudness follows the real distance.
pub const AHEAD_CAP: f64 = 8.0;

/// Where to place a sound `right` metres to the rider's right and `ahead`
/// metres ahead (negative behind).
pub fn place(right: f64, ahead: f64) -> Vec3 {
    let ahead = ahead.clamp(-AHEAD_CAP, AHEAD_CAP);
    Vec3::new(right as f32, ahead as f32, 0.0)
}

/// Loudness for a sound this many metres away.
pub fn distance_volume(metres: f64) -> f32 {
    (1.0 - metres.abs() / 80.0).clamp(0.2, 1.0) as f32
}

/// How often a hazard's marker ticks: twice a second at 3 seconds or
/// more, rising to ten a second at half a second.
pub fn tick_rate(secs: f64) -> f64 {
    let t = ((3.0 - secs) / 2.5).clamp(0.0, 1.0);
    2.0 + 8.0 * t
}

/// A marker's pitch: higher as it comes nearer.
pub fn tick_pitch(metres: f64) -> f32 {
    (1.0 + 0.5 * (1.0 - (metres / 30.0).clamp(0.0, 1.0))) as f32
}

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

pub fn play_at(ctx: &mut Ctx<Parlor>, key: &str, pos: Vec3) -> Sound {
    ctx.audio.play_at(key, pos, Bus::EFFECTS)
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// The city bed on, music off, for riding.
pub fn city_bed(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(CITY));
}

/// A district's bed while riding.
pub fn district_bed(ctx: &mut Ctx<Parlor>, district: crate::course::District) {
    use crate::course::District;
    ctx.audio.play_music(None, false);
    let bed = match district {
        District::Downtown => CITY,
        District::Riverside => RAIN,
        District::OldTown => OLD_TOWN,
    };
    ctx.audio.set_bed(Some(bed));
}

/// Car-free Sunday's quiet streets: birdsong and a far bell instead of
/// the city.
pub fn sunday_bed(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(SUNDAY_BED));
}

/// Every cue, with its name and meaning, for the sound check. The name is
/// the part before the colon.
pub fn catalog() -> Vec<(&'static str, &'static str)> {
    let mut all = vec![
        (ROAD_ROLL, "Road bike tyres: a thin hiss that rises with your speed."),
        (ROAD_FREEHUB, "Road bike freewheel: a fast, light ticking while you coast. It slows as you slow."),
        (ROAD_PEDAL, "Road bike chain: a fine, soft purr while you pedal."),
        (BMX_ROLL, "BMX tyres: a chunky buzz that rises with your speed."),
        (BMX_FREEHUB, "BMX freewheel: a loud, slower clatter while you coast."),
        (BMX_PEDAL, "BMX chain: a chunky rattle while you pedal."),
        (MTB_ROLL, "Mountain bike tyres: a knobbly hum that rises with your speed."),
        (MTB_FREEHUB, "Mountain bike freewheel: a deep, fast ticking while you coast. It slows as you slow."),
        (MTB_PEDAL, "Mountain bike chain: a gritty purr while you pedal."),
        (EBIKE_ROLL, "E-bike tyres: a soft hiss that rises with your speed."),
        (EBIKE_FREEHUB, "E-bike freewheel: a quiet tick while you coast."),
        (EBIKE_PEDAL, "E-bike motor: a high electric whine while you pedal. It fades above 25 kilometres an hour."),
        (MOPED_ROLL, "Moped tyres: a low road roar that rises with your speed."),
        (MOPED_IDLE, "Moped idle: a putter while you let go of the throttle."),
        (MOPED_THROTTLE, "Moped throttle: a two-stroke buzz that climbs as you speed up."),
        (WIND, "Wind: a rush in your ears that grows as you go faster."),
        (BRAKE, "Brakes: a short squeal as the brakes bite."),
        (STEER, "Steer: a soft swish as you lean left or right."),
        (SPRINT, "Sprint: a hard breath as you stand on the pedals."),
        (SPENT, "Out of breath: a long gasp. Sprint is spent until you rest."),
        (HOP, "Hop: a spring and a rattle as both wheels leave the road."),
        (LAND_CLEAN, "Clean landing: a light double thump."),
        (LAND_HARD, "Hard landing: a heavy crunch that costs you speed."),
        (BUMP, "Pothole jolt: a deep thud as a wide tyre drops through a pothole."),
        (FLAT, "Flat tyre: a pop and a hiss. Stop and stand still for 3 seconds to swap the tube."),
        (TUBE, "Tube swapped: a pump and a click. The tyre is fixed."),
        (CURB_STRIKE, "Curb strike: a hard metal bang as a wheel hits a curb."),
        (CRASH, "Crash: a clatter and a scrape. You are down for 3 seconds."),
        (UP, "Back up: a click into the pedals. Ride on."),
        (SKID, "Running wide: a long tyre skid at a turn taken too fast."),
        (EDGE, "Edge rumble: a low rumble from the edge you are pressed against, the parked cars or the centre line."),
        (LINE, "Lane line: a short thrum as you cross the line between the bike lane and the traffic lane."),
        (DOOR_ZONE, "Door zone: a faint murmur of parked cars beside you. A door could open here."),
        (MARKER, "Hazard tick: a pothole or an open door ahead, from its side. It ticks faster and higher as you get closer."),
        (CURB, "Curb tone: a low, hard tick from a raised curb across the bike lane. Ride around it, or hop it on a BMX."),
        (RAMP, "Ramp tone: a bright rising tick from a ramp ahead. Press hop at its lip for a jump."),
        (CAR, "Car: an engine hum. Muffled behind you, clearer as it draws level."),
        (ONCOMING, "Oncoming car: an engine coming toward you in the traffic lane. Get into the bike lane."),
        (ONCOMING_HORN, "Oncoming horn: a long blast from ahead. An oncoming car is coming."),
        (PARCEL_CALL, "Parcel: a shop doorbell ahead, from the side the parcel is on. Ride over it slowly to grab it."),
        (PARCEL_GRAB, "Parcel grabbed: a cardboard thump."),
        (COBBLES, "Cobbles: a rattling rumble under your wheels in Old Town."),
        (DOOR_CLICK, "Door handle: a sharp click from a parked car. Its door opens in a moment."),
        (DOOR_OPEN, "Door open: a creak as the door swings into the door zone."),
        (HORN, "Horn: a car stuck behind you. Move right to let it pass."),
        (TOOT, "Thank-you toot: two short beeps as a car you let by passes. That earns a tip."),
        (TURN_LEFT, "Left turn chime: from your left, a junction turn is coming."),
        (TURN_RIGHT, "Right turn chime: from your right, a junction turn is coming."),
        (TURNED, "Turn made: a bright chime as you take a junction turn cleanly."),
        (BEACON, "Finish bell: a bell from the finish line ahead."),
        (FINISH, "Finish: a cheer and a bell as you cross the line."),
        (ACT_NOW, "Act now: a sharp double beep. The hazard is in your line: hop, brake or steer now."),
        (NOPE, "Dull tap: nothing happens, such as a hop while stopped."),
        (COUNT, "Countdown beep: the ride starts after three."),
        (GO, "Go: a high beep. The ride has started."),
        (LESSON, "Lesson passed: a rising chime in training."),
        (LEAVES, "Leaf pile: a dry rustle from a pile of leaves ahead in the bike lane, in the Leaf season ride. It is not a hazard."),
        (CRUNCH, "Leaf crunch: a crackle as you ride through a leaf pile, for 1000 ride points. The freewheel goes quiet for a moment."),
    ];
    if crate::clock::offered() {
        let at = all
            .iter()
            .position(|(k, _)| *k == COUNT)
            .unwrap_or(all.len());
        all.insert(
            at,
            (CLOCK_TICK, "Clock tick: against the clock, under 15 seconds left. It ticks twice as fast in the last 5."),
        );
    }
    all
}

/// The name of a catalog entry: the part before its colon.
pub fn label(meaning: &str) -> &str {
    meaning.split(':').next().unwrap_or(meaning)
}
