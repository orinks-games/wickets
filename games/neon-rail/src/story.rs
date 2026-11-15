//! The Black Sky City archive: logs recovered across runs. The data core's
//! twelve logs are the spine. The first five are the original game's,
//! recovered the way it recovered them; the rest open as intel fragments
//! build up, one per run at most. Around the spine, `strands` open from
//! the kind of play a pilot does: the routes they clear, the runs they
//! lose, how deep they go on the endless rail, and their own record.
//! Before a run, and after a data relay, the rail's radio speaks a
//! fragment drawn from what the pilot knows so far.

use crate::profile::Profile;
use crate::rules::Route;
pub use crate::strands::STRANDS;

/// One archive log.
pub struct Log {
    pub title: &'static str,
    pub pages: &'static [&'static str],
    /// How it is recovered, for the archive's locked rows.
    pub how: &'static str,
}

pub const LOGS: &[Log] = &[
    Log {
        title: "Stolen Skimmer",
        pages: &["You are a rogue courier-pilot in Black Sky City, running dead transit arteries in a stolen combat skimmer. The city authority wants the craft back, but the skimmer's data core is the real prize."],
        how: "Finish any run.",
    },
    Log {
        title: "The Partition",
        pages: &["Black Sky City was intentionally partitioned after a failed orbital defense event. The rails still move, but the districts no longer agree on who owns the network."],
        how: "Last forty seconds in a run.",
    },
    Log {
        title: "Cargo That Breathes",
        pages: &["The rail network is carrying more than cargo. Priority vehicles broadcast clean transit codes over hidden biological telemetry."],
        how: "Clear three sectors in one run.",
    },
    Log {
        title: "Four Factions",
        pages: &["Transit Authority hunters want the skimmer, salvage unions want the route maps, corporate security wants the courier erased, and an unknown signal keeps guiding you deeper."],
        how: "Complete a run to extraction.",
    },
    Log {
        title: "Black Sky Run",
        pages: &["Every clean run recovers another fragment from the data core, and every fragment proves the collapse was planned."],
        how: "Complete a run to extraction.",
    },
    Log {
        title: "The Salvage Unions",
        pages: &[
            "The unions strip derailed haulers before the Authority can tag them. Their foremen keep route maps in their heads, never on a network.",
            "One of them left a message on your skimmer's hull: the haulers that derail are always the ones that were carrying priority codes.",
        ],
        how: "Recover 3 intel fragments.",
    },
    Log {
        title: "The Orbital Event",
        pages: &[
            "The orbital defense grid did not fail. It was told to stand down for ninety seconds, and in those ninety seconds the sky over the eastern districts went black.",
            "The order came through the transit network, on the same relays your skimmer passes every run.",
        ],
        how: "Recover 6 intel fragments.",
    },
    Log {
        title: "Corporate Security",
        pages: &[
            "Corporate security has no use for the skimmer. Their drones are tuned to destroy the data core.",
            "Whatever is on the core, someone would rather lose it forever than let a courier read it.",
        ],
        how: "Recover 9 intel fragments.",
    },
    Log {
        title: "The Unknown Signal",
        pages: &[
            "The signal that guides you deeper is the network itself, or something living in it. It routes you away from the hunters a moment before they close.",
            "It speaks only in transit codes. Tonight it sent one you have never heard: return to origin.",
        ],
        how: "Recover 12 intel fragments.",
    },
    Log {
        title: "Transit Authority",
        pages: &[
            "The Transit Authority's hunters are former couriers, given a pardon in exchange for the chase. One of them flew this skimmer before you stole it.",
            "Her callsign is still in the cockpit logs. So is her last route, cut off at the partition wall.",
        ],
        how: "Recover 16 intel fragments.",
    },
    Log {
        title: "Breathing Freight",
        pages: &[
            "The biological telemetry comes from passengers, sedated and being moved out of the dark districts under clean codes.",
            "The orbital event was the excuse. The partition decided who would be allowed to leave.",
        ],
        how: "Recover 20 intel fragments.",
    },
    Log {
        title: "Return to Origin",
        pages: &[
            "The data core's last fragment is a schedule: every evacuation train, every district left behind, signed by every faction that now hunts you.",
            "The signal was right. The skimmer was built at the origin depot, and that is where the rails can be opened for everyone. The next run is yours.",
        ],
        how: "Recover 25 intel fragments.",
    },
];

