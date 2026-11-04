//! The pilot's saved progress: the original's credits, parts, intel, rank
//! and story stage, plus the workshop's permanent parts. Saved through
//! `Records` under `neon-rail/profile`.

use std::collections::BTreeMap;

use hearken::Ctx;
use parlor_core::{Parlor, Records};
use serde::{Deserialize, Serialize};

use crate::rules::{Loadout, Mode, Phase, Route, Run, Upgrade, COLD_START_SECTORS};
use crate::story::{total_logs, INTEL_GATES, LOGS, STRANDS};

pub const KEY: &str = "neon-rail/profile";

/// Rail gun kills that earn the Ace gunner licence.
pub const ACE_KILLS: u64 = 50;

/// Black Sky extractions that earn ranks 2 to 5.
pub const RANK_EXTRACTIONS: [u32; 4] = [1, 4, 10, 20];

/// The most score an endless run turns into credits: one strong Black
/// Sky win's worth, so the endless rail is not a credit geyser.
pub const ENDLESS_PAY_CAP: u64 = 40_000;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub credits: u64,
    pub parts: u64,
    pub intel: u64,
    /// Pilot rank, 1 to 5 (0 in an old or empty save reads as 1).
    pub rank: u32,
    pub lifetime_score: u64,
    pub best_score: u64,
    pub runs_started: u64,
    pub runs_completed: u64,
    /// How many archive logs are recovered, in order.
    pub story_stage: usize,
    pub tutorial_complete: bool,
    /// Black Sky Runs finished to extraction.
    pub extractions: u32,
    /// Workshop parts bought, by id.
    pub owned: Vec<String>,
    /// Rail gun kills, lifetime.
    pub kills: u64,
    /// Spoken call-outs turned off.
    pub quiet_callouts: bool,
    /// The furthest sector reached on the endless rail.
    pub best_endless_sector: u32,
    /// Monthly Black Sky events flown to the end, which open the event log.
    pub black_sky_events: u32,
    /// Sectors cleared by route, lifetime, by `Route::key`.
    pub route_clears: BTreeMap<String, u32>,
    /// Runs lost to a hull breach or a network overload.
    pub failures: u32,
    /// Black Sky Runs flown to extraction after the data core was complete.
    pub origin_runs: u32,
    /// Strand logs recovered, as `<strand id>/<n>`.
    pub recovered: Vec<String>,
    /// Cold starts flown to extraction.
    pub cold_clears: u32,
    /// The furthest sector reached in a cold start.
    pub best_cold_sector: u32,
}

/// A permanent part from the workshop.
pub struct Part {
    pub id: &'static str,
    pub name: &'static str,
    pub blurb: &'static str,
    pub credits: u64,
    pub parts: u64,
    /// The pilot rank it is sold from.
    pub rank: u32,
}

