//! Daily, weekly and monthly challenges: the same rail for every pilot in
//! the period, flown on a standard skimmer.
//!
//! - Daily, the sector sprint: one seeded sector with a twist (dense
//!   traffic, boost offline or fog).
//! - Weekly, the long haul: a seeded three-sector route, its routes fixed,
//!   with a twist of its own, upgrades between sectors and extraction at
//!   the end.
//! - Monthly, the Black Sky event: four sectors escorting evacuation
//!   trains, which pay triple and cost double stress when delayed, with its
//!   own story log.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Parlor, Period};

use crate::profile::Profile;
use crate::rules::{Loadout, Mode, Route, Run, Twist};
use crate::screens::{launch, Launch};
use crate::{cues, ID};

/// The challenges menu's `State::name`, which results and pause return to.
pub const CHALLENGES_NAME: &str = "neon_rail_challenges";

/// The monthly event's own story log, read before the run and kept in the
/// archive once an event is flown to its end.
pub const EVENT_LOG_TITLE: &str = "Blackout Convoy";
pub const EVENT_LOG: &[&str] = &[
    "Black Sky event log. Once a month the orbital grid stands down for ninety seconds, as it did the night the sky went black. The partition gates open, and the evacuation trains run.",
    "Tonight the unknown signal has put your skimmer at the head of the convoy. Every train carries people out of the dark districts under clean transit codes. Every hunter on the rail knows it.",
];

/// How long the daily's one sector runs.
pub const DAILY_LENGTH: f64 = 60.0;

/// Routes a daily can draw: the ones with teeth.
const DAILY_ROUTES: [Route; 3] = [Route::Standard, Route::Ambush, Route::Storm];

/// Routes a weekly can draw.
const ROUTES: [Route; 5] = [
    Route::Standard,
    Route::Ambush,
    Route::Salvage,
    Route::Storm,
    Route::Relay,
];

/// What a challenge is built from.
#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    pub period: Period,
    pub routes: Vec<Route>,
    /// How much harder than a fresh Black Sky Run its first sector is.
    pub depth: usize,
    pub twist: Option<Twist>,
    pub convoy: bool,
    pub loadout: Loadout,
    /// Every sector this long, instead of growing sector by sector.
    pub sector_length: Option<f64>,
}

impl Spec {
    pub fn for_challenge(ch: &Challenge) -> Spec {
        let seed = ch.seed;
        let loadout = Loadout::default();
        match ch.period {
            Period::Daily => Spec {
                period: Period::Daily,
                routes: vec![DAILY_ROUTES[((seed >> 23) % 3) as usize]],
                depth: 4,
                twist: Some(Twist::from_seed(seed)),
                convoy: false,
                loadout,
                sector_length: Some(DAILY_LENGTH),
            },
            Period::Weekly => {
                let first = ((seed >> 17) % 5) as usize;
                let second = (first + 1 + ((seed >> 29) % 4) as usize) % 5;
                Spec {
                    period: Period::Weekly,
                    routes: vec![ROUTES[first], ROUTES[second], Route::Extraction],
                    depth: 4,
                    twist: Some(Twist::from_seed(seed >> 3)),
                    convoy: false,
                    loadout,
                    sector_length: None,
                }
            }
            Period::Monthly => Spec {
                period: Period::Monthly,
                routes: vec![
                    Route::Convoy,
                    Route::Convoy,
                    Route::Convoy,
                    Route::Extraction,
                ],
                depth: 1,
                twist: None,
                convoy: true,
                loadout: Loadout {
                    override_charges: 3,
                    ..Loadout::default()
                },
                sector_length: None,
            },
        }
    }

    /// The challenge's run, laid out from its seed.
    pub fn build(&self, ch: &Challenge, story_stage: usize) -> Run {
        let mut run = Run::challenge_event(
            Mode::from_period(self.period),
            self.loadout.clone(),
            story_stage,
            ch.rng(),
            &[],
            self.depth,
            self.twist,
            self.convoy,
        );
        run.sector_length = self.sector_length;
        run.build_route(&self.routes);
        run.event_rule = self.status_rule();
        run
    }

    /// One line for the status key.
    fn status_rule(&self) -> String {
        match (self.twist, self.convoy) {
            (Some(t), _) => format!("Twist: {}.", t.name()),
            (None, true) => {
                "Convoy escort: evacuation trains pay triple, and a delay costs double stress."
                    .to_string()
            }
            (None, false) => String::new(),
        }
    }
}

