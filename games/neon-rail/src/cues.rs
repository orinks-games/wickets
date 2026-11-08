//! Every sound the game plays, by key, and the sound check's list of them
//! with what each one means. The keys are the asset contract: a recording
//! at `sounds/neon-rail/<path>.ogg` replaces a prototype with no code change.

// Traffic, hazards and pickups: loops heard while each one approaches.
pub const FLOW: &str = "neon-rail/traffic/flow";
pub const FAST: &str = "neon-rail/traffic/fast_passer";
pub const HEAVY: &str = "neon-rail/traffic/heavy_blocker";
pub const PRIORITY: &str = "neon-rail/traffic/priority";
pub const UNSTABLE: &str = "neon-rail/traffic/unstable";
pub const DEBRIS: &str = "neon-rail/hazard/debris";
pub const DRONE: &str = "neon-rail/hazard/drone";
pub const BLOCKADE: &str = "neon-rail/hazard/blockade";
pub const REPAIR: &str = "neon-rail/pickup/repair";
pub const SALVAGE: &str = "neon-rail/pickup/salvage";
/// Halloween's ghost car: the flow vehicle's pulse with a hollow whistle.
pub const GHOST_FLOW: &str = "neon-rail/traffic/ghost_flow";
/// A Friday night run's cargo crate: the salvage pulse with a bright chime.
pub const CARGO_CRATE: &str = "neon-rail/pickup/moon_shard";

// Alerts.
pub const PRIORITY_NOTICE: &str = "neon-rail/alert/priority_notice";
pub const STRESS_WARNING: &str = "neon-rail/alert/stress_warning";
pub const OVERLOAD: &str = "neon-rail/alert/overload_danger";
pub const RECOVERY: &str = "neon-rail/alert/recovery";
pub const BOOST_CHIME: &str = "neon-rail/alert/boost_chime";

// The skimmer and its tools.
pub const SKIMMER: &str = "neon-rail/skimmer/hum";
pub const SHIFT: &str = "neon-rail/skimmer/shift";
pub const EDGE: &str = "neon-rail/skimmer/edge";
pub const BOOST: &str = "neon-rail/skimmer/boost";
pub const BOOST_READY: &str = "neon-rail/skimmer/boost_ready";
pub const RAILGUN: &str = "neon-rail/skimmer/railgun";
pub const NO_LOCK: &str = "neon-rail/skimmer/no_lock";
pub const SLOWDOWN: &str = "neon-rail/skimmer/slowdown";
pub const OVERRIDE: &str = "neon-rail/skimmer/override";
pub const DENIED: &str = "neon-rail/skimmer/denied";
pub const SHIELD: &str = "neon-rail/skimmer/shield";

// What happened as something arrived.
pub const CLEAN: &str = "neon-rail/outcome/clean";
pub const PASS: &str = "neon-rail/outcome/pass";
pub const IMPACT: &str = "neon-rail/outcome/impact";
pub const SMASH: &str = "neon-rail/outcome/smash";
pub const RAILGUN_HIT: &str = "neon-rail/outcome/railgun_hit";
pub const REPAIR_GET: &str = "neon-rail/outcome/repair";
pub const SALVAGE_GET: &str = "neon-rail/outcome/salvage";
pub const MISSED: &str = "neon-rail/outcome/missed";
pub const MULTIPLIER: &str = "neon-rail/outcome/multiplier";
pub const SECTOR_START: &str = "neon-rail/outcome/sector_start";
pub const SECTOR_CLEAR: &str = "neon-rail/outcome/sector_clear";
pub const CONVOY: &str = "neon-rail/outcome/convoy";

// Between runs.
pub const UNLOCK: &str = "neon-rail/ui/unlock";
pub const UPGRADE: &str = "neon-rail/ui/upgrade";
pub const CHALLENGE: &str = "neon-rail/ui/challenge";

// Music.
pub const MUSIC_TITLE: &str = "neon-rail/music/title";
pub const MUSIC_INTERMISSION: &str = "neon-rail/music/intermission";
pub const MUSIC_WORKSHOP: &str = "neon-rail/music/workshop";
pub const MUSIC_VICTORY: &str = "neon-rail/music/victory";
pub const MUSIC_FAILURE: &str = "neon-rail/music/failure";
/// The run's dynamic score (see `parlor_core::score`).
pub const SCORE: [parlor_core::Stem; 6] = parlor_core::score_stems!("neon-rail");
/// The Black Sky event's lead, in place of the score's own.
pub const SCORE_EVENT_LEAD: &str = "neon-rail/score/event";
/// The score for the monthly Black Sky event: the same, with its own lead.
pub const SCORE_EVENT: [parlor_core::Stem; 6] = {
    let mut stems = SCORE;
    stems[2].key = SCORE_EVENT_LEAD;
    stems
};

/// How the sound check plays a cue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Play {
    Once,
    /// A loop, heard for a few seconds.
    Loop,
    /// A loop, heard coming toward you down the center lane.
    Approach,
    Music,
}

/// One row of the sound check.
pub struct Entry {
    pub name: &'static str,
    pub meaning: &'static str,
    pub key: &'static str,
    pub play: Play,
}

