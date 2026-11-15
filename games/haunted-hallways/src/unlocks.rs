//! Everything Nevermore Manor lets you earn: modes, curses, the
//! housekeeper's diary, a bestiary of what haunts the house, its music,
//! and feats. Listed on the title's Unlocks row through
//! `parlor_core::unlocks`.

use hearken::Ctx;
use parlor_core::unlocks::{earned_line, Unlock, UnlockGame, Unlocks};
use parlor_core::{Challenges, Pages, Parlor, Period};

use crate::progress::{Progress, NIGHTMARE_AT};
use crate::rules::{FloorStats, GhostKind, Mode, Run, Twist, FLOORS};
use crate::{sound, ID, NAME};

pub const MODES: &str = "Modes";
pub const CURSES: &str = "Curses";
pub const DIARY: &str = "Diary pages";
pub const BESTIARY: &str = "Bestiary";
pub const MUSIC: &str = "Music";
pub const FEATS: &str = "Feats";

/// Floors between diary pages.
pub const DIARY_EVERY: u32 = 5;

const DIARY_IDS: [&str; 20] = [
    "diary-1", "diary-2", "diary-3", "diary-4", "diary-5", "diary-6", "diary-7", "diary-8",
    "diary-9", "diary-10", "diary-11", "diary-12", "diary-13", "diary-14", "diary-15", "diary-16",
    "diary-17", "diary-18", "diary-19", "diary-20",
];

/// The housekeeper's diary, a page for every five floors cleared.
pub const DIARY_PAGES: [&str; 20] = [
    "I have kept this house for Mister Silas Nevermore for eleven years. Tonight the front doors locked themselves, and the stairs went on further than I have ever counted. I will write down what I find, in case someone comes after me.",
    "The master was a clockmaker before he was rich. Every clock in the house is his, and each one ticks a little slower than the last. The cook says the house is winding down. I told her to hush.",
    "The mistress, Eleanor, died in the gallery in the winter of the fever. I hear her weeping in the long gallery at night. If I keep very still, she passes me by.",
    "Little Ada was the fever's second. She was seven. The master shut himself in his workshop for a month, and when he came out he had begun to build.",
    "Thomas the butler would not leave his post, even after the others fled. The master had the keys chained to his belt so they would never be lost. I hear the chains on the stairs still.",
    "There are rooms now that were never on the plans. Each time I take the stairs, there is another floor beyond. The master said a house can hold time, if it is big enough.",
    "Ada's dog went blind the year she died. It still searches the halls for her. It cannot see you, but it knows when you stand still, the way she stood still to hide.",
    "Some of the floorboards have gone soft. You hear them groan before they give. I lost a lantern through one and never heard it land.",
    "The master hated noise when Ada was sick. Every footstep woke her, he said. Now something dark walks the rooms and follows the sound of feet. I think it is what is left of him.",
    "I found the clock tower today, though I had been going down, not up. The house does not care which way is down. The pendulum swings so slowly I counted to forty between strokes.",
    "The things in the rooms have begun to copy one another. Two clocks ticking, and only one of them real. The false one never stays where you left it.",
    "The water came up through the cellar floor last spring. Nobody drained it. Now it is cold and black and full of echoes, and the house goes on beneath it.",
    "A medium came once, before the doors locked. She called up sounds one after another, and said the dead want their things back in the order they lost them.",
    "The catacombs are older than the house. The master did not build them. He only dug until he found them, and I think they were waiting.",
    "Some rooms down here are mirrored. Left is right and right is left, and the sounds all lie about where they come from. I trust my count of steps, not my ears.",
    "Eleanor's tomb is empty. Ada's is not. The master's has his name on it but no date, as if he has not finished dying.",
    "The heart under the floor beats louder the deeper I go. It is the house's heart, and the master built it out of a clock.",
    "I understand now. The master did not want to keep time. He wanted to stop it, on the night before Ada fell ill, and the house is that night, going on and on.",
    "I found the master in the last room, winding the heart. He did not see me. He only said, one more turn, and she will wake.",
    "I took the key from his hand, and the heart stopped. Every door in the house opened at once. If you are reading this, the way out is behind you. Walk quietly.",
];

