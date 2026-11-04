//! The rules of Liberty Lights, with no screens or sound: fireworks, the
//! shows each level asks for, how a recreation is scored, and the clock a
//! round runs on. Everything here is driven by `dt` so tests can step it.

use std::cell::Cell;

use parlor_core::Rng;
use serde::{Deserialize, Serialize};

/// Levels in the campaign, as in the original.
pub const TOTAL_LEVELS: u32 = 20;
/// The accuracy, in percent, that clears a level.
pub const PASS_PERCENT: u32 = 60;
/// Milliseconds between fireworks in a generated show, before overlap.
pub const SPACING_MS: i32 = 2000;
/// Seconds between count-in ticks.
pub const BEAT: f64 = 0.5;
/// Count-in ticks before the go.
pub const COUNT_TICKS: u8 = 3;
/// Seconds from the first tick to the go, which is time zero of a show.
pub const COUNT_IN: f64 = BEAT * COUNT_TICKS as f64;
/// How long the sky stays quiet after the last firework of a show before
/// it is the player's turn, in milliseconds (the original's two seconds).
pub const LISTEN_TAIL_MS: i32 = 2000;
/// Accuracy points a hint costs.
pub const HINT_COST: u32 = 15;
/// A firework matched this well (right type and place, within 300 ms)
/// keeps a combo going.
pub const COMBO_MATCH: u32 = 95;
/// Show points each step of a combo adds, times the combo length.
pub const COMBO_STEP: u64 = 50;
/// The most fireworks a freestyle show holds.
pub const FREESTYLE_CAP: usize = 64;

/// A tough crowd pays timing points only this close, in milliseconds.
pub const TOUGH_WINDOW_MS: i32 = 300;
/// Show points for each signature firework matched near-perfectly.
pub const SIGNATURE_BONUS: u64 = 100;

/// How a show is scored, beyond the original's rules: the challenges'
/// twists. The default is the campaign's scoring.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scoring {
    /// Timing points only within `TOUGH_WINDOW_MS`.
    pub tough: bool,
    /// The firework that pays `SIGNATURE_BONUS`.
    pub signature: Option<Kind>,
}

/// The four kinds of firework, on keys 1 to 4.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kind {
    Boom,
    Crackle,
    Whistle,
    Sparkle,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Boom, Kind::Crackle, Kind::Whistle, Kind::Sparkle];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Boom => "boom",
            Kind::Crackle => "crackle",
            Kind::Whistle => "whistle",
            Kind::Sparkle => "sparkle",
        }
    }

    /// The number key that launches it.
    pub fn digit(self) -> u8 {
        self as u8 + 1
    }

    pub fn from_index(i: u32) -> Kind {
        Kind::ALL[(i as usize) % 4]
    }

    /// What it sounds like, for the tutorial and the practice room.
    pub fn describe(self) -> &'static str {
        match self {
            Kind::Boom => "a deep bass thump that rolls away",
            Kind::Crackle => "a high scatter of tiny pops",
            Kind::Whistle => "a rising whistle that ends in a small pop",
            Kind::Sparkle => "a soft glassy shimmer",
        }
    }
}

/// Where a firework goes up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pos {
    Left,
    Center,
    Right,
}

impl Pos {
    pub const ALL: [Pos; 3] = [Pos::Left, Pos::Center, Pos::Right];

    pub fn name(self) -> &'static str {
        match self {
            Pos::Left => "left",
            Pos::Center => "center",
            Pos::Right => "right",
        }
    }

    /// "on the left", "in the center".
    pub fn phrase(self) -> &'static str {
        match self {
            Pos::Left => "on the left",
            Pos::Center => "in the center",
            Pos::Right => "on the right",
        }
    }

    pub fn from_index(i: u32) -> Pos {
        Pos::ALL[(i as usize) % 3]
    }
}

/// One firework in a show: what, where, and when, in milliseconds from the
/// go.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Launch {
    pub kind: Kind,
    pub pos: Pos,
    pub ms: i32,
}

impl Launch {
    pub fn new(kind: Kind, pos: Pos, ms: i32) -> Launch {
        Launch { kind, pos, ms }
    }
}

