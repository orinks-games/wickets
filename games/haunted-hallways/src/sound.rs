//! Sound keys, and where sounds sit around the listener.

use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::Parlor;

use crate::rules::{wing_index, GhostKind, Item, BALL_FLOOR, CELL};

/// The mansion's dynamic score (see `parlor_core::score`).
pub const SCORE: [parlor_core::Stem; 6] = parlor_core::score_stems!("haunted-hallways");
pub const STEP_WOOD_A: &str = "haunted-hallways/steps/wood_a";
pub const STEP_WOOD_B: &str = "haunted-hallways/steps/wood_b";
pub const STEP_STONE: &str = "haunted-hallways/steps/stone";
pub const BUMP: &str = "haunted-hallways/move/bump";
pub const DOOR_OPEN: &str = "haunted-hallways/door/open";
pub const DOOR_LOCKED: &str = "haunted-hallways/door/locked";
pub const DOOR_SHUT: &str = "haunted-hallways/door/shut";
pub const DOOR_UNLOCK: &str = "haunted-hallways/door/unlock";
pub const DRAFT: &str = "haunted-hallways/door/draft";
pub const SPIRIT: &str = "haunted-hallways/door/spirit";
pub const STAIRS: &str = "haunted-hallways/door/stairs";
pub const PICKUP: &str = "haunted-hallways/cue/pickup";
pub const WRONG: &str = "haunted-hallways/cue/wrong";
pub const SOLVED: &str = "haunted-hallways/cue/solved";
pub const NOTHING: &str = "haunted-hallways/cue/nothing";
pub const HINT: &str = "haunted-hallways/cue/hint";
pub const SELECT: &str = "haunted-hallways/cue/select";
pub const OFFER_OK: &str = "haunted-hallways/cue/offer_ok";
pub const OFFER_BAD: &str = "haunted-hallways/cue/offer_bad";
pub const PHONOGRAPH: &str = "haunted-hallways/cue/phonograph";
pub const TOLL: &str = "haunted-hallways/cue/toll";
pub const SEANCE: &str = "haunted-hallways/cue/seance";
pub const SEANCE_STEP: &str = "haunted-hallways/cue/seance_step";
pub const FREEZE: &str = "haunted-hallways/ghost/freeze";
pub const PASSED: &str = "haunted-hallways/ghost/passed";
pub const CAUGHT: &str = "haunted-hallways/ghost/caught";
pub const THUNDER: &str = "haunted-hallways/atmos/thunder";
pub const SCREAM: &str = "haunted-hallways/atmos/scream";
pub const TITLE_MUSIC: &str = "haunted-hallways/music/title";
pub const WIN: &str = "haunted-hallways/music/win";
pub const LOSE: &str = "haunted-hallways/music/lose";
pub const FLOOR_CLEAR: &str = "haunted-hallways/music/floor";
pub const GHOST_WIDOW: &str = "haunted-hallways/ghost/widow";
pub const GHOST_BUTLER: &str = "haunted-hallways/ghost/butler";
pub const GHOST_CHILD: &str = "haunted-hallways/ghost/child";
pub const GHOST_HOUND: &str = "haunted-hallways/ghost/hound";
/// The hound beside you: keep walking.
pub const HOWL: &str = "haunted-hallways/ghost/howl";
pub const SHADOW: &str = "haunted-hallways/ghost/shadow";
pub const GROAN: &str = "haunted-hallways/hazard/groan";
pub const FALL: &str = "haunted-hallways/hazard/fall";
pub const MIMIC: &str = "haunted-hallways/cue/mimic";
pub const CHARM: &str = "haunted-hallways/charm/call";
pub const CHARM_TAKE: &str = "haunted-hallways/charm/take";
pub const SALT: &str = "haunted-hallways/charm/salt";
pub const SWING: &str = "haunted-hallways/hazard/swing";
pub const SPLASH: &str = "haunted-hallways/hazard/splash";
pub const LAP: &str = "haunted-hallways/hazard/lap";
pub const DUST: &str = "haunted-hallways/hazard/dust";
pub const STONE: &str = "haunted-hallways/hazard/stone";
pub const SHIFT: &str = "haunted-hallways/hazard/shift";
pub const FOUNDER_STEP: &str = "haunted-hallways/founder/step";
pub const WINDUP: &str = "haunted-hallways/founder/windup";
pub const STRIKE_HIT: &str = "haunted-hallways/founder/hit";
pub const OPENING: &str = "haunted-hallways/founder/opening";
pub const PARRY: &str = "haunted-hallways/founder/parry";
pub const CRUMBLE: &str = "haunted-hallways/founder/crumble";
pub const LANDING: &str = "haunted-hallways/cue/landing";
pub const UNLOCK: &str = "haunted-hallways/cue/unlock";
pub const BED_HOUSE: &str = "haunted-hallways/ambience/house";
pub const BED_ATTIC: &str = "haunted-hallways/ambience/attic";
pub const BED_TOWER: &str = "haunted-hallways/ambience/tower";
pub const BED_CELLAR: &str = "haunted-hallways/ambience/cellar";
pub const BED_CRYPT: &str = "haunted-hallways/ambience/crypt";
pub const BED_BALL: &str = "haunted-hallways/ambience/ball";

