//! The dance as plain rules, with no screens or sound, so tests can drive a
//! street beat by beat.
//!
//! A street is a run of bars, four drum beats each. Its plan says what
//! happens on which beat: the pearl moving to another lane, a gong to rear
//! up on, a red envelope or a lantern coming down a lane, a string of
//! firecrackers in a lane, a shop and its crowd. [`Dance::update`] runs the
//! clock in fixed substeps and turns each beat into [`Event`]s; the player
//! moves the head with [`Dance::step`] and rears with [`Dance::rear`].

use std::cell::Cell;

use parlor_core::{FixedStep, Rng};

/// Drum beats counted in before the first bar.
pub const LEAD_IN: i64 = 8;
/// Beats of warning before an envelope, lantern or firecracker string arrives.
pub const WARN: i64 = 4;
/// Beats of drum roll before a gong.
pub const ROLL: i64 = 2;
/// A rear this close to the gong, in seconds, is perfect.
pub const PERFECT_S: f64 = 0.1;
/// A rear this close to the gong, in seconds, still counts.
pub const GOOD_S: f64 = 0.22;
/// The dragon's segments as a street begins.
pub const START_SEGMENTS: u32 = 6;
/// The longest the dragon grows.
pub const MAX_SEGMENTS: u32 = 40;
/// Beats followed in a row for a combo bonus.
pub const COMBO_EVERY: u32 = 16;
pub const COMBO_BONUS: u64 = 50;
pub const GONG_PERFECT: u64 = 100;
pub const GONG_GOOD: u64 = 60;
pub const ENVELOPE_POINTS: u64 = 40;
pub const LANTERN_POINTS: u64 = 30;
pub const SHOP_POINTS: u64 = 20;
/// The combo a shop's crowd needs to cheer.
pub const CHEER_COMBO: u32 = 4;
pub const STUMBLE_POINTS: u64 = 50;
/// Beats off the pearl in a row that break the combo.
pub const DRIFT_BEATS: u32 = 2;
/// Beats after the street's last before it ends.
pub const TAIL_BEATS: i64 = 3;
/// Beats counted down to midnight.
pub const COUNTDOWN: i64 = 5;
/// Substeps a second.
pub const STEP_HZ: f64 = 240.0;

thread_local! {
    static FIXED_SEED: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Pins the seed of every new street, for tests; `None` uses the clock.
pub fn fix_seed(seed: Option<u64>) {
    FIXED_SEED.with(|s| s.set(seed));
}

/// A seed for a new street.
pub fn run_seed() -> u64 {
    FIXED_SEED
        .with(|s| s.get())
        .unwrap_or_else(|| Rng::from_time().next_u64())
}

/// How a street ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finale {
    /// A last gong on the final bar.
    Gong,
    /// A countdown to midnight, a great gong and the firecrackers.
    Midnight,
    /// The walk simply ends.
    Calm,
}

/// One street's shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Street {
    pub name: &'static str,
    pub lanes: usize,
    pub bpm: f64,
    pub bars: u32,
    /// Bars between gongs, 0 for none.
    pub gong_every: u32,
    /// The chance, each bar, that the pearl moves.
    pub move_chance: f64,
    /// The chance, each bar, of a red envelope (a lantern on a calm walk).
    pub envelope_chance: f64,
    /// The chance, each bar, of a string of firecrackers.
    pub firecracker_chance: f64,
    /// The pearl may jump two lanes at once.
    pub jumps: bool,
    /// Firecrackers may wait in the lane the pearl has just left.
    pub tight: bool,
    pub finale: Finale,
    pub shops: &'static [&'static str],
}

impl Street {
    /// Seconds between beats.
    pub fn beat_len(&self) -> f64 {
        60.0 / self.bpm
    }

    /// Beats in the street, not counting the lead-in or the tail.
    pub fn beats(&self) -> i64 {
        self.bars as i64 * 4
    }

    pub fn calm(&self) -> bool {
        self.finale == Finale::Calm
    }

    /// "4 lanes, 32 bars, firecrackers, the pearl jumps".
    pub fn describe(&self) -> String {
        let mut parts = vec![
            format!("{} lanes", self.lanes),
            format!("{} bars", self.bars),
        ];
        if self.calm() {
            parts.push("lanterns and no firecrackers".to_string());
        } else if self.firecracker_chance > 0.0 {
            parts.push("firecrackers".to_string());
        } else {
            parts.push("no firecrackers".to_string());
        }
        if self.jumps {
            parts.push("the pearl can jump 2 lanes".to_string());
        }
        if self.tight {
            parts.push("firecrackers close behind the pearl".to_string());
        }
        parts.join(", ")
    }