/// The four bands of the campaign.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// Levels 1 to 5: one firework, match its type and timing.
    Single,
    /// Levels 6 to 10: two fireworks.
    Types,
    /// Levels 11 to 15: three fireworks, left, center and right.
    Positions,
    /// Levels 16 to 20: longer shows whose fireworks overlap.
    Overlap,
}

pub fn tier(level: u32) -> Tier {
    match level {
        0..=5 => Tier::Single,
        6..=10 => Tier::Types,
        11..=15 => Tier::Positions,
        _ => Tier::Overlap,
    }
}

/// The level's line in the level select, as the original wrote it.
pub fn tier_description(level: u32) -> &'static str {
    match tier(level) {
        Tier::Single => "Single firework timing.",
        Tier::Types => "Multiple firework types and timing.",
        Tier::Positions => "Adds left, center, and right launch positions.",
        Tier::Overlap => "Advanced overlapping fireworks.",
    }
}

/// How many fireworks a level's show holds.
pub fn level_size(level: u32) -> usize {
    match tier(level) {
        Tier::Single => 1,
        Tier::Types => 2,
        Tier::Positions => 3,
        Tier::Overlap => 3 + (level.min(TOTAL_LEVELS) - 15) as usize,
    }
}

/// Whether the level's fireworks go up anywhere but the center.
pub fn uses_positions(level: u32) -> bool {
    matches!(tier(level), Tier::Positions | Tier::Overlap)
}

thread_local! {
    static SEED: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Makes every show this thread builds from now on use `Rng::seeded(seed)`
/// (or the clock again, with `None`), so a test knows the show it will hear.
pub fn fix_seed(seed: Option<u64>) {
    SEED.with(|s| s.set(seed));
}

/// The random source a new show uses: the fixed seed, else the clock.
pub fn show_rng() -> Rng {
    match SEED.with(|s| s.get()) {
        Some(seed) => Rng::seeded(seed),
        None => Rng::from_time(),
    }
}

/// The show a level asks for, as the original built it: `level_size`
/// fireworks of random kinds two seconds apart, random places from level
/// 11, and from level 16 each may come up to 0.8 s early so they overlap.
/// Two changes, since a count-in now marks time zero: the show never starts
/// before the go, and after level 1 it starts up to two seconds after the go
/// on a half beat, so even one firework is a test of timing.
pub fn generate(level: u32, rng: &mut Rng) -> Vec<Launch> {
    let n = level_size(level);
    let positions = uses_positions(level);
    let overlap = tier(level) == Tier::Overlap;
    let mut show = Vec::with_capacity(n);
    for i in 0..n {
        let kind = Kind::from_index(rng.range(0, 3) as u32);
        let pos = if positions {
            Pos::from_index(rng.range(0, 2) as u32)
        } else {
            Pos::Center
        };
        let mut ms = i as i32 * SPACING_MS;
        if overlap && rng.range(0, 1) == 1 {
            ms -= rng.range(200, 800);
        }
        show.push(Launch::new(kind, pos, ms));
    }
    let offset = if level <= 1 { 0 } else { rng.range(0, 4) * 500 };
    settle(&mut show, offset);
    show
}

/// A grand-finale round: longer and faster every round, always placed and
/// overlapping.
pub fn generate_finale(round: u32, rng: &mut Rng) -> Vec<Launch> {
    let n = (5 + round as usize).min(14);
    let spacing = (SPACING_MS - 100 * round as i32).max(900);
    let mut show = Vec::with_capacity(n);
    for i in 0..n {
        let kind = Kind::from_index(rng.range(0, 3) as u32);
        let pos = Pos::from_index(rng.range(0, 2) as u32);
        let mut ms = i as i32 * spacing;
        if rng.range(0, 1) == 1 {
            ms -= rng.range(150, spacing / 2);
        }
        show.push(Launch::new(kind, pos, ms));
    }
    settle(&mut show, rng.range(0, 2) * 500);
    show
}

/// Sorts a show by time and moves it so its first firework is `offset`
/// milliseconds after the go.
pub fn settle(show: &mut [Launch], offset: i32) {
    show.sort_by_key(|l| l.ms);
    if let Some(first) = show.first().map(|l| l.ms) {
        for l in show.iter_mut() {
            l.ms = l.ms - first + offset;
        }
    }
}

/// Timing points for being `diff` milliseconds off, the original's windows.
pub fn timing_points(diff: i32) -> u32 {
    match diff.unsigned_abs() {
        0..=100 => 25,
        101..=300 => 20,
        301..=500 => 15,
        501..=1000 => 10,
        1001..=2000 => 5,
        _ => 0,
    }
}

/// What a timing difference sounds like in words.
pub fn timing_word(diff: i32) -> &'static str {
    match diff.unsigned_abs() {
        0..=100 => "perfect",
        101..=300 => "good",
        301..=500 => "OK",
        501..=1000 => "poor",
        1001..=2000 => "very poor",
        _ => "missed",
    }
}