pub fn item(item: Item) -> &'static str {
    match item {
        Item::Clock => "haunted-hallways/items/clock",
        Item::Crystal => "haunted-hallways/items/crystal",
        Item::Potion => "haunted-hallways/items/potion",
        Item::MusicBox => "haunted-hallways/items/musicbox",
        Item::Bell => "haunted-hallways/items/bell",
        Item::Candle => "haunted-hallways/items/candle",
        Item::Raven => "haunted-hallways/items/raven",
        Item::Keys => "haunted-hallways/items/keys",
        Item::Heart => "haunted-hallways/items/heart",
    }
}

pub fn ghost(kind: GhostKind) -> &'static str {
    match kind {
        GhostKind::Widow => GHOST_WIDOW,
        GhostKind::Butler => GHOST_BUTLER,
        GhostKind::Child => GHOST_CHILD,
        GhostKind::Hound => GHOST_HOUND,
    }
}

/// The quiet bed under each floor: the house grows stranger as you go.
pub fn ambience(floor: u32) -> &'static str {
    if floor == BALL_FLOOR {
        return BED_BALL;
    }
    match wing_index(floor) {
        0..=3 => BED_HOUSE,
        4 => BED_ATTIC,
        5 => BED_TOWER,
        6 => BED_CELLAR,
        _ => BED_CRYPT,
    }
}

/// A cell offset (x east, y north) as metres around the listener, who
/// always faces north.
pub fn at(dx: f32, dy: f32) -> Vec3 {
    Vec3::new(dx * CELL, dy * CELL, 0.0)
}

pub fn play_at(ctx: &mut Ctx<Parlor>, key: &str, dx: f32, dy: f32) -> Sound {
    ctx.audio.play_at(key, at(dx, dy), Bus::EFFECTS)
}

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// A floor's bed on, music off.
pub fn floor_bed(ctx: &mut Ctx<Parlor>, floor: u32) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(ambience(floor)));
}