const fn e(name: &'static str, meaning: &'static str, key: &'static str, play: Play) -> Entry {
    Entry {
        name,
        meaning,
        key,
        play,
    }
}

/// Every cue, in the order the sound check lists them.
pub const SOUND_CHECK: &[Entry] = &[
    e("Lane shift", "The magnetic snap when your skimmer changes lanes.", SHIFT, Play::Once),
    e("Rail edge", "You are already in the outside lane and cannot shift further.", EDGE, Play::Once),
    e("Flow vehicle", "Baseline traffic. Be in its lane when it arrives for steady score and lower stress. Missing it raises stress.", FLOW, Play::Approach),
    e("Fast passer", "Fast traffic with a short warning. Be in its lane, or have a slowdown pulse running, or it clips the corridor.", FAST, Play::Approach),
    e("Heavy blocker", "A dense vehicle. Stay out of its lane, or boost into it to force it open. Meeting it unboosted jams the corridor.", HEAVY, Play::Approach),
    e("Priority vehicle", "Priority traffic. Be in its lane, or use priority override or a slowdown pulse. Delaying it sharply raises stress.", PRIORITY, Play::Approach),
    e("Unstable vehicle", "A damaged vehicle. Stay out of its lane, boost through it, or shoot it with the rail gun. Even dodged, it spreads a little stress.", UNSTABLE, Play::Approach),
    e("Debris", "Incoming debris. Shift away before impact, or boost through it.", DEBRIS, Play::Approach),
    e("Drone", "A hostile drone. Shift away, boost through it, or fire the rail gun.", DRONE, Play::Approach),
    e("Full blockade", "A barrier across all three lanes. Boost on the bright chime to breach it.", BLOCKADE, Play::Loop),
    e("Boost chime", "Boost now: a blockade arrives in under a second.", BOOST_CHIME, Play::Once),
    e("Repair pickup", "Restores hull when you are in its lane as it arrives.", REPAIR, Play::Approach),
    e("Salvage", "Score and debrief credits when you are in its lane as it arrives.", SALVAGE, Play::Approach),
    e("Ghost car", "From 17 October to the day after Halloween, a flow vehicle in costume: its pulse with a hollow whistle. Meet it as usual.", GHOST_FLOW, Play::Approach),
    e("Cargo crate", "In the Friday night run, salvage as a cargo crate: its pulse with a bright chime. Be in its lane for double salvage points.", CARGO_CRATE, Play::Approach),
    e("Priority notice", "Plays with every priority vehicle as it is announced.", PRIORITY_NOTICE, Play::Once),
    e("Stress warning", "Network pressure is climbing. Also plays with heavy blockers and unstable vehicles as they are announced.", STRESS_WARNING, Play::Once),
    e("Overload danger", "High stress. The corridor is close to collapse. Repeats while stress stays high.", OVERLOAD, Play::Once),
    e("Recovery window", "Stress is falling out of danger.", RECOVERY, Play::Once),
    e("Clean pass", "You met traffic in its lane: threaded, cleared or held.", CLEAN, Play::Once),
    e("Dodge", "A hazard went by in another lane.", PASS, Play::Once),
    e("Impact", "Something hit the skimmer. Hull lost and the multiplier resets.", IMPACT, Play::Once),
    e("Boost smash", "You boosted through something: debris, a drone, a blocker or a blockade.", SMASH, Play::Once),
    e("Flow lane missed", "A flow vehicle went by without you. Stress rises.", MISSED, Play::Once),
    e("Repair secured", "Hull restored.", REPAIR_GET, Play::Once),
    e("Salvage secured", "Salvage collected.", SALVAGE_GET, Play::Once),
    e("Boost", "Boost engaged, for just under a second.", BOOST, Play::Once),
    e("Boost ready", "Your boost has recharged.", BOOST_READY, Play::Once),
    e("Rail gun", "The rail gun fires at the nearest drone or unstable vehicle in the last second of its approach.", RAILGUN, Play::Once),
    e("Rail gun hit", "The rail gun destroyed its target.", RAILGUN_HIT, Play::Once),
    e("No lock", "The rail gun found nothing in range to shoot. Fire again as the target closes in.", NO_LOCK, Play::Once),
    e("Slowdown pulse", "Emergency slowdown: fast passers, priority traffic and heavy blockers are eased for a few seconds.", SLOWDOWN, Play::Once),
    e("Priority override", "Priority traffic is held for you for a few seconds, whatever lane you are in.", OVERRIDE, Play::Once),
    e("Not ready", "A tool is recharging or out of charges.", DENIED, Play::Once),
    e("Shield", "Shield plating absorbed a hit.", SHIELD, Play::Once),
    e("Multiplier up", "Four clean plays in a row raised the score multiplier.", MULTIPLIER, Play::Once),
    e("Sector start", "A new sector begins.", SECTOR_START, Play::Once),
    e("Sector clear", "Sector clear bonus: stress eases and the multiplier rises.", SECTOR_CLEAR, Play::Once),
    e("Evacuation train through", "In the monthly Black Sky event, an evacuation train cleared: triple priority points.", CONVOY, Play::Once),
];