/// How well one launch matches one target firework, 0 to 100: 50 for the
/// type, 25 for the place, up to 25 for the timing.
pub fn match_points(target: &Launch, player: &Launch) -> u32 {
    match_points_with(target, player, Scoring::default())
}

/// `match_points` under a challenge's scoring: a tough crowd pays timing
/// points only within `TOUGH_WINDOW_MS`.
pub fn match_points_with(target: &Launch, player: &Launch, scoring: Scoring) -> u32 {
    let diff = player.ms - target.ms;
    if scoring.tough && diff.unsigned_abs() > TOUGH_WINDOW_MS as u32 {
        let mut points = 0;
        if player.kind == target.kind {
            points += 50;
        }
        if player.pos == target.pos {
            points += 25;
        }
        return points;
    }
    let mut points = 0;
    if player.kind == target.kind {
        points += 50;
    }
    if player.pos == target.pos {
        points += 25;
    }
    points + timing_points(player.ms - target.ms)
}

/// The best any of the player's launches matches `target`, and which one.
pub fn best_match(target: &Launch, player: &[Launch]) -> (u32, Option<usize>) {
    best_match_with(target, player, Scoring::default())
}

/// `best_match` under a challenge's scoring.
pub fn best_match_with(
    target: &Launch,
    player: &[Launch],
    scoring: Scoring,
) -> (u32, Option<usize>) {
    let mut best = (0, None);
    for (i, p) in player.iter().enumerate() {
        let points = match_points_with(target, p, scoring);
        if points > best.0 {
            best = (points, Some(i));
        }
    }
    best
}

/// The original's score, 0 to 100: every target firework takes its best
/// match among the player's launches, 20 points come off for each firework
/// too many or too few, and the total is a percentage of a perfect show.
pub fn score_pattern(target: &[Launch], player: &[Launch]) -> u32 {
    score_pattern_with(target, player, Scoring::default())
}

/// `score_pattern` under a challenge's scoring.
pub fn score_pattern_with(target: &[Launch], player: &[Launch], scoring: Scoring) -> u32 {
    if target.is_empty() {
        return 0;
    }
    let max = target.len() as i64 * 100;
    let mut total: i64 = target
        .iter()
        .map(|t| best_match_with(t, player, scoring).0 as i64)
        .sum();
    let difference = (player.len() as i64 - target.len() as i64).abs();
    total -= difference * 20;
    (total * 100 / max).clamp(0, 100) as u32
}

/// How the crowd takes a show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crowd {
    /// 80 percent and up.
    Cheer,
    /// 60 and up.
    Ooh,
    /// 40 and up.
    Aah,
    /// Below 40: a confused murmur.
    Murmur,
}

impl Crowd {
    pub fn for_accuracy(accuracy: u32) -> Crowd {
        match accuracy {
            80.. => Crowd::Cheer,
            60..=79 => Crowd::Ooh,
            40..=59 => Crowd::Aah,
            _ => Crowd::Murmur,
        }
    }

    /// The words for each reaction, read with the results.
    pub fn line(self) -> &'static str {
        match self {
            Crowd::Cheer => "The crowd cheers.",
            Crowd::Ooh => "The crowd oohs.",
            Crowd::Aah => "The crowd aahs.",
            Crowd::Murmur => "The crowd murmurs, confused.",
        }
    }
}

/// Everything a finished round is scored on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Judgement {
    /// The final accuracy, 0 to 100, after any hint.
    pub accuracy: u32,
    /// Each target firework's best match, in show order.
    pub per_target: Vec<u32>,
    /// The longest run of fireworks matched at `COMBO_MATCH` or better.
    pub best_combo: u32,
    /// Show points the combos added.
    pub combo_bonus: u64,
    /// Show points a festival's signature fireworks added.
    pub signature_bonus: u64,
    /// Accuracy times ten plus the combo bonus: what best scores keep.
    pub points: u64,
    pub launched: usize,
    pub expected: usize,
    pub hint_used: bool,
}