    /// Roughly how long the street lasts, in whole seconds.
    pub fn seconds(&self) -> u32 {
        ((self.beats() + LEAD_IN + TAIL_BEATS) as f64 * self.beat_len()).round() as u32
    }
}

/// A side of the street.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub fn words(self) -> &'static str {
        match self {
            Side::Left => "on your left",
            Side::Right => "on your right",
        }
    }
}

/// What happens on a beat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// The pearl moves to `lane`.
    Pearl {
        lane: usize,
    },
    /// A gong strikes; `great` is midnight's, worth double.
    Gong {
        great: bool,
    },
    Envelope {
        lane: usize,
    },
    Lantern {
        lane: usize,
        kind: usize,
    },
    Firecracker {
        lane: usize,
    },
    Shop {
        side: Side,
        name: &'static str,
    },
    /// Midnight's countdown: `n` beats to go.
    Countdown {
        n: i64,
    },
}

/// A mark on its beat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Planned {
    pub beat: i64,
    pub mark: Mark,
}

/// Something coming down a lane, warned ahead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Thing {
    Envelope,
    Lantern,
    Firecracker,
}

impl Thing {
    pub fn name(self) -> &'static str {
        match self {
            Thing::Envelope => "red envelope",
            Thing::Lantern => "lantern",
            Thing::Firecracker => "firecrackers",
        }
    }
}

/// How a rear met its gong.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade {
    Perfect,
    Good,
}

/// What a beat or a key did.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A drum beat of the lead-in, `left` to go.
    CountIn {
        left: i64,
    },
    /// The first beat of the street.
    Start,
    /// A drum beat; `down` is the first of a bar.
    Beat {
        beat: i64,
        down: bool,
    },
    PearlMoved {
        from: usize,
        to: usize,
    },
    /// Something is coming down `lane`, `WARN` beats away.
    Warn {
        id: usize,
        thing: Thing,
        lane: usize,
    },
    /// A gong is `ROLL` beats away.
    Roll {
        great: bool,
    },
    Gong {
        great: bool,
    },
    Rear {
        grade: Grade,
        points: u64,
        great: bool,
    },
    GongMissed {
        great: bool,
    },
    /// A rear with no gong near; `broke` when it broke a combo.
    Stray {
        broke: bool,
    },
    Envelope {
        lane: usize,
        points: u64,
        segments: u32,
    },
    EnvelopeMissed {
        lane: usize,
    },
    Lantern {
        lane: usize,
        kind: usize,
        points: u64,
    },
    LanternMissed {
        lane: usize,
        kind: usize,
    },
    /// The head met firecrackers in `lane`: the tail stumbles.
    Stumble {
        lane: usize,
        lost: u64,
    },
    /// Firecrackers burst in another lane.
    Crackled {
        lane: usize,
    },
    Shop {
        side: Side,
        name: &'static str,
        cheer: bool,
        points: u64,
    },
    Combo {
        combo: u32,
        bonus: u64,
    },
    /// Off the pearl long enough to break a combo of `combo`.
    LostPearl {
        combo: u32,
    },
    Countdown {
        n: i64,
    },
    Midnight,
    Finished,
}

/// What a step did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Moved {
        lane: usize,
    },
    /// Already at the edge of the street on that side.
    Edge {
        left: bool,
    },
}

/// Something warned and still coming.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Incoming {
    pub id: usize,
    pub thing: Thing,
    pub lane: usize,
    /// Beats until it arrives.
    pub beats: f64,
}

/// A street's plan: its marks, beat by beat, and where the pearl starts.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub marks: Vec<Planned>,
    pub start: usize,
    pub beats: i64,
}

impl Plan {
    /// The pearl's lane on `beat`.
    pub fn pearl_at(&self, beat: i64) -> usize {
        self.marks
            .iter()
            .filter(|p| p.beat <= beat)
            .filter_map(|p| match p.mark {
                Mark::Pearl { lane } => Some(lane),
                _ => None,
            })
            .next_back()
            .unwrap_or(self.start)
    }