pub const PARTS: &[Part] = &[
    Part {
        id: "frame",
        name: "Reinforced frame",
        blurb: "15 more hull on every run.",
        credits: 150,
        parts: 0,
        rank: 1,
    },
    Part {
        id: "pulse",
        name: "Spare pulse cell",
        blurb: "One more slowdown pulse charge on every run.",
        credits: 200,
        parts: 1,
        rank: 1,
    },
    Part {
        id: "override",
        name: "Override licence",
        blurb: "One more priority override charge on every run.",
        credits: 200,
        parts: 1,
        rank: 1,
    },
    Part {
        id: "boost",
        name: "Boost tuning",
        blurb: "Boost recharges a second faster.",
        credits: 250,
        parts: 2,
        rank: 1,
    },
    Part {
        id: "storm",
        name: "Storm charts",
        blurb: "Opens the storm rail as a route choice: the hardest route, with the biggest sector bonus.",
        credits: 300,
        parts: 2,
        rank: 3,
    },
    Part {
        id: "magnet",
        name: "Pickup magnet",
        blurb: "Pickups one lane away are pulled in.",
        credits: 300,
        parts: 2,
        rank: 1,
    },
    Part {
        id: "dampers",
        name: "Stress dampers",
        blurb: "Every stress gain is a tenth smaller.",
        credits: 350,
        parts: 3,
        rank: 1,
    },
    Part {
        id: "nose",
        name: "Armoured nose",
        blurb: "Blockade collisions do half damage.",
        credits: 300,
        parts: 2,
        rank: 2,
    },
    Part {
        id: "sensors",
        name: "Long-range sensors",
        blurb: "Repairs and salvage are announced half a second sooner.",
        credits: 300,
        parts: 2,
        rank: 2,
    },
    Part {
        id: "capacitor",
        name: "Capacitor bank",
        blurb: "Boost lasts 0.3 seconds longer.",
        credits: 350,
        parts: 3,
        rank: 3,
    },
    Part {
        id: "heatsink",
        name: "Heat sink",
        blurb: "The rail gun cools 0.15 seconds faster.",
        credits: 400,
        parts: 3,
        rank: 3,
    },
    Part {
        id: "emitter",
        name: "Shield emitter",
        blurb: "Every run starts with one shield charge.",
        credits: 500,
        parts: 4,
        rank: 4,
    },
    Part {
        id: "governor",
        name: "Chain governor",
        blurb: "A hit halves your multiplier instead of resetting it.",
        credits: 600,
        parts: 5,
        rank: 5,
    },
    Part {
        id: "tuner",
        name: "Relay tuner",
        blurb: "A data relay recovers 2 intel fragments.",
        credits: 400,
        parts: 3,
        rank: 1,
    },
    Part {
        id: "jammer",
        name: "Hunter jammer",
        blurb: "Drones and unstable vehicles are announced half a second sooner.",
        credits: 450,
        parts: 4,
        rank: 1,
    },
];

/// The pilot rank the Relay tuner needs, so doubled intel cannot rush
/// the data core from the first runs.
pub const TUNER_RANK: u32 = 4;

/// Cold starts finished that earn Hot chain.
pub const HOT_CHAIN_CLEARS: u32 = 3;

/// What the forecaster says when a cold start clears the storm rail.
pub const STORM_GIFT: &str =
    "Storm charts fitted free. The forecaster: You flew my storm blind. Keep the charts.";

impl Profile {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Profile {
        Records::load(ctx, KEY)
    }

    /// The profile as last read, for a Settings row.
    pub fn peek(ctx: &Ctx<Parlor>) -> Profile {
        Records::peek(ctx, KEY)
    }