impl Judgement {
    pub fn passed(&self) -> bool {
        self.accuracy >= PASS_PERCENT
    }

    pub fn crowd(&self) -> Crowd {
        Crowd::for_accuracy(self.accuracy)
    }
}

/// Scores a round: the original's percentage, less `HINT_COST` if a hint
/// was heard, then combos for runs of near-perfect fireworks, which add
/// bonus points only when the show clears.
pub fn judge(target: &[Launch], player: &[Launch], hint_used: bool) -> Judgement {
    judge_with(target, player, hint_used, Scoring::default())
}

/// `judge` under a challenge's scoring: a tough crowd's narrow timing
/// window, and a festival's signature firework, which pays
/// `SIGNATURE_BONUS` for each one matched at `COMBO_MATCH` or better on a
/// cleared show.
pub fn judge_with(
    target: &[Launch],
    player: &[Launch],
    hint_used: bool,
    scoring: Scoring,
) -> Judgement {
    let mut accuracy = score_pattern_with(target, player, scoring);
    if hint_used {
        accuracy = accuracy.saturating_sub(HINT_COST);
    }
    let per_target: Vec<u32> = target
        .iter()
        .map(|t| best_match_with(t, player, scoring).0)
        .collect();
    let mut signature_bonus = 0u64;
    if let Some(kind) = scoring.signature {
        let hits = target
            .iter()
            .zip(&per_target)
            .filter(|(t, p)| t.kind == kind && **p >= COMBO_MATCH)
            .count() as u64;
        signature_bonus = hits * SIGNATURE_BONUS;
    }
    let mut combo = 0u32;
    let mut best_combo = 0u32;
    let mut combo_bonus = 0u64;
    for &points in &per_target {
        if points >= COMBO_MATCH {
            combo += 1;
            best_combo = best_combo.max(combo);
            if combo >= 2 {
                combo_bonus += COMBO_STEP * (combo as u64 - 1);
            }
        } else {
            combo = 0;
        }
    }
    // Combos pay only on a cleared show, so spraying extra fireworks to
    // catch every target cannot buy a best score on a failed one.
    if accuracy < PASS_PERCENT {
        combo_bonus = 0;
        signature_bonus = 0;
    }
    Judgement {
        accuracy,
        per_target,
        best_combo,
        combo_bonus,
        signature_bonus,
        points: accuracy as u64 * 10 + combo_bonus + signature_bonus,
        launched: player.len(),
        expected: target.len(),
        hint_used,
    }
}

/// A show in words, for the hint.
pub fn describe_show(show: &[Launch]) -> String {
    let count = match show.len() {
        1 => "One firework".to_string(),
        n => format!("{n} fireworks"),
    };
    let mut parts = Vec::new();
    let mut last: Option<i32> = None;
    for l in show {
        let when = match last {
            None if l.ms == 0 => "right on the go".to_string(),
            None => format!("{} after the go", seconds(l.ms)),
            Some(prev) if l.ms == prev => "at the same moment".to_string(),
            Some(prev) => format!("{} later", seconds(l.ms - prev)),
        };
        parts.push(format!("{} {}, {when}", l.kind.name(), l.pos.phrase()));
        last = Some(l.ms);
    }
    format!("{count}: {}.", parts.join("; "))
}

/// "1.5 seconds", "half a second".
pub fn seconds(ms: i32) -> String {
    let tenths = (ms as f64 / 100.0).round() as i64;
    match tenths {
        5 => "half a second".to_string(),
        10 => "1 second".to_string(),
        t if t % 10 == 0 => format!("{} seconds", t / 10),
        t => format!("{}.{} seconds", t / 10, (t % 10).abs()),
    }
}

/// Plays a show forward in time: hand it `dt`, get back the fireworks that
/// go up in that step.
#[derive(Clone, Debug)]
pub struct Playback {
    show: Vec<Launch>,
    /// Seconds since time zero.
    clock: f64,
    cursor: usize,
}