/// Every cue, with its name and meaning, for the sound check.
pub fn catalog() -> Vec<(&'static str, String)> {
    let mut v: Vec<(&'static str, String)> = vec![
        (
            STEP_WOOD_A,
            "Footstep on floorboards: you moved one step, in the upper wings.".to_string(),
        ),
        (
            STEP_STONE,
            "Footstep on stone: you moved one step, from the flooded cellars down.".to_string(),
        ),
        (
            BUMP,
            "Wall thud: you walked into a wall, or a hole, and did not move.".to_string(),
        ),
        (
            DOOR_LOCKED,
            "Rattling handle: the exit door is still locked. Solve the room first.".to_string(),
        ),
        (
            DOOR_SHUT,
            "Heavy slam: the door you came in by is shut fast. There is no going back.".to_string(),
        ),
        (
            DOOR_UNLOCK,
            "Bolt sliding back: the room is solved and its exit door unlocks.".to_string(),
        ),
        (
            DRAFT,
            "Cold draught: an open exit door. Follow it to leave the room.".to_string(),
        ),
        (
            SPIRIT,
            "Spirit's whisper: a spirit seals the exit door and wants an offering. Go to it."
                .to_string(),
        ),
        (
            DOOR_OPEN,
            "Creaking door: you pass through into the next room.".to_string(),
        ),
        (STAIRS, "Stairs: you leave the floor.".to_string()),
        (
            PHONOGRAPH,
            "Phonograph crackle: a matching puzzle, or a missing sound. The sounds that follow are the clue."
                .to_string(),
        ),
        (
            SEANCE,
            "Medium's gong: a séance. Two to four sounds follow; take their things in that order."
                .to_string(),
        ),
        (
            SEANCE_STEP,
            "Medium's hum: you took the séance's next thing in the right order.".to_string(),
        ),
    ];
    for i in Item::ALL {
        v.push((item(i), format!("The {}: {}.", i.name(), i.sound())));
    }
    v.extend([
        (PICKUP, "Grab: you picked up the right thing.".to_string()),
        (SOLVED, "Chime: the room's puzzle is solved.".to_string()),
        (
            WRONG,
            "Mocking laugh: not what the room wants. You lose time on the haunt clock.".to_string(),
        ),
        (
            MIMIC,
            "Warped shriek: a mimic, a copy of the answer. You lose time, and it hops away."
                .to_string(),
        ),
        (
            NOTHING,
            "Dust: there is nothing to pick up here.".to_string(),
        ),
        (
            SELECT,
            "Rustle: you chose another thing you carry.".to_string(),
        ),
        (
            OFFER_OK,
            "The spirit sighs: it accepts your offering.".to_string(),
        ),
        (
            OFFER_BAD,
            "The spirit shrieks: wrong offering. You lose time.".to_string(),
        ),
        (
            TOLL,
            "Bell toll: the haunt clock is running low.".to_string(),
        ),
        (
            GROAN,
            "Groaning board: a rotten board next to you, in the direction you hear it. Do not step on it."
                .to_string(),
        ),
        (
            FALL,
            "Crash of boards: you fell through a rotten board and lost courage.".to_string(),
        ),
        (
            LANDING,
            "Rising chord: a landing. You get one courage back.".to_string(),
        ),
    ]);
    for g in GhostKind::ALL {
        let what = if g.hunts_stillness() {
            "When you hear it, the hound is coming: be ready to walk."
        } else {
            "When you hear it, a ghost is coming."
        };
        v.push((
            ghost(g),
            format!("A ghost, {}: {}. {what}", g.name(), g.voice()),
        ));
    }
    v.extend([
        (
            FREEZE,
            "Freeze stab: a ghost is passing right by you. Do not move until it has passed."
                .to_string(),
        ),
        (
            HOWL,
            "Howl: the blind hound is passing right by you. Keep walking until it has passed."
                .to_string(),
        ),
        (
            PASSED,
            "Long breath out: the ghost has passed without finding you.".to_string(),
        ),
        (
            CAUGHT,
            "Scream: something caught you. You lose courage and flee to the room's door."
                .to_string(),
        ),
        (
            SHADOW,
            "The shadow: a low breathing drone that walks the room, following your footsteps. Do not let it reach you."
                .to_string(),
        ),
        (
            CHARM,
            "Soft glass chime: a charm lying in the room. Walk onto it to take it.".to_string(),
        ),
        (
            CHARM_TAKE,
            "Bright pluck: you took a charm.".to_string(),
        ),
        (
            SALT,
            "Hiss of salt: your salt warded something off. You stay where you are.".to_string(),
        ),
        (
            SWING,
            "Swish: the pendulum's blade, sweeping along its row. Cross that row just after it passes."
                .to_string(),
        ),
        (
            SPLASH,
            "Splash: you waded into deep water. Wading is slow and brings ghosts sooner."
                .to_string(),
        ),
        (
            LAP,
            "Lapping water: deep water next to you, in the direction you hear it.".to_string(),
        ),
        (
            DUST,
            "Trickling dust: a stone is about to fall where you stand. Step off at once."
                .to_string(),
        ),
        (
            STONE,
            "Crash of stone: a stone hit the floor.".to_string(),
        ),
        (
            SHIFT,
            "Deep rumble: the house shifted, and the exit door moved to another wall.".to_string(),
        ),
        (
            FOUNDER_STEP,
            "Heavy footstep: the founder, coming after you.".to_string(),
        ),
        (
            WINDUP,
            "Rising scrape: the founder winds up a blow beside you. Strike at the ringing note, or step away."
                .to_string(),
        ),
        (
            OPENING,
            "Ringing note: the founder's blow is at its height. Strike now.".to_string(),
        ),
        (
            STRIKE_HIT,
            "Iron clang: your candlestick hit the founder.".to_string(),
        ),
        (
            PARRY,
            "Dull knock: you struck too soon and the founder blocked. Step away.".to_string(),
        ),
        (
            CRUMBLE,
            "Crumbling stone: the founder falls to dust for the rest of the floor.".to_string(),
        ),
    ]);
    v
}