    pub fn count(&self, f: impl Fn(&Mark) -> bool) -> usize {
        self.marks.iter().filter(|p| f(&p.mark)).count()
    }
}

/// Makes a street's plan from a seed.
pub fn plan(street: &Street, seed: u64) -> Plan {
    let mut rng = Rng::seeded(seed);
    let lanes = street.lanes.max(1);
    let bars = street.bars.max(2) as i64;
    let beats = bars * 4;
    let start = lanes / 2;
    let mut marks = Vec::new();
    let mut pearl = start;
    let mut pearl_at = vec![start; beats as usize];
    let mut moved = vec![false; bars as usize];
    let last_bar = bars - 1;
    // The pearl moves at the start of a bar, never in the first two or the last.
    for bar in 0..bars {
        let b0 = bar * 4;
        if bar >= 2 && bar < last_bar && rng.chance(street.move_chance) && lanes > 1 {
            let jump = if street.jumps && lanes >= 4 && rng.chance(0.3) {
                2
            } else {
                1
            };
            let up = if pearl == 0 {
                true
            } else if pearl + 1 == lanes {
                false
            } else {
                rng.chance(0.5)
            };
            let to = if up {
                (pearl + jump).min(lanes - 1)
            } else {
                pearl.saturating_sub(jump)
            };
            if to != pearl {
                marks.push(Planned {
                    beat: b0,
                    mark: Mark::Pearl { lane: to },
                });
                pearl = to;
                moved[bar as usize] = true;
            }
        }
        for i in 0..4 {
            pearl_at[(b0 + i) as usize] = pearl;
        }
    }
    // Gongs on the third beat of every few bars.
    let mut gong_beats = Vec::new();
    if street.gong_every > 0 {
        let mut bar = street.gong_every as i64;
        while bar < last_bar {
            gong_beats.push(bar * 4 + 2);
            bar += street.gong_every as i64;
        }
    }
    for &beat in &gong_beats {
        marks.push(Planned {
            beat,
            mark: Mark::Gong { great: false },
        });
    }
    match street.finale {
        Finale::Gong => marks.push(Planned {
            beat: last_bar * 4,
            mark: Mark::Gong { great: false },
        }),
        Finale::Midnight => {
            for n in (1..=COUNTDOWN).rev() {
                marks.push(Planned {
                    beat: beats - n,
                    mark: Mark::Countdown { n },
                });
            }
            marks.push(Planned {
                beat: beats,
                mark: Mark::Gong { great: true },
            });
        }
        Finale::Calm => {}
    }
    // Shops every other bar, on its second beat, from the shuffled list.
    let mut shops: Vec<&'static str> = street.shops.to_vec();
    rng.shuffle(&mut shops);
    let mut shop = 0;
    let mut bar = 1;
    while bar < bars && !shops.is_empty() {
        let side = if rng.chance(0.5) {
            Side::Left
        } else {
            Side::Right
        };
        marks.push(Planned {
            beat: bar * 4 + 1,
            mark: Mark::Shop {
                side,
                name: shops[shop % shops.len()],
            },
        });
        shop += 1;
        bar += 2;
    }
    // Envelopes, or lanterns on a calm walk, on a bar's last beat.
    let mut kinds: Vec<usize> = (0..crate::festival::LANTERNS.len()).collect();
    rng.shuffle(&mut kinds);
    let mut kind = 0;
    let mut envelope_lane = vec![None; bars as usize];
    for bar in 2..last_bar {
        if !rng.chance(street.envelope_chance) {
            continue;
        }
        let beat = bar * 4 + 3;
        let on = pearl_at[beat as usize];
        let lane = if rng.chance(0.45) || lanes == 1 {
            on
        } else {
            neighbour(&mut rng, on, lanes)
        };
        envelope_lane[bar as usize] = Some(lane);
        let mark = if street.calm() {
            kind += 1;
            Mark::Lantern {
                lane,
                kind: kinds[(kind - 1) % kinds.len()],
            }
        } else {
            Mark::Envelope { lane }
        };
        marks.push(Planned { beat, mark });
    }
    // Firecrackers, never in the pearl's lane.
    if !street.calm() && lanes > 1 {
        for bar in 3..last_bar {
            if !rng.chance(street.firecracker_chance) {
                continue;
            }
            let b0 = bar * 4;
            if moved[bar as usize] && !street.tight {
                continue;
            }
            let beat = if gong_beats.contains(&(b0 + 2)) || rng.chance(0.5) {
                b0 + 1
            } else {
                b0 + 2
            };
            let on = pearl_at[beat as usize];
            let left_behind = if moved[bar as usize] {
                Some(pearl_at[(b0 - 1) as usize])
            } else {
                None
            };
            let lane = match left_behind {
                Some(lane) if lane != on && rng.chance(0.7) => lane,
                _ => neighbour(&mut rng, on, lanes),
            };
            marks.push(Planned {
                beat,
                mark: Mark::Firecracker { lane },
            });
        }
    }
    marks.sort_by_key(|p| p.beat);
    Plan {
        marks,
        start,
        beats,
    }
}