/// The challenge's own name.
pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Sector sprint",
        Period::Weekly => "Long haul",
        Period::Monthly => "Black Sky event: Blackout Convoy",
    }
}

fn route_list(routes: &[Route]) -> String {
    let names: Vec<&str> = routes.iter().map(|r| r.name()).collect();
    match names.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{}, then {last}", rest.join(", ")),
        Some((last, _)) => last.to_string(),
        None => String::new(),
    }
}

/// A few words on what the challenge is, for its row in the list.
pub fn tag(ch: &Challenge) -> String {
    let s = Spec::for_challenge(ch);
    let twist = s.twist.map_or("", |t| t.name());
    match ch.period {
        Period::Daily => format!("one {} sector, {twist}", s.routes[0].name()),
        Period::Weekly => format!("{}, {twist}", route_list(&s.routes)),
        Period::Monthly => "four sectors escorting evacuation trains".to_string(),
    }
}

/// What the challenge asks, in a sentence or a few.
pub fn rule(ch: &Challenge) -> String {
    let s = Spec::for_challenge(ch);
    match (ch.period, s.twist) {
        (Period::Daily, Some(t)) => format!(
            "One {}-second {} sector, as hard as a fourth sector. Twist, {}: {}",
            DAILY_LENGTH,
            s.routes[0].name(),
            t.name(),
            t.describe()
        ),
        (Period::Weekly, Some(t)) => format!(
            "Three sectors: {}. You fit an upgrade after each of the first two. Twist, {}: {}",
            route_list(&s.routes),
            t.name(),
            t.describe()
        ),
        _ => "Three convoy sectors, then the extraction sprint. Evacuation trains are priority traffic: each one cleared pays three times, each one delayed costs double stress. Stress starts at 30. You carry three priority overrides.".to_string(),
    }
}

/// The pages read before a challenge run.
pub fn briefing_pages(_ctx: &mut Ctx<Parlor>, ch: &Challenge) -> Vec<String> {
    let mut pages = vec![format!(
        "{}: {} Standard skimmer, no workshop parts or rank bonuses. Pays credits and parts.",
        name(ch.period),
        rule(ch)
    )];
    if ch.period == Period::Monthly {
        pages.extend(EVENT_LOG.iter().map(|p| p.to_string()));
    }
    pages
}

/// Pushes the briefing and then the run of the current `period` challenge.
pub fn start(ctx: &mut Ctx<Parlor>, period: Period, replace: bool) {
    let day = parlor_core::challenges::today(ctx);
    launch(
        ctx,
        Launch {
            mode: Mode::from_period(period),
            seed: None,
            day: Some(day),
        },
        replace,
    );
}

/// The game's side of the shared challenges menu.
pub struct Rails;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Rails>;

impl ChallengeGame for Rails {
    const ID: &'static str = ID;
    const NAME: &'static str = "Neon Rail";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        name(period).to_string()
    }

    fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        tag(&Challenge::current(ctx, ID, period))
    }

    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        rule(&Challenge::current(ctx, ID, period))
    }

    fn open(ctx: &mut Ctx<Parlor>, period: Period) {
        start(ctx, period, false);
    }

    fn music(ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(Some(cues::MUSIC_TITLE), true);
    }

    fn intro_help() -> String {
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back to the title. Only a better score replaces your best, and any finished run keeps your streak.".to_string()
    }
}

/// Stands in for the challenges menu when the launcher opens a challenge directly:
/// it opens the briefing, and once the run is over (or left from the
/// pause menu) it steps aside, back to the launcher.
pub struct ChallengeHost {
    period: Period,
    opened: bool,
}

impl ChallengeHost {
    pub fn new(period: Period) -> ChallengeHost {
        ChallengeHost {
            period,
            opened: false,
        }
    }
}

impl State<Parlor> for ChallengeHost {
    /// Named as the challenges menu is, so results and pause land here.
    fn name(&self) -> &'static str {
        CHALLENGES_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.opened {
            self.opened = true;
            start(ctx, self.period, false);
        }
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.pop();
    }

    fn resume_quiet(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.pop();
    }

    fn resume_with(&mut self, ctx: &mut Ctx<Parlor>, _result: &str) {
        ctx.pop();
    }

    fn help(&self, _ctx: &Ctx<Parlor>) -> String {
        "Opening the Neon Rail challenge.".to_string()
    }
}

/// Whether this pilot has flown a Black Sky event to its end.
pub fn event_log_open(ctx: &mut Ctx<Parlor>) -> bool {
    Profile::load(ctx).black_sky_events > 0
}