/// The intel needed for each log past the first five. Intel comes one a
/// data relay and one an extraction, so these are run eight or so to
/// run twenty for a pilot who flies the relay.
pub const INTEL_GATES: [u64; 7] = [3, 6, 9, 12, 16, 20, 25];

/// A log in a strand, and what opens it.
pub struct StrandLog {
    pub title: &'static str,
    pub pages: &'static [&'static str],
    pub unlock: Unlock,
}

/// A line of the archive that opens from one kind of play. Its logs open
/// in order, at most one a run.
pub struct Strand {
    /// Saved in the profile as `<id>/<n>` once a log is recovered.
    pub id: &'static str,
    pub name: &'static str,
    /// What the strand is, for the archive's help.
    pub blurb: &'static str,
    pub logs: &'static [StrandLog],
}

/// What a strand log waits for, read off the pilot's record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unlock {
    /// Sectors of this route cleared, lifetime.
    RouteClears(Route, u32),
    /// Runs lost to a hull breach or a network overload.
    Failures(u32),
    /// The sector reached on the endless rail.
    EndlessSector(u32),
    /// Runs flown to a debrief, won or lost.
    RunsFinished(u64),
    /// Rail gun kills, lifetime.
    Kills(u64),
    /// Pilot rank.
    Rank(u32),
    /// Black Sky Runs flown to extraction after the data core was complete.
    OriginRuns(u32),
}

impl Unlock {
    pub fn met(self, p: &Profile) -> bool {
        match self {
            Unlock::RouteClears(route, n) => p.route_clears(route) >= n,
            Unlock::Failures(n) => p.failures >= n,
            Unlock::EndlessSector(n) => p.best_endless_sector >= n,
            Unlock::RunsFinished(n) => p.runs_completed >= n,
            Unlock::Kills(n) => p.kills >= n,
            Unlock::Rank(n) => p.rank() >= n,
            Unlock::OriginRuns(n) => p.origin_runs >= n,
        }
    }

    /// How the log is recovered, for a locked row.
    pub fn how(self) -> String {
        match self {
            // The storm rail is only offered once its charts are bought in
            // the workshop, except in a cold start.
            Unlock::RouteClears(Route::Storm, 1) => {
                "Clear the storm rail once. Needs Storm charts, except in a cold start.".to_string()
            }
            Unlock::RouteClears(route, 1) => {
                format!("Clear the {} once.", route.name().to_lowercase())
            }
            Unlock::RouteClears(route, n) => {
                format!("Clear the {} {n} times.", route.name().to_lowercase())
            }
            Unlock::Failures(1) => "Lose a run.".to_string(),
            Unlock::Failures(n) => format!("Lose {n} runs."),
            Unlock::EndlessSector(n) => {
                format!("Reach endless sector {n}. Quitting from the pause menu counts nothing.")
            }
            Unlock::RunsFinished(n) => format!("Finish {n} runs."),
            Unlock::Kills(n) => format!("Shoot down {n} drones or unstable vehicles."),
            Unlock::Rank(n) => format!("Reach pilot rank {n}."),
            Unlock::OriginRuns(1) => {
                "Recover all 12 data core logs, then reach extraction again.".to_string()
            }
            Unlock::OriginRuns(n) => {
                format!("Recover all 12 data core logs, then reach extraction {n} more times.")
            }
        }
    }
}

/// Every log in the archive, spine and strands, not counting the event log.
pub fn total_logs() -> usize {
    LOGS.len() + STRANDS.iter().map(|s| s.logs.len()).sum::<usize>()
}

/// One radio fragment, and how many data core logs it waits for.
pub struct Radio {
    pub stage: usize,
    pub text: &'static str,
}

/// The story line before the first run, as the original gave it.
pub const FIRST_RUN: &str =
    "Story. You are a rogue courier-pilot in a stolen combat skimmer. Black Sky City is listening.";

/// The radio before a run: the first run's story line, then every second
/// run a fragment the pilot's archive has earned, never the same one twice
/// in a row.
pub fn launch_text(next_run: u64, story_stage: usize) -> Option<String> {
    if next_run <= 1 {
        return Some(FIRST_RUN.to_string());
    }
    if !next_run.is_multiple_of(2) {
        return None;
    }
    radio_fragment(next_run / 2, story_stage).map(str::to_string)
}