impl Playback {
    pub fn new(show: Vec<Launch>) -> Playback {
        Playback {
            show,
            clock: 0.0,
            cursor: 0,
        }
    }

    pub fn update(&mut self, dt: f64) -> Vec<Launch> {
        self.clock += dt;
        let mut out = Vec::new();
        while let Some(l) = self.show.get(self.cursor) {
            if self.clock * 1000.0 + 1e-6 >= l.ms as f64 {
                out.push(*l);
                self.cursor += 1;
            } else {
                break;
            }
        }
        out
    }

    /// Every firework is up.
    pub fn launched_all(&self) -> bool {
        self.cursor >= self.show.len()
    }

    /// Every firework is up and `tail_ms` has passed since the last.
    pub fn finished(&self, tail_ms: i32) -> bool {
        let last = self.show.last().map_or(0, |l| l.ms);
        self.launched_all() && self.clock * 1000.0 >= (last + tail_ms) as f64
    }
}

/// Where a round is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Waiting for the player to ask to hear the show.
    Ready,
    /// The count-in and the show are playing.
    Listening,
    /// The show is over; waiting for the player to start their count-in.
    YourTurn,
    /// The player's count-in has run or is running; launches are recorded.
    Playing,
}

/// Something a round asks the screen to play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    /// Count-in tick 1, 2 or 3.
    Tick(u8),
    /// Time zero.
    Go,
    /// A firework of the show goes up.
    Burst(Launch),
    /// The show is over: the player's turn.
    TurnBegins,
}

/// One round: hear a show, then launch it back.
#[derive(Clone, Debug)]
pub struct Round {
    pub show: Vec<Launch>,
    pub phase: Phase,
    /// Seconds since the current count-in began.
    clock: f64,
    ticks: u8,
    gone: bool,
    playback: Playback,
    pub launches: Vec<Launch>,
    /// How many times the show has been heard.
    pub listens: u32,
    pub hint_used: bool,
}

impl Round {
    pub fn new(show: Vec<Launch>) -> Round {
        Round {
            playback: Playback::new(show.clone()),
            show,
            phase: Phase::Ready,
            clock: 0.0,
            ticks: 0,
            gone: false,
            launches: Vec::new(),
            listens: 0,
            hint_used: false,
        }
    }

    fn start_count(&mut self) {
        self.clock = 0.0;
        self.ticks = 0;
        self.gone = false;
    }

    /// Starts the count-in and the show, from `Ready` or (to hear it again,
    /// dropping any launches) from `YourTurn` and `Playing`. False while it
    /// is already playing.
    pub fn listen(&mut self) -> bool {
        if self.phase == Phase::Listening {
            return false;
        }
        self.phase = Phase::Listening;
        self.launches.clear();
        self.playback = Playback::new(self.show.clone());
        self.listens += 1;
        self.start_count();
        true
    }

    /// Starts the player's count-in. Only from `YourTurn`.
    pub fn play(&mut self) -> bool {
        if self.phase != Phase::YourTurn {
            return false;
        }
        self.phase = Phase::Playing;
        self.start_count();
        true
    }

    /// Milliseconds since the go (negative during the count-in).
    pub fn elapsed_ms(&self) -> i32 {
        ((self.clock - COUNT_IN) * 1000.0).round() as i32
    }

    /// Records a launch now, while playing.
    pub fn launch(&mut self, kind: Kind, pos: Pos) -> Option<Launch> {
        if self.phase != Phase::Playing {
            return None;
        }
        let l = Launch::new(kind, pos, self.elapsed_ms());
        self.launches.push(l);
        Some(l)
    }

    /// Takes back the last launch, as the original's Backspace.
    pub fn undo(&mut self) -> Option<Launch> {
        if self.phase != Phase::Playing {
            return None;
        }
        self.launches.pop()
    }

    pub fn judge(&self) -> Judgement {
        judge(&self.show, &self.launches, self.hint_used)
    }

    fn count_cues(&mut self, out: &mut Vec<Cue>) {
        while self.ticks < COUNT_TICKS && self.clock + 1e-9 >= self.ticks as f64 * BEAT {
            self.ticks += 1;
            out.push(Cue::Tick(self.ticks));
        }
        if !self.gone && self.clock + 1e-9 >= COUNT_IN {
            self.gone = true;
            out.push(Cue::Go);
        }
    }