    pub fn save(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, KEY, self);
    }

    pub fn rank(&self) -> u32 {
        self.rank.max(1)
    }

    /// Pilot rank: a licence earned by Black Sky extractions alone, one
    /// rank at each of `RANK_EXTRACTIONS`, never down. Score, lost runs,
    /// practice runs and challenges move nothing.
    pub fn recalculate_rank(&mut self) {
        let rank = 1 + RANK_EXTRACTIONS
            .iter()
            .filter(|&&n| self.extractions >= n)
            .count() as u32;
        self.rank = rank.max(self.rank());
    }

    /// What the next rank costs, for the progress page.
    pub fn next_rank_line(&self) -> String {
        match RANK_EXTRACTIONS.get(self.rank() as usize - 1) {
            Some(&n) => format!("Rank {} at {}.", self.rank() + 1, extractions_phrase(n)),
            None => "Top rank.".to_string(),
        }
    }

    pub fn owns(&self, id: &str) -> bool {
        self.owned.iter().any(|o| o == id)
    }

    pub fn ace_gunner(&self) -> bool {
        self.kills >= ACE_KILLS
    }

    pub fn endless_unlocked(&self) -> bool {
        self.extractions > 0
    }

    /// The first run is the guided one, until training is done.
    pub fn needs_tutorial(&self) -> bool {
        !self.tutorial_complete
    }

    /// What still keeps a part off the workshop's shelf, as how it opens;
    /// `None` once it can be bought.
    pub fn part_lock(&self, part: &Part) -> Option<String> {
        match part.id {
            "tuner" => {
                let log = self.strand_recovered(strand_index("signal")) >= 1;
                let rank = self.rank() >= TUNER_RANK;
                match (log, rank) {
                    (true, true) => None,
                    (false, true) => Some("Opens with the signal's first log.".to_string()),
                    (true, false) => Some(format!("Opens at pilot rank {TUNER_RANK}.")),
                    (false, false) => Some(format!(
                        "Opens with the signal's first log, at pilot rank {TUNER_RANK}."
                    )),
                }
            }
            "jammer" => {
                let s = strand_index("security");
                (self.strand_recovered(s) < STRANDS[s].logs.len())
                    .then(|| "Opens with Security's last log.".to_string())
            }
            _ => None,
        }
    }

    /// Buys a workshop part: `Err` says why not.
    pub fn buy(&mut self, part: &Part) -> Result<(), String> {
        if self.owns(part.id) {
            return Err(format!("{} is already fitted.", part.name));
        }
        if part.id == "storm" && self.rank() < part.rank {
            return Err("Storm charts need pilot rank 3.".to_string());
        }
        if self.rank() < part.rank {
            return Err(format!("{} needs pilot rank {}.", part.name, part.rank));
        }
        if let Some(how) = self.part_lock(part) {
            return Err(format!("{} is locked. {how}", part.name));
        }
        if self.credits < part.credits || self.parts < part.parts {
            return Err(format!(
                "Not enough credits or parts. {} needs {} credits and {}. You have {} credits and {}.",
                part.name,
                part.credits,
                parts_phrase(part.parts),
                self.credits,
                parts_phrase(self.parts)
            ));
        }
        self.credits -= part.credits;
        self.parts -= part.parts;
        self.owned.push(part.id.to_string());
        Ok(())
    }

    /// The skimmer this pilot launches: rank bonuses as the original gave
    /// them, then the workshop's parts.
    pub fn loadout(&self) -> Loadout {
        let mut l = Loadout::default();
        let rank = self.rank();
        if rank >= 2 {
            l.max_hull += 10;
        }
        if rank >= 3 {
            l.override_charges = 3;
        }
        if rank >= 4 {
            l.slowdown_charges = 3;
        }
        if rank >= 5 {
            l.start_multiplier = 2;
        }
        if self.owns("frame") {
            l.max_hull += 15;
        }
        if self.owns("pulse") {
            l.slowdown_charges += 1;
        }
        if self.owns("override") {
            l.override_charges += 1;
        }
        if self.owns("boost") {
            l.boost_cooldown -= 1.0;
        }
        if self.owns("magnet") {
            l.magnet = true;
        }
        if self.owns("dampers") {
            l.stress_damp = 0.1;
        }
        if self.ace_gunner() {
            l.weapon_cooldown = 0.6;
        }
        if self.owns("nose") {
            l.armoured_nose = true;
        }
        if self.owns("sensors") {
            l.sensors = true;
        }
        if self.owns("capacitor") {
            l.boost_duration += 0.3;
        }
        if self.owns("heatsink") {
            l.weapon_cooldown -= 0.15;
        }
        if self.owns("emitter") {
            l.shields += 1;
        }
        if self.owns("governor") {
            l.governor = true;
        }
        l.relay_tuner = self.owns("tuner");
        l.jammer = self.owns("jammer");
        l
    }

    /// Whether cold starts have earned upgrade `u` for the pool.
    pub fn has_earned(&self, u: Upgrade) -> bool {
        match u {
            Upgrade::HardLock => self.best_cold_sector >= 4,
            Upgrade::ScrapMesh => self.best_cold_sector >= 7,
            Upgrade::DeadMan => self.cold_clears >= 1,
            Upgrade::HotChain => self.cold_clears >= HOT_CHAIN_CLEARS,
            _ => false,
        }
    }

    /// The upgrades earned for the pool between sectors.
    pub fn earned_upgrades(&self) -> Vec<Upgrade> {
        Upgrade::EARNED
            .into_iter()
            .filter(|&u| self.has_earned(u))
            .collect()
    }

    /// The earned upgrades, and how the rest open, for Pilot progress.
    pub fn upgrades_page(&self) -> String {
        let earned: Vec<&str> = self.earned_upgrades().iter().map(|u| u.name()).collect();
        let mut s = if earned.is_empty() {
            "No upgrades earned yet.".to_string()
        } else {
            format!("Upgrades earned: {}.", earned.join(", "))
        };
        let locked: Vec<String> = Upgrade::EARNED
            .into_iter()
            .filter(|&u| !self.has_earned(u))
            .map(|u| format!("{}, {}", u.name(), earned_how(u)))
            .collect();
        if !locked.is_empty() {
            s.push_str(&format!(" Locked: {}.", locked.join(". ")));
        }
        s
    }

    /// The routes the pilot may be offered.
    pub fn routes(&self) -> Vec<Route> {
        Route::CHOOSABLE
            .into_iter()
            .filter(|r| *r != Route::Storm || self.owns("storm"))
            .collect()
    }

    /// The rank bonuses unlocked, as the original's progress page read them.
    pub fn rank_bonuses(&self) -> String {
        let rank = self.rank();
        let mut s = String::new();
        if rank >= 2 {
            s += "Reinforced hull. ";
        }
        if rank >= 3 {
            s += "Extra priority override charge. ";
        }
        if rank >= 4 {
            s += "Extra slowdown pulse charge. ";
        }
        if rank >= 5 {
            s += "Elite courier licence active. ";
        }
        if s.is_empty() {
            s = "No rank bonuses unlocked yet.".to_string();
        }
        s.trim_end().to_string()
    }

    /// The pilot progress pages.
    pub fn pages(&self) -> Vec<String> {
        let mut owned: Vec<&str> = PARTS
            .iter()
            .filter(|p| self.owns(p.id))
            .map(|p| p.name)
            .collect();
        if self.ace_gunner() {
            owned.push("Ace gunner licence");
        }
        let fitted = if owned.is_empty() {
            "No workshop parts fitted yet.".to_string()
        } else {
            format!("Fitted: {}.", owned.join(", "))
        };
        let modes = if self.endless_unlocked() {
            format!(
                "Endless rail and cold start unlocked. {}",
                self.cold_start_record()
            )
        } else {
            "Finish a Black Sky Run to extraction to unlock the endless rail and cold start."
                .to_string()
        };
        let mut pages = vec![
            format!(
                "Rank {}. Credits {}. Parts {}. Intel fragments {}.",
                self.rank(),
                self.credits,
                self.parts,
                self.intel
            ),
            format!(
                "Best score {}. Lifetime score {}. Runs started {}. Runs completed {}. Extractions {}. {}",
                self.best_score,
                self.lifetime_score,
                self.runs_started,
                self.runs_completed,
                self.extractions,
                self.next_rank_line()
            ),
            format!(
                "Story logs recovered {} of {}. {}",
                self.logs_recovered(),
                total_logs(),
                if self.tutorial_complete {
                    "Tutorial complete."
                } else {
                    "Tutorial not complete."
                }
            ),
            format!("Rank bonuses. {}", self.rank_bonuses()),
            format!(
                "{fitted} Rail gun kills {} of {ACE_KILLS} for the Ace gunner licence. {modes}",
                self.kills.min(ACE_KILLS)
            ),
        ];
        // Earned upgrades come from cold starts, which open with the
        // endless rail.
        if self.endless_unlocked() {
            pages.push(self.upgrades_page());
        }
        pages
    }

    /// Cold starts finished and the furthest sector reached, for Records.
    pub fn cold_start_record(&self) -> String {
        if self.best_cold_sector == 0 {
            "No cold start flown yet.".to_string()
        } else {
            format!(
                "Cold starts finished {}, furthest sector {} of {}.",
                self.cold_clears, self.best_cold_sector, COLD_START_SECTORS
            )
        }
    }

    /// Moves the story on to `stage`, never back: whether anything new was
    /// recovered.
    pub fn unlock_story(&mut self, stage: usize) -> bool {
        let stage = stage.min(LOGS.len());
        if stage > self.story_stage {
            self.story_stage = stage;
            true
        } else {
            false
        }
    }

    /// Whether the data core's twelve logs are all recovered.
    pub fn core_complete(&self) -> bool {
        self.story_stage >= LOGS.len()
    }

    pub fn route_clears(&self, route: Route) -> u32 {
        self.route_clears.get(route.key()).copied().unwrap_or(0)
    }

    /// How many of strand `s`'s logs are recovered: they open in order.
    pub fn strand_recovered(&self, s: usize) -> usize {
        let strand = &STRANDS[s];
        (0..strand.logs.len())
            .take_while(|n| {
                self.recovered
                    .iter()
                    .any(|r| r == &strand_key(strand.id, *n))
            })
            .count()
    }

    /// Every log recovered, spine and strands.
    pub fn logs_recovered(&self) -> usize {
        self.story_stage.min(LOGS.len())
            + (0..STRANDS.len())
                .map(|s| self.strand_recovered(s))
                .sum::<usize>()
    }

    /// Opens the next log of each strand whose condition is met: at most
    /// one a strand, as `(strand, log)`.
    pub fn unlock_strands(&mut self) -> Vec<(usize, usize)> {
        let mut new = Vec::new();
        for (s, strand) in STRANDS.iter().enumerate() {
            let n = self.strand_recovered(s);
            if let Some(log) = strand.logs.get(n) {
                if log.unlock.met(self) {
                    self.recovered.push(strand_key(strand.id, n));
                    new.push((s, n));
                }
            }
        }
        new
    }
}