/// A lane next to `lane`, picked at random when there are two.
fn neighbour(rng: &mut Rng, lane: usize, lanes: usize) -> usize {
    if lane == 0 {
        1.min(lanes - 1)
    } else if lane + 1 >= lanes || rng.chance(0.5) {
        lane - 1
    } else {
        lane + 1
    }
}

/// Stars for a score out of the street's best: 0 to 3.
pub fn stars(score: u64, best: u64) -> u8 {
    if best == 0 {
        return 0;
    }
    let percent = score * 100 / best;
    match percent {
        80.. => 3,
        60.. => 2,
        35.. => 1,
        _ => 0,
    }
}

/// The percent of the best score each star needs, first star first.
pub const STAR_PERCENT: [u64; 3] = [35, 60, 80];

/// "here", "1 lane to your left", "2 lanes to your right".
pub fn relative(from: usize, to: usize) -> String {
    match to.cmp(&from) {
        std::cmp::Ordering::Equal => "here".to_string(),
        std::cmp::Ordering::Less => lanes_words(from - to, "left"),
        std::cmp::Ordering::Greater => lanes_words(to - from, "right"),
    }
}

fn lanes_words(n: usize, side: &str) -> String {
    match n {
        1 => format!("1 lane to your {side}"),
        n => format!("{n} lanes to your {side}"),
    }
}