/// The bestiary: an id, the name, its sound, and what to do about it.
const BEASTS: [(&str, &str, &str, &str, &str); 16] = [
    (
        "beast-widow",
        "The weeping widow",
        sound::GHOST_WIDOW,
        "Evade the weeping widow.",
        "Eleanor Nevermore, the master's wife. A long falling wail. She drifts slowly past. Keep still from the stab until her breath out.",
    ),
    (
        "beast-butler",
        "The chained butler",
        sound::GHOST_BUTLER,
        "Evade the chained butler.",
        "Thomas, who never left his post. Dragging chains and a groan. He moves fast. Keep still from the stab until his breath out.",
    ),
    (
        "beast-child",
        "The whispering child",
        sound::GHOST_CHILD,
        "Evade the whispering child.",
        "Ada, the master's daughter. Soft whispers and a giggle, quiet until she is close. Keep still from the stab until her breath out.",
    ),
    (
        "beast-hound",
        "The blind hound",
        sound::GHOST_HOUND,
        "Evade the blind hound, from floor 25.",
        "Ada's dog, still searching. Panting and a low growl. It cannot see you, but it finds whoever stands still. When it howls beside you, keep walking until its breath out.",
    ),
    (
        "beast-shadow",
        "The shadow",
        sound::SHADOW,
        "Clear a floor where the shadow walks, from floor 20.",
        "What is left of the master. A low breathing drone that walks the room one cell at a time and follows your footsteps. Stand still and it wanders. Let it reach you and it catches you.",
    ),
    (
        "beast-mimic",
        "Mimics",
        sound::MIMIC,
        "Clear a floor with mimics, from floor 35.",
        "Copies of the answer that sound just like it but hop to a new place every few seconds. Stand still: the real thing never moves. Touching a mimic costs time.",
    ),
    (
        "beast-rotten",
        "Rotten boards",
        sound::GROAN,
        "Clear a floor with rotten boards, from floor 8.",
        "A groan from a board next to you. Step on it and you fall through, losing one courage. The hole it leaves stops you after that.",
    ),
    (
        "beast-spirit",
        "The sealing spirit",
        sound::SPIRIT,
        "Clear a room sealed by a spirit, from floor 2.",
        "A whisper from a sealed exit door. It plays the sound of something you carry. Choose that thing and offer it at the door.",
    ),
    (
        "beast-medium",
        "The medium",
        sound::SEANCE,
        "Clear a séance room, from floor 12.",
        "A low gong, then two to four sounds in turn. Pick up their things in the same order. Out of order costs time.",
    ),
    (
        "beast-dark",
        "Dark rooms",
        sound::NOTHING,
        "Clear a floor with a dark room, from floor 15.",
        "In a dark room, things call out only when you are 3 steps away or nearer. Sweep the room row by row.",
    ),
    (
        "beast-mirror",
        "Mirrored rooms",
        sound::DRAFT,
        "Clear a floor with a mirrored room, from floor 45.",
        "In a mirrored room every sound comes from the other side: east sounds from your left, west from your right. North and south are true.",
    ),
    (
        "beast-pendulum",
        "The pendulum",
        sound::SWING,
        "Clear a floor where a pendulum swings, from floor 51.",
        "A blade from the clock tower's works, swinging along one row of a room, back and forth. Its swish tells you where it is. Cross its row just after it passes.",
    ),
    (
        "beast-water",
        "Deep water",
        sound::LAP,
        "Clear a floor with deep water, from floor 61.",
        "Floodwater in the cellars, lapping next to you. Wading through it is slow, and every splash brings the next ghost sooner.",
    ),
    (
        "beast-stones",
        "Falling stones",
        sound::DUST,
        "Clear a floor where stones fall, from floor 71.",
        "The catacombs' ceilings crumble. Dust trickles over you first, and 2 seconds later a stone falls on that spot. Step off as the dust trickles.",
    ),
    (
        "beast-founder",
        "The founder",
        sound::FOUNDER_STEP,
        "Meet the founder, from floor 81.",
        "The first Nevermore, buried in the crypt long before the house was built over him, and risen from the founder's grave. Heavy boots that follow you from room to room. He winds up a blow with a rising scrape: strike at the ringing note, or step away.",
    ),
    (
        "beast-shift",
        "The shifting house",
        sound::SHIFT,
        "Clear a floor where the house shifts, from floor 91.",
        "At the heart of the house the walls move. A deep rumble means the exit door has moved to another wall. Status says where it went.",
    ),
];