/// How an earned upgrade opens, for a locked line.
pub fn earned_how(u: Upgrade) -> String {
    match u {
        Upgrade::HardLock => "reach cold start sector 4".to_string(),
        Upgrade::ScrapMesh => "reach cold start sector 7".to_string(),
        Upgrade::DeadMan => "finish a cold start".to_string(),
        Upgrade::HotChain => format!("finish {HOT_CHAIN_CLEARS} cold starts"),
        _ => String::new(),
    }
}

/// Where strand `id` sits in `STRANDS`.
fn strand_index(id: &str) -> usize {
    STRANDS
        .iter()
        .position(|s| s.id == id)
        .expect("a known strand")
}

/// The saved key of strand log `n` of strand `id`.
pub fn strand_key(id: &str, n: usize) -> String {
    format!("{id}/{n}")
}

pub const EVENT_PAYS: &str = "The Friday night run keeps to its own board. It pays no credits, parts or intel, and your rank and story stay as they were.";
/// What a cold start's debrief carries out: only the archive.
pub const COLD_START_PAYS: &str = "Cold start keeps to its own board. It pays no credits, parts or intel, and your rank stays as it was. Routes cleared, runs lost and kills still count toward the archive.";

/// What a finished run paid out.
#[derive(Clone, Debug, PartialEq)]
pub struct Debrief {
    pub won: bool,
    pub mode: Mode,
    pub reason: String,
    pub score: i64,
    pub hull: i32,
    pub stress: i32,
    pub multiplier: i32,
    pub sectors: usize,
    pub credits: u64,
    pub parts: u64,
    pub intel: u64,
    /// The unlock summary, sentence by sentence.
    pub unlocks: Vec<String>,
    /// Archive logs recovered by this run, by index.
    pub new_logs: Vec<usize>,
    /// Strand logs recovered by this run, as `(strand, log)`.
    pub new_strand_logs: Vec<(usize, usize)>,
    pub rank: u32,
    pub best_score: u64,
    /// The guided route flown again: nothing paid, nothing counted.
    pub practice: bool,
}