    /// Moves the round on by `dt` seconds. Call with 0 right after
    /// `listen` or `play` to hear the first tick at once.
    pub fn update(&mut self, dt: f64) -> Vec<Cue> {
        let mut out = Vec::new();
        match self.phase {
            Phase::Ready | Phase::YourTurn => {}
            Phase::Listening => {
                self.clock += dt;
                self.count_cues(&mut out);
                if self.gone {
                    let since_go = self.clock - COUNT_IN;
                    let step = since_go - self.playback.clock;
                    for l in self.playback.update(step.max(0.0)) {
                        out.push(Cue::Burst(l));
                    }
                    if self.playback.finished(LISTEN_TAIL_MS) {
                        self.phase = Phase::YourTurn;
                        out.push(Cue::TurnBegins);
                    }
                }
            }
            Phase::Playing => {
                self.clock += dt;
                self.count_cues(&mut out);
            }
        }
        out
    }
}

/// A freestyle show being composed: time zero is the first launch.
#[derive(Clone, Debug, Default)]
pub struct Composer {
    pub launches: Vec<Launch>,
    clock: Option<f64>,
}

impl Composer {
    pub fn new() -> Composer {
        Composer::default()
    }

    pub fn update(&mut self, dt: f64) {
        if let Some(c) = self.clock.as_mut() {
            *c += dt;
        }
    }

    /// Records a launch, unless the show is full.
    pub fn launch(&mut self, kind: Kind, pos: Pos) -> Option<Launch> {
        if self.launches.len() >= FREESTYLE_CAP {
            return None;
        }
        let clock = *self.clock.get_or_insert(0.0);
        let l = Launch::new(kind, pos, (clock * 1000.0).round() as i32);
        self.launches.push(l);
        Some(l)
    }

    pub fn undo(&mut self) -> Option<Launch> {
        let l = self.launches.pop();
        if self.launches.is_empty() {
            self.clock = None;
        }
        l
    }