/// Music: an id, the name, its key, and the rule.
const TUNES: [(&str, &str, &str, &str); 10] = [
    (
        "music-title",
        "The title theme",
        sound::TITLE_MUSIC,
        "Start a haunt.",
    ),
    (
        "music-floor",
        "The floor-cleared tune",
        sound::FLOOR_CLEAR,
        "Clear a floor.",
    ),
    (
        "music-lose",
        "The house keeps you",
        sound::LOSE,
        "Lose a haunt.",
    ),
    ("music-win", "The way out", sound::WIN, "Clear floor 100."),
    (
        "bed-house",
        "The old house at night",
        sound::BED_HOUSE,
        "Clear floor 10.",
    ),
    (
        "bed-attic",
        "The attic",
        sound::BED_ATTIC,
        "Clear floor 40 to reach the attic.",
    ),
    (
        "bed-tower",
        "The clock tower",
        sound::BED_TOWER,
        "Clear floor 50 to reach the clock tower.",
    ),
    (
        "bed-cellar",
        "The flooded cellars",
        sound::BED_CELLAR,
        "Clear floor 60 to reach the flooded cellars.",
    ),
    (
        "bed-crypt",
        "The crypt",
        sound::BED_CRYPT,
        "Clear floor 80 to reach the crypt.",
    ),
    (
        "bed-ball",
        "The ghostly waltz",
        sound::BED_BALL,
        "Finish a midnight masquerade, the monthly challenge.",
    ),
];

/// The floor each tune's rule asks you to clear, when it is a floor.
fn tune_floor(id: &str) -> Option<u32> {
    match id {
        "music-win" => Some(FLOORS),
        "bed-house" => Some(10),
        "bed-attic" => Some(40),
        "bed-tower" => Some(50),
        "bed-cellar" => Some(60),
        "bed-crypt" => Some(80),
        _ => None,
    }
}

/// Feats: an id, the name, the rule.
const FEATS_LIST: [(&str, &str, &str); 14] = [
    (
        "feat-untouched",
        "Untouched",
        "Clear a floor of 5 or more rooms without being caught or falling.",
    ),
    (
        "feat-iron",
        "Iron hand",
        "Bring the founder down on a floor where his blow never lands.",
    ),
    (
        "feat-unaided",
        "Unaided",
        "Clear floor 20 or deeper without a hint.",
    ),
    (
        "feat-swift",
        "Swift",
        "Clear floor 10 or deeper with more than half its haunt clock left.",
    ),
    (
        "feat-light-feet",
        "Light feet",
        "Clear a floor with rotten boards without falling.",
    ),
    (
        "feat-unseen",
        "Unseen",
        "Clear a floor where the shadow walks without it catching you.",
    ),
    (
        "feat-ghosts-100",
        "Ghost watcher",
        "Evade 100 ghosts in all.",
    ),
    (
        "feat-ghosts-500",
        "Ghost keeper",
        "Evade 500 ghosts in all.",
    ),
    (
        "feat-cursed",
        "Thrice cursed",
        "Clear floor 30 or deeper with 3 curses on.",
    ),
    (
        "feat-nightmare-50",
        "Bad dream",
        "Clear floor 50 in Nightmare.",
    ),
    (
        "feat-nightmare-100",
        "Waking up",
        "Clear floor 100 in Nightmare.",
    ),
    (
        "feat-endless-150",
        "Deep night",
        "Clear floor 150 in Endless night.",
    ),
    (
        "feat-masked",
        "Masked guest",
        "Finish a midnight masquerade, the monthly challenge.",
    ),
    (
        "feat-streak-7",
        "Every night",
        "Finish the daily challenge 7 days in a row.",
    ),
];

fn diary_name(i: usize) -> String {
    format!("Diary page {}", i + 1)
}

fn curse_id(t: Twist) -> &'static str {
    match t {
        Twist::Restless => "curse-restless",
        Twist::LightsOut => "curse-lights-out",
        Twist::WitchingHour => "curse-witching-hour",
        Twist::CursedHands => "curse-cursed-hands",
        Twist::SilentSpirits => "curse-silent-spirits",
        Twist::Darkness => "curse-pitch-dark",
        Twist::Mirrors => "curse-mirrors",
    }
}

fn curse_of(id: &str) -> Option<Twist> {
    Twist::ALL.into_iter().find(|&t| curse_id(t) == id)
}