/// The original's `apply_run_rewards`, plus the workshop's currency, the
/// later archive logs and the endless rail.
pub fn debrief(p: &mut Profile, run: &Run, story_before: usize) -> Debrief {
    let won = matches!(run.phase, Phase::Over { won: true });
    let score = run.score.max(0) as u64;
    let rank_before = p.rank();
    let earned_before = p.earned_upgrades();
    let mut unlocks = Vec::new();

    // A Friday night run keeps to its own board: its doubled salvage pays no
    // rank, credits, parts, intel or story, and it leaves the pilot as it was.
    // A practice run (the guided route after training) pays and counts
    // nothing at all.
    let practice = run.mode == Mode::Tutorial && p.tutorial_complete;
    if run.mode.is_event() || practice {
        return Debrief {
            won,
            mode: run.mode,
            reason: run.reason.clone(),
            score: run.score,
            hull: run.hull,
            stress: run.stress,
            multiplier: run.multiplier,
            sectors: run.sectors_cleared,
            credits: 0,
            parts: 0,
            intel: 0,
            unlocks,
            new_logs: Vec::new(),
            new_strand_logs: Vec::new(),
            rank: rank_before,
            best_score: p.best_score,
            practice,
        };
    }

    // A cold start carries nothing out but the archive: no pay, no rank,
    // no personal best, and it does not count as a finished run.
    let cold = run.mode == Mode::ColdStart;
    let mut credits = 0;
    let mut parts = 0;
    let mut intel = 0;
    if !cold {
        p.runs_completed += 1;
        p.lifetime_score += score;
        if score > p.best_score {
            p.best_score = score;
            unlocks.push("New personal best.".to_string());
        }
        let paid = if run.mode == Mode::Endless {
            score.min(ENDLESS_PAY_CAP)
        } else {
            score
        };
        credits = paid / 35;
        parts = run.sectors_cleared as u64 / 2;
        // Intel comes from the rail's data relays and from extraction,
        // never from a challenge, the corridor or the guided run.
        if matches!(run.mode, Mode::BlackSky | Mode::Endless) {
            intel = run.stats.intel_found as u64;
        }
        if won {
            credits += 75;
            parts += 1;
            if run.mode == Mode::BlackSky {
                intel += 1;
            }
        }
        if run.mode == Mode::Tutorial {
            p.tutorial_complete = true;
            credits += 50;
            unlocks.push("Tutorial complete.".to_string());
        }
        p.credits += credits;
        p.parts += parts;
        p.intel += intel;
    }

    let had_ace = p.ace_gunner();
    p.kills += run.stats.kills as u64;
    if !had_ace && p.ace_gunner() {
        unlocks.push("Ace gunner licence earned: the rail gun cools faster.".to_string());
    }
    if run.mode == Mode::BlackSky && won {
        p.extractions += 1;
        // The rank is the licence the extractions earn: said first, then
        // what it opens.
        p.recalculate_rank();
        if p.rank() > rank_before {
            unlocks.push(format!("Pilot rank advanced to {}.", p.rank()));
        }
        if p.extractions == 1 {
            unlocks.push("Endless rail and cold start unlocked.".to_string());
        }
        // Log 12 says the next run is yours: extractions after it count.
        if story_before >= LOGS.len() {
            p.origin_runs += 1;
        }
    }
    if !won && run.mode != Mode::Tutorial && run.reason != crate::rules::ABANDONED {
        p.failures += 1;
    }
    for sector in run.sectors.iter().take(run.sectors_cleared) {
        if Route::CHOOSABLE.contains(&sector.route) {
            *p.route_clears
                .entry(sector.route.key().to_string())
                .or_insert(0) += 1;
        }
    }
    if run.mode == Mode::Endless {
        let reached = run.current_sector.map_or(1, |i| i + 1) as u32;
        if reached > p.best_endless_sector {
            p.best_endless_sector = reached;
        }
    }
    // A cold start keeps its own record: finishes, and how far it got.
    if cold {
        let reached = if won {
            COLD_START_SECTORS as u32
        } else {
            run.current_sector.map_or(1, |i| i + 1) as u32
        };
        if won {
            p.cold_clears += 1;
            unlocks.push(format!("Cold starts finished: {}.", p.cold_clears));
        }
        if reached > p.best_cold_sector {
            p.best_cold_sector = reached;
            if !won {
                unlocks.push(format!("Furthest cold start sector yet: {reached}."));
            }
        }
        let new: Vec<Upgrade> = p
            .earned_upgrades()
            .into_iter()
            .filter(|u| !earned_before.contains(u))
            .collect();
        match new.as_slice() {
            [] => {}
            [u] => unlocks.push(format!(
                "New upgrade between sectors: {}, {}.",
                u.name(),
                u.gist()
            )),
            [rest @ .., last] => {
                let names: Vec<&str> = rest.iter().map(|u| u.name()).collect();
                unlocks.push(format!(
                    "New upgrades between sectors: {} and {}.",
                    names.join(", "),
                    last.name()
                ));
            }
        }
        // The storm rail flown without charts, the hard way: the
        // forecaster hands them over.
        let storm_cleared = run
            .sectors
            .iter()
            .take(run.sectors_cleared)
            .any(|s| s.route == Route::Storm);
        if storm_cleared && !p.owns("storm") {
            p.owned.push("storm".to_string());
            unlocks.push(STORM_GIFT.to_string());
        }
    }

    p.unlock_story(run.story_stage);
    if p.story_stage < 1 {
        p.unlock_story(1);
    }
    if won && p.story_stage < 5 {
        p.unlock_story(5);
    }
    // The later logs open on intel, one a run.
    if p.story_stage >= 5 {
        let gate = p.story_stage - 5;
        if gate < INTEL_GATES.len() && p.intel >= INTEL_GATES[gate] {
            p.unlock_story(p.story_stage + 1);
        }
    }
    let new_logs: Vec<usize> = (story_before..p.story_stage).collect();
    let new_strand_logs = p.unlock_strands();

    Debrief {
        won,
        mode: run.mode,
        reason: run.reason.clone(),
        score: run.score,
        hull: run.hull,
        stress: run.stress,
        multiplier: run.multiplier,
        sectors: run.sectors_cleared,
        credits,
        parts,
        intel,
        unlocks,
        new_logs,
        new_strand_logs,
        rank: p.rank(),
        best_score: p.best_score,
        practice: false,
    }
}