/// The `n`th fragment a pilot who knows `story_stage` logs hears.
pub fn radio_fragment(n: u64, story_stage: usize) -> Option<&'static str> {
    let pool: Vec<&'static str> = RADIO
        .iter()
        .filter(|r| r.stage <= story_stage)
        .map(|r| r.text)
        .collect();
    if pool.is_empty() {
        return None;
    }
    Some(pool[(n as usize) % pool.len()])
}

/// The signal's code after a data relay is cleared: the `n`th relay this
/// pilot has cleared picks the line.
pub fn relay_fragment(n: u32) -> &'static str {
    RELAY[n as usize % RELAY.len()]
}

/// The radio before a run: overheard traffic, each line waiting for the
/// data core logs that make sense of it.
pub const RADIO: &[Radio] = &[
    Radio {
        stage: 0,
        text: "Hunt channel: stolen skimmer sighted on the dead artery. All units, close the lane.",
    },
    Radio {
        stage: 0,
        text: "Hunt channel: the craft is Authority property. Recover it intact, whatever the pilot does.",
    },
    Radio {
        stage: 0,
        text: "Drone dispatch: 3 units to the dead artery. Report any skimmer traffic.",
    },
    Radio {
        stage: 0,
        text: "Weather: rain on the upper rails until dawn. Metal will be slick.",
    },
    Radio {
        stage: 0,
        text: "Station announcement: this platform is closed. Trains do not stop here anymore.",
    },
    Radio {
        stage: 1,
        text: "Hunt channel: the core is the priority. Nobody fires on the hull tonight.",
    },
    Radio {
        stage: 2,
        text: "Station announcement: partition gates sealed. Which district owns this platform is still disputed.",
    },
    Radio {
        stage: 2,
        text: "Weather: dust front crossing the partition wall. Visibility is nothing past the gates.",
    },
    Radio {
        stage: 3,
        text: "Union chatter: that priority hauler reads clean, but something under the code is breathing.",
    },
    Radio {
        stage: 4,
        text: "Union chatter: that skimmer is carrying routes we want. Nobody touches the courier.",
    },
    Radio {
        stage: 4,
        text: "Drone dispatch: target is the data core. Leave the craft flying. Destroy and confirm.",
    },
    Radio {
        stage: 4,
        text: "The signal: Clear lane. Hunters rerouted.",
    },
    Radio {
        stage: 5,
        text: "Hunt channel: every fragment the courier recovers is a problem upstairs. Close faster.",
    },
    Radio {
        stage: 6,
        text: "Union chatter: hauler down on the lower rail. Strip it before the Authority tags it.",
    },
    Radio {
        stage: 6,
        text: "Foreman Hale: that hauler was running priority codes. They always are when they derail.",
    },
    Radio {
        stage: 7,
        text: "Hunt channel: 90 seconds is not a failure. Strike that from the band.",
    },
    Radio {
        stage: 7,
        text: "Weather: clear over the eastern districts. No lights down there to see it by.",
    },
    Radio {
        stage: 8,
        text: "Operator Nine, sortie 22: tune for the core. Hull damage is no concern.",
    },
    Radio {
        stage: 9,
        text: "The signal: Return to origin. Hunters closing in 40 seconds.",
    },
    Radio {
        stage: 10,
        text: "Hunt channel: Sable, you flew that skimmer. Tell us how the courier will turn.",
    },
    Radio {
        stage: 10,
        text: "Hunt channel: half this band were couriers once. Remember that before you fire.",
    },
    Radio {
        stage: 11,
        text: "Station announcement: evacuation train boarding under clean codes. Passengers will not need to wake.",
    },
    Radio {
        stage: 12,
        text: "Union chatter: every faction signed that schedule. Hale says the maps stay in our heads.",
    },
    Radio {
        stage: 12,
        text: "Weather: storm over the origin depot. No skimmer has gone back there since the partition.",
    },
];

/// The signal's codes after a data relay.
pub const RELAY: &[&str] = &[
    "The signal: Clear lane. Hunt channel is reading an empty rail.",
    "The signal: Hold at relay. 2 drones pass in 20 seconds.",
    "The signal: Lane change logged. Authority holds old route.",
    "The signal: Priority traffic ahead. Give it the rail.",
    "The signal: Relay quiet. Band closed to Authority.",
    "The signal: Route deeper. The hunters are holding at the wall.",
    "The signal: Code accepted. Clean traffic to next relay.",
    "The signal: Wet rail, clear. Drones grounded until rain ends.",
];