/// Every unlockable, in the order the Unlocks menu lists them.
pub fn list() -> Vec<Unlock> {
    let mut v = vec![
        Unlock::new(
            "mode-nightmare",
            MODES,
            "Nightmare",
            format!("Clear floor {NIGHTMARE_AT}."),
            "The whole house from floor 1, with less time, faster ghosts and no warning call. Courage lasts the run, with one back every ten floors. Find it on the title.",
        ),
        Unlock::new(
            "mode-endless",
            MODES,
            "Endless night",
            format!("Clear floor {FLOORS}."),
            "The floors below floor 100, without end, each harder than the last. Courage lasts the run, with one back every five floors. Find it on the title.",
        ),
    ];
    for t in Twist::ALL {
        v.push(Unlock::new(
            curse_id(t),
            CURSES,
            t.name(),
            format!(
                "Clear a floor that carries the {} twist, from floor {}.",
                t.name(),
                t.from_floor()
            ),
            format!(
                "A curse you can switch on: {}. Each curse on adds a quarter to your score in Play, Floor select, Nightmare and Endless night. Enter switches it.",
                t.describe()
            ),
        ));
    }
    for (i, id) in DIARY_IDS.into_iter().enumerate() {
        v.push(Unlock::new(
            id,
            DIARY,
            diary_name(i),
            format!("Clear floor {}.", (i as u32 + 1) * DIARY_EVERY),
            "A page of the housekeeper's diary. Enter reads it.",
        ));
    }
    for (id, name, _, rule, about) in BEASTS {
        v.push(Unlock::new(
            id,
            BESTIARY,
            name,
            rule,
            format!("{about} Enter plays its sound."),
        ));
    }
    for (id, name, _, rule) in TUNES {
        v.push(Unlock::new(id, MUSIC, name, rule, "Enter plays it."));
    }
    for (id, name, rule) in FEATS_LIST {
        v.push(Unlock::new(
            id,
            FEATS,
            name,
            rule,
            format!("Earned: {}", lower_first(rule)),
        ));
    }
    v
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// The game's side of the shared Unlocks menu.
pub struct Keepsakes;

/// The Unlocks menu's `State::name`.
pub const UNLOCKS_NAME: &str = "haunted_hallways_unlocks";

impl UnlockGame for Keepsakes {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
    const MENU_NAME: &'static str = UNLOCKS_NAME;

    fn unlocks(_ctx: &mut Ctx<Parlor>) -> Vec<Unlock> {
        list()
    }

    fn use_unlock(ctx: &mut Ctx<Parlor>, unlock: &Unlock) {
        if let Some(t) = curse_of(unlock.id) {
            let mut progress = Progress::load(ctx);
            progress.toggle_curse(t);
            progress.store(ctx);
            return;
        }
        if let Some(i) = DIARY_IDS.iter().position(|&d| d == unlock.id) {
            ctx.push(Pages::new([format!(
                "The housekeeper's diary, page {}. {}",
                i + 1,
                DIARY_PAGES[i]
            )]));
            return;
        }
        if let Some((_, _, key, _, about)) = BEASTS.iter().find(|b| b.0 == unlock.id) {
            sound::play(ctx, key);
            ctx.say(*about);
            return;
        }
        if let Some((_, name, key, _)) = TUNES.iter().find(|t| t.0 == unlock.id) {
            ctx.audio.play_music(Some(key), false);
            ctx.say(format!("{name}."));
            return;
        }
        ctx.say(unlock.about.clone());
    }

    fn state(ctx: &mut Ctx<Parlor>, unlock: &Unlock) -> Option<String> {
        let t = curse_of(unlock.id)?;
        let on = Progress::load(ctx).curses().has(t);
        Some(if on { "on" } else { "off" }.to_string())
    }

    fn music(ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }
}

/// Earns `ids` and returns the names of the new ones.
fn earn(ctx: &mut Ctx<Parlor>, ids: &[&str]) -> Vec<String> {
    Unlocks::earn_all(ctx, ID, &list(), ids)
}

/// Unlocks that follow from saved progress alone: modes, diary pages and
/// music by the floors cleared, and feats by ghosts evaded. Run on the
/// title, so older saves catch up, and after every floor.
pub fn sync(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let p = Progress::load(ctx);
    let mut ids: Vec<&str> = Vec::new();
    if p.nightmare_open() {
        ids.push("mode-nightmare");
    }
    if p.endless_open() {
        ids.push("mode-endless");
    }
    for (i, id) in DIARY_IDS.iter().enumerate() {
        if p.cleared >= (i as u32 + 1) * DIARY_EVERY {
            ids.push(id);
        }
    }
    for (id, ..) in TUNES {
        if tune_floor(id).is_some_and(|f| p.cleared >= f) {
            ids.push(id);
        }
    }
    if p.ghosts_evaded >= 100 {
        ids.push("feat-ghosts-100");
    }
    if p.ghosts_evaded >= 500 {
        ids.push("feat-ghosts-500");
    }
    if p.nightmare_deepest >= 50 {
        ids.push("feat-nightmare-50");
    }
    if p.nightmare_deepest >= FLOORS {
        ids.push("feat-nightmare-100");
    }
    if p.endless_deepest >= 150 {
        ids.push("feat-endless-150");
    }
    earn(ctx, &ids)
}

/// A haunt began.
pub fn on_start(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    earn(ctx, &["music-title"])
}

/// What clearing a floor earned, from what happened on it.
pub fn on_floor(ctx: &mut Ctx<Parlor>, run: &Run, number: u32) -> Vec<String> {
    let s: &FloorStats = &run.stats;
    let spec = &run.spec;
    let mut ids: Vec<&str> = vec!["music-floor"];
    for t in &spec.twists {
        ids.push(curse_id(*t));
    }
    let ghosts = [
        (GhostKind::Widow, "beast-widow"),
        (GhostKind::Butler, "beast-butler"),
        (GhostKind::Child, "beast-child"),
        (GhostKind::Hound, "beast-hound"),
    ];
    for (g, id) in ghosts {
        if s.ghosts.contains(&g) {
            ids.push(id);
        }
    }
    let seen = [
        (s.shadow, "beast-shadow"),
        (s.mimic, "beast-mimic"),
        (s.rotten, "beast-rotten"),
        (s.spirit, "beast-spirit"),
        (s.seance && !spec.seance, "beast-medium"),
        (s.dark, "beast-dark"),
        (s.mirror, "beast-mirror"),
        (s.pendulum, "beast-pendulum"),
        (s.water, "beast-water"),
        (s.stones, "beast-stones"),
        (s.founder, "beast-founder"),
        (s.shifted, "beast-shift"),
    ];
    for (yes, id) in seen {
        if yes {
            ids.push(id);
        }
    }
    let challenge = matches!(run.mode, Mode::Challenge(_));
    if !challenge {
        if run.rooms.len() >= 5 && s.caught == 0 {
            ids.push("feat-untouched");
        }
        if number >= 20 && s.hints == 0 {
            ids.push("feat-unaided");
        }
        if number >= 10 && run.haunt_left > spec.haunt / 2.0 {
            ids.push("feat-swift");
        }
        if s.rotten && s.falls == 0 {
            ids.push("feat-light-feet");
        }
        if s.shadow && s.shadow_catches == 0 {
            ids.push("feat-unseen");
        }
        if s.banished && s.founder_hits == 0 {
            ids.push("feat-iron");
        }
        if number >= 30 && run.curses.count() >= 3 {
            ids.push("feat-cursed");
        }
    }
    let mut names = earn(ctx, &ids);
    names.extend(sync(ctx));
    names
}

/// What ending a run earned.
pub fn on_end(ctx: &mut Ctx<Parlor>, run: &Run, won: bool) -> Vec<String> {
    let mut ids: Vec<&str> = Vec::new();
    if !won {
        ids.push("music-lose");
    }
    if let Mode::Challenge(e) = run.mode {
        if won && e.period == Period::Monthly {
            ids.extend(["bed-ball", "feat-masked"]);
        }
        if Challenges::streak(ctx, ID, Period::Daily) >= 7 {
            ids.push("feat-streak-7");
        }
    }
    let mut names = earn(ctx, &ids);
    names.extend(sync(ctx));
    names
}

/// "Unlocked: Diary page 3. Find it in Unlocks on the title." or None.
pub fn announce(names: &[String]) -> Option<String> {
    let where_ = if names.len() == 1 { "it" } else { "them" };
    earned_line(names).map(|l| format!("{l} Find {where_} in Unlocks on the title."))
}