impl Debrief {
    /// The original's result pages.
    pub fn pages(&self) -> Vec<String> {
        let mut pages = vec![
            format!("{} Final score {}.", self.reason, self.score),
            format!(
                "Hull {}. Stress {} of {}. Multiplier {}. Sectors cleared {}.",
                self.hull,
                self.stress,
                crate::rules::MAX_STRESS,
                self.multiplier,
                self.sectors
            ),
        ];
        if self.practice {
            pages.push("Practice run. Nothing to pay.".to_string());
            return pages;
        }
        if self.credits > 0 || self.parts > 0 || self.intel > 0 {
            pages.push(if self.won {
                format!(
                    "Debrief rewards. Credits {}, from your score and a finished run. Parts {}, from sectors cleared and a finished run. Intel fragments {}.",
                    self.credits, self.parts, self.intel
                )
            } else {
                format!(
                    "Debrief rewards. Credits {}, from your score. Parts {}, from sectors cleared. Intel fragments {}.",
                    self.credits, self.parts, self.intel
                )
            });
        }
        match self.mode {
            Mode::FridayNight => pages.push(EVENT_PAYS.to_string()),
            Mode::ColdStart => pages.push(COLD_START_PAYS.to_string()),
            _ => {}
        }
        if !self.unlocks.is_empty() {
            pages.push(self.unlocks.join(" "));
        }
        // The logs themselves wait in the archive: a debrief is no place
        // for a page of story each.
        match self.new_logs.len() + self.new_strand_logs.len() {
            0 => {}
            1 => pages.push("1 new log in the Story archive.".to_string()),
            n => pages.push(format!("{n} new logs in the Story archive.")),
        }
        for &(s, n) in &self.new_strand_logs {
            let strand = &STRANDS[s];
            let log = &strand.logs[n];
            pages.push(format!(
                "{} log {}. {}. {}",
                strand.name,
                n + 1,
                log.title,
                log.pages.join(" ")
            ));
        }
        // A cold start keeps to its own board, read next; its pays page
        // already said the rank stays.
        if self.mode != Mode::ColdStart {
            pages.push(format!(
                "Pilot rank {}. Best score {}.",
                self.rank, self.best_score
            ));
        }
        pages
    }
}

/// "1 extraction", "4 extractions".
pub fn extractions_phrase(n: u32) -> String {
    if n == 1 {
        "1 extraction".to_string()
    } else {
        format!("{n} extractions")
    }
}

/// "1 part", "3 parts".
pub fn parts_phrase(n: u64) -> String {
    if n == 1 {
        "1 part".to_string()
    } else {
        format!("{n} parts")
    }
}
