//! Tech: what holding nights and downing motherships earns, listed on the
//! title's Unlocks row through `parlor_core::unlocks`. Most entries open
//! at a total of tech points across every colony; the modes open on what
//! one colony did; the logbook opens a page for each night held at most.
//!
//! An earned entry changes the next colony through [`Perks`], which the
//! run carries, so a saved colony keeps the rules it started under.

use hearken::Ctx;
use parlor_core::unlocks::{Unlock, UnlockGame, Unlocks};
use parlor_core::{Pages, Parlor, Records};

use crate::progress::{Progress, LONG_WATCH_BOARD, NIGHTS_BOARD};
use crate::rules::perks::{BONUS_PERCENT, HARD_VACUUM_SHIELD};
use crate::rules::Perks;
use crate::{sound, ID};

pub const UNLOCKS_NAME: &str = "dustfall_unlocks";

pub const COLONY: &str = "Colony";
pub const ARMOURY: &str = "Armoury";
pub const INTELLIGENCE: &str = "Intelligence";
pub const MODES: &str = "Modes";
pub const LOGBOOK: &str = "Reva's logbook";

/// Nights one colony must hold for Hard vacuum.
pub const HARD_VACUUM_AT: u8 = 8;

/// The tech-point entries: id, group, name, points, what it gives.
const TECH: [(&str, &str, &str, u32, &str); 9] = [
    (
        "dome-thick",
        COLONY,
        "Thicker dome",
        8,
        "The dome starts at 110 and repairs up to it.",
    ),
    (
        "shield-big",
        COLONY,
        "Bigger shield",
        12,
        "The shield holds 70 instead of 50.",
    ),
    (
        "slot-4",
        COLONY,
        "Fourth slot",
        16,
        "The far left slot is open to build in.",
    ),
    (
        "slot-5",
        COLONY,
        "Fifth slot",
        30,
        "The far right slot is open to build in too.",
    ),
    (
        "rack-missile",
        ARMOURY,
        "A missile in the rack",
        6,
        "Every colony starts with a missile made.",
    ),
    (
        "mines-two",
        ARMOURY,
        "Two mines a night",
        10,
        "Two mines may be laid before a night instead of one.",
    ),
    (
        "rack-deep",
        ARMOURY,
        "A deeper rack",
        20,
        "The rack holds 5 missiles instead of 3.",
    ),
    (
        "quirks-forecast",
        INTELLIGENCE,
        "Quirks in the forecast",
        7,
        "The forecast names a raider's quirk without a scanner.",
    ),
    (
        "lane-forecast",
        INTELLIGENCE,
        "Lanes in the forecast",
        15,
        "The forecast names the lanes the raiders enter at without a scanner.",
    ),
];

const LOG_IDS: [&str; 12] = [
    "log-1", "log-2", "log-3", "log-4", "log-5", "log-6", "log-7", "log-8", "log-9", "log-10",
    "log-11", "log-12",
];

/// Reva's logbook from the radio room at Low Harbour, a page a night.
pub const LOG_PAGES: [&str; 12] = [
    "Low Harbour, first night. The dust came down at dusk and something walked out of it. We have one gun on a rail and a dome that was never meant to be shot at. I am keeping this log so that someone knows what happened here.",
    "They come in kinds, and in waves. The small ones drift and fire. The quick ones come in pairs and slip a lane the moment the gun finds them. I have started chalking the five lanes on the wall of the radio room.",
    "Relief answered tonight. Twelve nights, they said, and then the ship. I did not ask what happens to the ship if there is nobody left to light the landing field.",
    "A hulk came down the left lane slow as a tide and drank the shield dry from close in. Marcus wants more mines. I want to know which lane they take, and one mine where it matters.",
    "The big one holds at mid range and slides from lane to lane, dropping its skitters like seeds. The gun has to go to it. We learned that the hard way.",
    "Sappers go for the hydroponics. Of course they do. Take the food and the gun reloads at half speed, and a slow gun is a dead colony.",
    "Some of them are different now. A drifter with plating the gun bounces off. Another that moves before the rest. The scanner picks them out of the dust, if you listen.",
    "Eight nights. The dome is patched in four places and we are eating the seed stock. Nobody says the word hungry. Everybody counts the food.",
    "The dust storm crossed the sky tonight, one lane at a time. Inside it you hear nothing, and they come on regardless. The flak post under it went quiet and we never knew what passed over it.",
    "The second mothership. We put two missiles into it before it wound up and it kept coming. Marcus laughed, which he does when he is frightened.",
    "Eleven nights. I have stopped chalking the wall. The lanes are in my head now, and I know which way a skitter will slip before it does. That is either skill or the dust getting to me.",
    "The carrier came over the horizon with two motherships behind it, and then Relief came over the top of them all. We held. Tomorrow I will walk out on the landing field and feel the sun. Then I will come back to the radio room, because somebody has to keep the watch.",
];