    /// Milliseconds from the first launch to the last.
    pub fn length_ms(&self) -> i32 {
        self.launches.last().map_or(0, |l| l.ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_grow_like_the_original() {
        assert_eq!(level_size(1), 1);
        assert_eq!(level_size(5), 1);
        assert_eq!(level_size(6), 2);
        assert_eq!(level_size(11), 3);
        assert_eq!(level_size(16), 4);
        assert_eq!(level_size(20), 8);
        assert!(!uses_positions(10));
        assert!(uses_positions(11));
    }

    #[test]
    fn generated_shows_are_sorted_and_start_at_or_after_the_go() {
        for level in 1..=TOTAL_LEVELS {
            for seed in 0..20 {
                let show = generate(level, &mut Rng::seeded(seed));
                assert_eq!(show.len(), level_size(level));
                assert!(show.windows(2).all(|w| w[0].ms <= w[1].ms));
                assert!(show[0].ms >= 0 && show[0].ms <= 2000);
                if !uses_positions(level) {
                    assert!(show.iter().all(|l| l.pos == Pos::Center));
                }
            }
        }
        assert_eq!(generate(1, &mut Rng::seeded(3))[0].ms, 0);
    }

    #[test]
    fn overlap_levels_bring_fireworks_closer_than_two_seconds() {
        let close = (0..50).any(|seed| {
            let show = generate(18, &mut Rng::seeded(seed));
            show.windows(2).any(|w| w[1].ms - w[0].ms < SPACING_MS)
        });
        assert!(close);
    }

    #[test]
    fn timing_windows_match_the_original() {
        assert_eq!(timing_points(0), 25);
        assert_eq!(timing_points(-100), 25);
        assert_eq!(timing_points(101), 20);
        assert_eq!(timing_points(300), 20);
        assert_eq!(timing_points(500), 15);
        assert_eq!(timing_points(1000), 10);
        assert_eq!(timing_points(2000), 5);
        assert_eq!(timing_points(2001), 0);
    }

    #[test]
    fn a_perfect_copy_scores_100_and_the_wrong_count_costs_20_each() {
        let show = vec![
            Launch::new(Kind::Boom, Pos::Left, 0),
            Launch::new(Kind::Sparkle, Pos::Right, 2000),
        ];
        assert_eq!(score_pattern(&show, &show), 100);
        let mut extra = show.clone();
        extra.push(Launch::new(Kind::Crackle, Pos::Center, 4000));
        // 200 - 20 of 200 is 90 percent.
        assert_eq!(score_pattern(&show, &extra), 90);
        assert_eq!(score_pattern(&show, &[]), 0);
        // Right type only, wrong place, two seconds late: 50 + 0 + 5.
        let late = [Launch::new(Kind::Boom, Pos::Right, 2000)];
        assert_eq!(match_points(&show[0], &late[0]), 55);
    }

    #[test]
    fn combos_need_runs_of_near_perfect_fireworks() {
        let show: Vec<Launch> = (0..4)
            .map(|i| Launch::new(Kind::from_index(i), Pos::Center, i as i32 * 2000))
            .collect();
        let j = judge(&show, &show, false);
        assert_eq!(j.best_combo, 4);
        // 50 + 100 + 150.
        assert_eq!(j.combo_bonus, 300);
        assert_eq!(j.points, 1300);
        let mut broken = show.clone();
        broken[1].kind = Kind::Sparkle;
        let j = judge(&show, &broken, false);
        assert_eq!(j.best_combo, 2);
        assert_eq!(j.combo_bonus, 50);
    }

    #[test]
    fn a_hint_costs_fifteen_points() {
        let show = vec![Launch::new(Kind::Boom, Pos::Center, 0)];
        assert_eq!(judge(&show, &show, true).accuracy, 85);
    }

    #[test]
    fn the_crowd_follows_the_original_thresholds() {
        assert_eq!(Crowd::for_accuracy(80), Crowd::Cheer);
        assert_eq!(Crowd::for_accuracy(79), Crowd::Ooh);
        assert_eq!(Crowd::for_accuracy(60), Crowd::Ooh);
        assert_eq!(Crowd::for_accuracy(40), Crowd::Aah);
        assert_eq!(Crowd::for_accuracy(39), Crowd::Murmur);
    }

    #[test]
    fn a_round_counts_in_plays_the_show_and_hands_over() {
        let show = vec![
            Launch::new(Kind::Boom, Pos::Center, 0),
            Launch::new(Kind::Whistle, Pos::Center, 1000),
        ];
        let mut r = Round::new(show);
        assert!(r.listen());
        let mut cues = r.update(0.0);
        for _ in 0..200 {
            cues.extend(r.update(0.05));
            if r.phase == Phase::YourTurn {
                break;
            }
        }
        let kinds: Vec<_> = cues
            .iter()
            .map(|c| match c {
                Cue::Tick(n) => format!("tick {n}"),
                Cue::Go => "go".into(),
                Cue::Burst(l) => l.kind.name().into(),
                Cue::TurnBegins => "turn".into(),
            })
            .collect();
        assert_eq!(
            kinds,
            ["tick 1", "tick 2", "tick 3", "go", "boom", "whistle", "turn"]
        );
        assert_eq!(r.phase, Phase::YourTurn);
        assert!(r.launch(Kind::Boom, Pos::Center).is_none());
        assert!(r.play());
        r.update(0.0);
        for _ in 0..30 {
            r.update(0.05);
        }
        let l = r.launch(Kind::Boom, Pos::Center).unwrap();
        assert_eq!(l.ms, 0);
        assert_eq!(r.undo(), Some(l));
        assert!(r.launches.is_empty());
    }

    #[test]
    fn the_composer_starts_its_clock_at_the_first_launch() {
        let mut c = Composer::new();
        c.update(5.0);
        assert_eq!(c.launch(Kind::Boom, Pos::Left).unwrap().ms, 0);
        c.update(1.25);
        assert_eq!(c.launch(Kind::Sparkle, Pos::Right).unwrap().ms, 1250);
        assert_eq!(c.length_ms(), 1250);
    }

    #[test]
    fn hints_read_the_show_in_words() {
        let show = vec![
            Launch::new(Kind::Boom, Pos::Left, 500),
            Launch::new(Kind::Crackle, Pos::Right, 2000),
        ];
        assert_eq!(
            describe_show(&show),
            "2 fireworks: boom on the left, half a second after the go; crackle on the right, 1.5 seconds later."
        );
    }
}