/// "1 beat" or "3 beats".
pub fn beats_words(n: i64) -> String {
    match n {
        1 => "1 beat".to_string(),
        n => format!("{n} beats"),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct GongState {
    beat: i64,
    great: bool,
    done: bool,
}

/// A street being danced.
#[derive(Clone, Debug)]
pub struct Dance {
    pub street: Street,
    pub plan: Plan,
    /// The head's lane, from 0 on the left.
    pub lane: usize,
    pub pearl: usize,
    pub score: u64,
    pub combo: u32,
    pub best_combo: u32,
    pub segments: u32,
    pub envelopes: u32,
    /// The lanterns caught on a calm walk, by number.
    pub lanterns: Vec<usize>,
    pub stumbles: u32,
    pub perfect: u32,
    pub good: u32,
    pub gongs: u32,
    pub follow_beats: u32,
    pub beats_played: u32,
    /// The run-up to New Year: envelopes count double.
    pub run_up: bool,
    /// Practice: stumbles cost nothing.
    pub forgiving: bool,
    pub clock: f64,
    next: i64,
    off_beats: u32,
    gong_states: Vec<GongState>,
    step: FixedStep,
    events: Vec<Event>,
    pub finished: bool,
}

impl Dance {
    pub fn new(street: Street, seed: u64) -> Dance {
        let plan = plan(&street, seed);
        Dance::with_plan(street, plan)
    }

    /// A dance on a plan a test wrote.
    pub fn with_plan(street: Street, plan: Plan) -> Dance {
        let gong_states = plan
            .marks
            .iter()
            .filter_map(|p| match p.mark {
                Mark::Gong { great } => Some(GongState {
                    beat: p.beat,
                    great,
                    done: false,
                }),
                _ => None,
            })
            .collect();
        Dance {
            street,
            lane: plan.start,
            pearl: plan.start,
            plan,
            score: 0,
            combo: 0,
            best_combo: 0,
            segments: START_SEGMENTS,
            envelopes: 0,
            lanterns: Vec::new(),
            stumbles: 0,
            perfect: 0,
            good: 0,
            gongs: 0,
            follow_beats: 0,
            beats_played: 0,
            run_up: false,
            forgiving: false,
            clock: 0.0,
            next: -LEAD_IN,
            off_beats: 0,
            gong_states,
            step: FixedStep::hz(STEP_HZ),
            events: Vec::new(),
            finished: false,
        }
    }

    pub fn lanes(&self) -> usize {
        self.street.lanes
    }

    /// The time of `beat` from the start of the lead-in, in seconds.
    pub fn time_of(&self, beat: i64) -> f64 {
        (beat + LEAD_IN) as f64 * self.street.beat_len()
    }

    /// The beat played last: below 0 in the lead-in.
    pub fn beat(&self) -> i64 {
        self.next - 1
    }

    /// Still counting in.
    pub fn counting_in(&self) -> bool {
        self.next <= 0
    }

    /// The bar the dance is in, from 1.
    pub fn bar(&self) -> u32 {
        (self.beat().max(0) / 4 + 1).min(self.street.bars as i64) as u32
    }

    /// The beat after which the street ends.
    pub fn end_beat(&self) -> i64 {
        let last = self.plan.marks.iter().map(|p| p.beat).max().unwrap_or(0);
        self.plan.beats.max(last + 1) + TAIL_BEATS
    }

    pub fn envelope_points(&self) -> u64 {
        if self.run_up {
            ENVELOPE_POINTS * 2
        } else {
            ENVELOPE_POINTS
        }
    }

    /// The most a perfect dance could score here: every beat followed,
    /// every envelope and lantern caught, every gong perfect.
    pub fn best_score(&self) -> u64 {
        let mut segments = START_SEGMENTS;
        let mut total = 0u64;
        for beat in 0..self.plan.beats {
            for p in self.plan.marks.iter().filter(|p| p.beat == beat) {
                if let Mark::Envelope { .. } = p.mark {
                    segments = (segments + 1).min(MAX_SEGMENTS);
                }
            }
            total += segments as u64;
        }
        total += (self.plan.beats as u64 / COMBO_EVERY as u64) * COMBO_BONUS;
        for p in &self.plan.marks {
            total += match p.mark {
                Mark::Gong { great } => GONG_PERFECT * if great { 2 } else { 1 },
                Mark::Envelope { .. } => self.envelope_points(),
                Mark::Lantern { .. } => LANTERN_POINTS,
                Mark::Shop { .. } => SHOP_POINTS,
                _ => 0,
            };
        }
        total
    }

    pub fn stars(&self) -> u8 {
        stars(self.score, self.best_score())
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Runs the clock for `dt` seconds, in fixed substeps.
    pub fn update(&mut self, dt: f64) {
        let mut step = self.step;
        step.advance(dt, |s| self.substep(s));
        self.step = step;
    }

    fn substep(&mut self, s: f64) {
        if self.finished {
            return;
        }
        self.clock += s;
        while !self.finished && self.clock + 1e-9 >= self.time_of(self.next) {
            let beat = self.next;
            self.next += 1;
            self.play_beat(beat);
        }
        let clock = self.clock;
        let beat_len = self.street.beat_len();
        let mut missed = Vec::new();
        for g in &mut self.gong_states {
            if !g.done && clock > (g.beat + LEAD_IN) as f64 * beat_len + GOOD_S {
                g.done = true;
                missed.push(g.great);
            }
        }
        for great in missed {
            self.gongs += 1;
            self.events.push(Event::GongMissed { great });
        }
    }

    fn play_beat(&mut self, k: i64) {
        if k < 0 {
            self.events.push(Event::CountIn { left: -k });
            return;
        }
        let end = self.end_beat();
        if k >= end {
            self.finished = true;
            self.events.push(Event::Finished);
            return;
        }
        if k == 0 {
            self.events.push(Event::Start);
        }
        self.events.push(Event::Beat {
            beat: k,
            down: k % 4 == 0,
        });
        let marks: Vec<(usize, Planned)> = self
            .plan
            .marks
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, p)| p.beat == k + WARN || p.beat == k + ROLL || p.beat == k)
            .collect();
        for (id, p) in &marks {
            let id = *id;
            if p.beat == k + WARN {
                let warn = match p.mark {
                    Mark::Envelope { lane } => Some((Thing::Envelope, lane)),
                    Mark::Lantern { lane, .. } => Some((Thing::Lantern, lane)),
                    Mark::Firecracker { lane } => Some((Thing::Firecracker, lane)),
                    _ => None,
                };
                if let Some((thing, lane)) = warn {
                    self.events.push(Event::Warn { id, thing, lane });
                }
            }
            if p.beat == k + ROLL {
                if let Mark::Gong { great } = p.mark {
                    self.events.push(Event::Roll { great });
                }
            }
        }
        let mut moved = false;
        for (_, p) in marks.iter().filter(|(_, p)| p.beat == k) {
            match p.mark {
                Mark::Pearl { lane } => {
                    let from = self.pearl;
                    self.pearl = lane;
                    moved = true;
                    self.events.push(Event::PearlMoved { from, to: lane });
                }
                Mark::Gong { great } => self.events.push(Event::Gong { great }),
                Mark::Envelope { lane } => {
                    if self.lane == lane {
                        self.envelopes += 1;
                        self.segments = (self.segments + 1).min(MAX_SEGMENTS);
                        let points = self.envelope_points();
                        self.score += points;
                        self.events.push(Event::Envelope {
                            lane,
                            points,
                            segments: self.segments,
                        });
                    } else {
                        self.events.push(Event::EnvelopeMissed { lane });
                    }
                }
                Mark::Lantern { lane, kind } => {
                    if self.lane == lane {
                        self.lanterns.push(kind);
                        self.score += LANTERN_POINTS;
                        self.events.push(Event::Lantern {
                            lane,
                            kind,
                            points: LANTERN_POINTS,
                        });
                    } else {
                        self.events.push(Event::LanternMissed { lane, kind });
                    }
                }
                Mark::Firecracker { lane } => {
                    if self.lane == lane {
                        self.stumbles += 1;
                        let lost = if self.forgiving {
                            0
                        } else {
                            STUMBLE_POINTS.min(self.score)
                        };
                        self.score -= lost;
                        self.combo = 0;
                        self.events.push(Event::Stumble { lane, lost });
                    } else {
                        self.events.push(Event::Crackled { lane });
                    }
                }
                Mark::Shop { side, name } => {
                    let cheer = self.combo >= CHEER_COMBO;
                    let points = if cheer { SHOP_POINTS } else { 0 };
                    self.score += points;
                    self.events.push(Event::Shop {
                        side,
                        name,
                        cheer,
                        points,
                    });
                }
                Mark::Countdown { n } => self.events.push(Event::Countdown { n }),
            }
            if p.mark == (Mark::Gong { great: true }) {
                self.events.push(Event::Midnight);
            }
        }
        if k < self.plan.beats {
            self.follow(moved);
        }
    }

    /// The beat's follow check: on the pearl, the dragon dances and
    /// scores; off it for long, the combo breaks. The beat the pearl
    /// moves is a grace beat.
    fn follow(&mut self, moved: bool) {
        self.beats_played += 1;
        if self.lane == self.pearl {
            self.off_beats = 0;
            self.follow_beats += 1;
            self.combo += 1;
            self.best_combo = self.best_combo.max(self.combo);
            self.score += self.segments as u64;
            if self.combo.is_multiple_of(COMBO_EVERY) {
                self.score += COMBO_BONUS;
                self.events.push(Event::Combo {
                    combo: self.combo,
                    bonus: COMBO_BONUS,
                });
            }
        } else if !moved {
            self.off_beats += 1;
            if self.off_beats >= DRIFT_BEATS && self.combo > 0 {
                let combo = self.combo;
                self.combo = 0;
                self.events.push(Event::LostPearl { combo });
            }
        }
    }

    /// Moves the head one lane left (`delta` below 0) or right.
    pub fn step(&mut self, delta: i32) -> Move {
        let left = delta < 0;
        if (left && self.lane == 0) || (!left && self.lane + 1 >= self.lanes()) {
            return Move::Edge { left };
        }
        self.lane = if left { self.lane - 1 } else { self.lane + 1 };
        Move::Moved { lane: self.lane }
    }

    /// Rears the dragon up. Near a gong it scores; with no gong near it
    /// breaks the combo. The result is among the events.
    pub fn rear(&mut self) {
        if self.finished {
            return;
        }
        let clock = self.clock;
        let beat_len = self.street.beat_len();
        let nearest = self
            .gong_states
            .iter_mut()
            .filter(|g| !g.done)
            .map(|g| {
                let off = clock - (g.beat + LEAD_IN) as f64 * beat_len;
                (off.abs(), g)
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        match nearest {
            Some((off, g)) if off <= GOOD_S => {
                g.done = true;
                let great = g.great;
                let (grade, base) = if off <= PERFECT_S {
                    (Grade::Perfect, GONG_PERFECT)
                } else {
                    (Grade::Good, GONG_GOOD)
                };
                let points = base * if great { 2 } else { 1 };
                self.gongs += 1;
                match grade {
                    Grade::Perfect => self.perfect += 1,
                    Grade::Good => self.good += 1,
                }
                self.score += points;
                self.events.push(Event::Rear {
                    grade,
                    points,
                    great,
                });
            }
            _ => {
                let broke = self.combo > 0;
                self.combo = 0;
                self.events.push(Event::Stray { broke });
            }
        }
    }

    /// What is warned and still coming, nearest first.
    pub fn incoming(&self) -> Vec<Incoming> {
        let now = self.beat();
        let beat_len = self.street.beat_len();
        let mut out: Vec<Incoming> = self
            .plan
            .marks
            .iter()
            .enumerate()
            .filter(|(_, p)| p.beat > now && p.beat <= now + WARN)
            .filter_map(|(id, p)| {
                let (thing, lane) = match p.mark {
                    Mark::Envelope { lane } => (Thing::Envelope, lane),
                    Mark::Lantern { lane, .. } => (Thing::Lantern, lane),
                    Mark::Firecracker { lane } => (Thing::Firecracker, lane),
                    _ => return None,
                };
                let beats = ((self.time_of(p.beat) - self.clock) / beat_len).max(0.0);
                Some(Incoming {
                    id,
                    thing,
                    lane,
                    beats,
                })
            })
            .collect();
        out.sort_by(|a, b| a.beats.total_cmp(&b.beats));
        out
    }

    /// Whole beats until the next gong still to rear on, if one is near.
    pub fn next_gong(&self) -> Option<i64> {
        let now = self.beat();
        self.gong_states
            .iter()
            .filter(|g| !g.done && g.beat > now)
            .map(|g| g.beat - now)
            .min()
    }

    /// Whole beats until the pearl next moves, if it is due within a bar.
    pub fn pearl_moves_in(&self) -> Option<i64> {
        let now = self.beat();
        self.plan
            .marks
            .iter()
            .filter(|p| matches!(p.mark, Mark::Pearl { .. }) && p.beat > now && p.beat <= now + 4)
            .map(|p| p.beat - now)
            .min()
    }

    /// "on the pearl" or "pearl in lane 3, 1 lane to your right".
    pub fn pearl_words(&self) -> String {
        if self.lane == self.pearl {
            "on the pearl".to_string()
        } else {
            format!(
                "pearl in lane {}, {}",
                self.pearl + 1,
                relative(self.lane, self.pearl)
            )
        }
    }

    /// "firecrackers in lane 1 in 2 beats".
    pub fn incoming_words(item: &Incoming) -> String {
        let beats = (item.beats.ceil() as i64).max(1);
        format!(
            "{} in lane {} in {}",
            item.thing.name(),
            item.lane + 1,
            beats_words(beats)
        )
    }

    /// The status line, in a fixed order: lane, pearl, what is coming,
    /// the next gong, score, the dragon, the bar.
    /// The status key, in layers. First: your lane, the pearl, what is
    /// coming and the next gong. A second press: score, length, combo and
    /// the bar.
    pub fn status_layers(&self) -> Vec<String> {
        let mut first = vec![
            format!("Lane {} of {}", self.lane + 1, self.lanes()),
            self.pearl_words(),
        ];
        for item in self.incoming() {
            first.push(Dance::incoming_words(&item));
        }
        if let Some(n) = self.next_gong().filter(|&n| n <= 8) {
            first.push(format!("gong in {}", beats_words(n)));
        }
        let mut more = vec![
            format!("{} points", self.score),
            format!("dragon {} long", self.segments),
            format!("combo {}", self.combo),
        ];
        if self.counting_in() {
            more.push("counting in".to_string());
        } else {
            more.push(format!("bar {} of {}", self.bar(), self.street.bars));
        }
        vec![
            format!("{}.", first.join(", ")),
            format!("{}.", hearken::text::sentence_start(&more.join(", "))),
        ]
    }

    /// Every layer of the status key at once.
    pub fn status(&self) -> String {
        self.status_layers().join(" ")
    }
}