/// The game's side of the shared Unlocks menu.
pub struct Tech;

impl UnlockGame for Tech {
    const ID: &'static str = ID;
    const NAME: &'static str = "Dustfall";
    const MENU_NAME: &'static str = UNLOCKS_NAME;

    fn unlocks(_ctx: &mut Ctx<Parlor>) -> Vec<Unlock> {
        let mut list: Vec<Unlock> = TECH
            .iter()
            .map(|(id, group, name, points, about)| {
                Unlock::new(
                    id,
                    group,
                    *name,
                    format!("Reach {points} tech in all."),
                    *about,
                )
            })
            .collect();
        list.push(Unlock::new(
            "mode-long-watch",
            MODES,
            "The long watch",
            "Hold a colony to Relief.",
            "A colony that goes on past Relief, the nights harder each round, until the dome falls. On the title, and the monthly challenge.",
        ));
        list.push(Unlock::new(
            "mode-hard-vacuum",
            MODES,
            "Hard vacuum",
            format!("Hold {HARD_VACUUM_AT} nights in one colony."),
            format!("The shield holds {HARD_VACUUM_SHIELD} at most, for {BONUS_PERCENT} percent more score. Enter turns it on or off for your next colony."),
        ));
        for (i, id) in LOG_IDS.iter().enumerate() {
            let n = i + 1;
            list.push(Unlock::new(
                id,
                LOGBOOK,
                format!("Logbook, night {n}"),
                if n == 1 {
                    "Hold a night.".to_string()
                } else {
                    format!("Hold {n} nights in one colony.")
                },
                "A page of Reva's logbook from the radio room.",
            ));
        }
        list
    }

    fn use_unlock(ctx: &mut Ctx<Parlor>, unlock: &Unlock) {
        if unlock.id == "mode-hard-vacuum" {
            let mut p = Progress::load(ctx);
            p.hard_vacuum = !p.hard_vacuum;
            p.save(ctx);
            ctx.say(if p.hard_vacuum {
                format!("Hard vacuum on. Your next colony's shield holds {HARD_VACUUM_SHIELD} at most, for {BONUS_PERCENT} percent more score.")
            } else {
                "Hard vacuum off.".to_string()
            });
            return;
        }
        if let Some(i) = LOG_IDS.iter().position(|id| *id == unlock.id) {
            ctx.push(Pages::new([format!(
                "Reva's logbook, night {}. {}",
                i + 1,
                LOG_PAGES[i]
            )]));
            return;
        }
        ctx.say(unlock.about.clone());
    }

    fn state(ctx: &mut Ctx<Parlor>, unlock: &Unlock) -> Option<String> {
        if unlock.id != "mode-hard-vacuum" {
            return None;
        }
        let on = Progress::load(ctx).hard_vacuum;
        Some(if on { "on" } else { "off" }.to_string())
    }

    fn music(ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }
}

/// Whether the long watch is open: Relief has landed once.
pub fn long_watch_open(ctx: &mut Ctx<Parlor>) -> bool {
    Unlocks::has(ctx, ID, "mode-long-watch")
}

/// Earns everything the records say is due, and returns the new names.
pub fn sync(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let p = Progress::load(ctx);
    let most = Records::best(ctx, ID, NIGHTS_BOARD)
        .max(Records::best(ctx, ID, LONG_WATCH_BOARD))
        .unwrap_or(0);
    let mut ids: Vec<&str> = TECH
        .iter()
        .filter(|(_, _, _, points, _)| p.tech >= *points)
        .map(|(id, ..)| *id)
        .collect();
    if p.reliefs > 0 {
        ids.push("mode-long-watch");
    }
    if most >= u64::from(HARD_VACUUM_AT) {
        ids.push("mode-hard-vacuum");
    }
    for (i, id) in LOG_IDS.iter().enumerate() {
        if most > i as u64 {
            ids.push(id);
        }
    }
    let list = Tech::unlocks(ctx);
    Unlocks::earn_all(ctx, ID, &list, &ids)
}

/// What the player's unlocks and choices set a new colony up with.
/// `quiet` is the call-outs setting as the colony starts.
pub fn perks(ctx: &mut Ctx<Parlor>, quiet: bool) -> Perks {
    let earned = Unlocks::earned(ctx, ID);
    let has = |id: &str| earned.contains(id);
    let p = Progress::load(ctx);
    Perks {
        extra_slots: u8::from(has("slot-4")) + u8::from(has("slot-5")),
        big_shield: has("shield-big"),
        thick_dome: has("dome-thick"),
        start_missiles: u32::from(has("rack-missile")),
        more_mines: has("mines-two"),
        deep_rack: has("rack-deep"),
        lane_forecast: has("lane-forecast"),
        forecast_quirks: has("quirks-forecast"),
        hard_vacuum: p.hard_vacuum && has("mode-hard-vacuum"),
        quiet,
    }
}
