//! The colony phase: between nights the stores are spent, works go on the
//! grid and buildings into the slots, then Ready sends the night down.
//!
//! Three screens share one run: the colony menu, the slot picker for a
//! building, and the placer that walks the grid for a work.

use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::text::{end_sentence, sentence_start};
use hearken::{Bus, Ctx, InputEvent, Key, State};
use parlor_core::menus::{self, labels, Leave};
use parlor_core::{Parlor, ParlorAction, Period};

use crate::challenge::CHALLENGES_NAME;
use crate::music::Conductor;
use crate::night::NightScreen;
use crate::progress::Progress;
use crate::rules::grid::{column_name, COLS, WORK_ROWS};
use crate::rules::{colony, Building, Event, Input, Refusal, Run, Sector, Work, WorkKind};
use crate::sound;
use crate::{HELP, TITLE_NAME};

pub const COLONY_NAME: &str = "dustfall_colony";
pub const SLOTS_NAME: &str = "dustfall_slots";
pub const PLACER_NAME: &str = "dustfall_placer";

/// What kind of watch a run is, which decides what is saved and where
/// leaving goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// A colony of your own: saved between nights, left for the title.
    Colony,
    /// The tutorial's night: nothing saved, nothing left for.
    Practice,
    /// A challenge: scored on its board, left for the challenges menu.
    Challenge(Period),
}

impl Mode {
    /// Where the pause menu's leave row goes.
    pub fn leave(self) -> Leave {
        match self {
            Mode::Challenge(_) => Leave::Challenge(CHALLENGES_NAME),
            _ => Leave::Title(TITLE_NAME),
        }
    }
}

/// A run and the kind of watch it is. It reads as the run itself.
#[derive(Clone, Debug)]
pub struct Watch {
    pub run: Run,
    pub mode: Mode,
    /// The colony phase's bed, kept here so the slot picker and the placer
    /// carry it on.
    pub bed: Conductor,
}

impl Watch {
    /// Keeps the bed going, the bass alone while the colony is hungry.
    pub fn play_bed(&mut self, audio: &mut hearken::Audio, dt: f64) {
        let hungry = self.run.colony.hungry();
        self.bed.set_hungry(hungry);
        self.bed.update(audio, dt);
    }
}

impl Watch {
    /// Saves the run as the colony to continue, when it is one.
    pub fn keep(&self, ctx: &mut Ctx<Parlor>) {
        if self.mode == Mode::Colony {
            Progress::keep(ctx, &self.run);
        }
    }
}

impl Deref for Watch {
    type Target = Run;

    fn deref(&self) -> &Run {
        &self.run
    }
}

impl DerefMut for Watch {
    fn deref_mut(&mut self) -> &mut Run {
        &mut self.run
    }
}

/// The run, shared by the screens of one watch.
pub type Shared = Rc<RefCell<Watch>>;

/// A colony of your own.
pub fn share(run: Run) -> Shared {
    share_as(run, Mode::Colony)
}

pub fn share_as(run: Run, mode: Mode) -> Shared {
    let bed = Conductor::colony(run.seed() ^ 0x00c0_104e);
    Rc::new(RefCell::new(Watch { run, mode, bed }))
}

/// "8 metal", "6 metal and 2 power".
pub fn cost_words(metal: u32, power: u32) -> String {
    match (metal, power) {
        (m, 0) => format!("{m} metal"),
        (0, p) => format!("{p} power"),
        (m, p) => format!("{m} metal and {p} power"),
    }
}

/// Why the colony refused, in a line.
pub fn refusal_words(refusal: Refusal, run: &Run) -> String {
    match refusal {
        Refusal::CantAfford { power, metal } => format!(
            "Too dear. It needs {}. {}.",
            cost_words(metal, power),
            run.colony.resources.words()
        ),
        Refusal::SlotClosed => "That slot is not open.".to_string(),
        Refusal::SlotTaken => "A building stands there already.".to_string(),
        Refusal::RowOpen => "Works go on rows 2 to 5.".to_string(),
        Refusal::SectorTaken => "That sector is taken.".to_string(),
        Refusal::NothingThere => "Nothing stands there.".to_string(),
        Refusal::NotDark => "It is working.".to_string(),
        Refusal::DomeWhole => "The dome is whole.".to_string(),
        Refusal::MissilesFull => format!("The rack holds {} already.", colony::MISSILES_MAX),
        Refusal::WrongPhase => "Not now.".to_string(),
        Refusal::Edge => "The rail ends there.".to_string(),
        Refusal::NoMissiles => "No missiles.".to_string(),
        Refusal::NoTarget => "Nothing to fire at.".to_string(),
    }
}

/// What a colony-phase event did, with the stores after it.
fn done_words(event: &Event, run: &Run) -> Option<String> {
    let r = run.colony.resources;
    let metal = format!("Metal {}.", r.metal);
    Some(match event {
        Event::Bought { building, col } => {
            let (_, power) = building.cost();
            let stores = if power > 0 {
                format!("Metal {}, power {}.", r.metal, r.power)
            } else {
                metal
            };
            format!(
                "{} built under the {}. {stores}",
                building.title(),
                column_name(*col)
            )
        }
        Event::Placed { work, at } => format!(
            "{} placed at {}. {metal}",
            sentence_start(work.name()),
            at.words()
        ),
        Event::DomeRepaired { dome } => format!("Dome {dome}. {metal}"),
        Event::BuildingRepaired { building, col } => format!(
            "{} under the {} repaired. {metal}",
            building.title(),
            column_name(*col)
        ),
        Event::MissileMade { missiles } => format!(
            "Missile made, {missiles} of {}. Power {}, metal {}.",
            colony::MISSILES_MAX,
            r.power,
            r.metal
        ),
        _ => return None,
    })
}

/// What the night left for the colony to hear: hunger, posts without
/// power, dark buildings.
pub fn notices(event: &Event, run: &Run) -> Vec<String> {
    let Event::ColonyOpens {
        hungry,
        unpowered,
        dark,
        ..
    } = event
    else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    if *hungry {
        lines.push(format!(
            "The colony is hungry: {} action a turn until there is food.",
            colony::ACTIONS_HUNGRY
        ));
    }
    for at in unpowered {
        lines.push(format!("No power for the flak post at {}.", at.words()));
    }
    for col in dark {
        if let Some(slot) = run.colony.slot(*col) {
            lines.push(format!(
                "The {} under the {} is dark.",
                slot.building.name(),
                column_name(*col)
            ));
        }
    }
    lines
}

/// F1 on a building's row.
pub fn building_help(building: Building) -> &'static str {
    match building {
        Building::Solar => "Makes 3 power a night.",
        Building::Hydroponics => "Grows 2 food a night. The colony eats 2 a night, and a hungry colony gets 1 action a turn. Sappers head for it.",
        Building::Scanner => "Tells the forecast which columns the raiders enter at, and tells the route finder a raider's whole route. Draws 1 power a night.",
        Building::Workshop => "Halves the cost of a missile.",
        Building::ShieldCoil => "Adds 10 shield a night. Draws 2 power a night.",
    }
}

/// F1 on a work's row.
pub fn work_help(work: WorkKind) -> &'static str {
    match work {
        WorkKind::Wall => "Blocks a sector. Raiders go round it, or bash it when the way round is long: 2 bashes bring it down.",
        WorkKind::Flak => "Fires at the nearest raider in the sectors beside it, every phase. Draws 1 power a night. Drifters keep out of its cover.",
        WorkKind::Mine => "Blows up under the first raider that steps on it, for 2 damage.",
    }
}

/// What stands at a sector: "open", "a wall, 1 hit left", "a flak post".
pub fn work_words(work: Option<Work>) -> String {
    match work {
        None => "open".to_string(),
        Some(Work::Wall { hits: 1 }) => "a wall, 1 hit left".to_string(),
        Some(Work::Wall { .. }) => "a wall".to_string(),
        Some(Work::Flak { powered: true }) => "a flak post".to_string(),
        Some(Work::Flak { powered: false }) => "a flak post without power".to_string(),
        Some(Work::Mine) => "a mine".to_string(),
    }
}

/// Every work on the grid, nearest row first.
pub fn grid_words(run: &Run) -> String {
    let mut works = run.grid.works();
    if works.is_empty() {
        return "The grid is open ground.".to_string();
    }
    works.sort_by_key(|(at, _)| (at.row, at.col));
    let parts: Vec<String> = works
        .iter()
        .map(|(at, work)| format!("{} at {}", work_words(Some(*work)), at.words()))
        .collect();
    format!("On the grid: {}.", parts.join("; "))
}

/// Every slot: what stands in it, or that it is open or closed.
pub fn slots_words(run: &Run) -> String {
    let mut parts = Vec::new();
    for col in 0..COLS {
        let name = column_name(col);
        let part = match run.colony.slot(col) {
            Some(slot) if slot.dark => format!("{} under the {name}, dark", slot.building.name()),
            Some(slot) if !slot.powered => {
                format!("{} under the {name}, without power", slot.building.name())
            }
            Some(slot) => format!("{} under the {name}", slot.building.name()),
            None if run.colony.open.contains(&col) => format!("the {name} slot open"),
            None => continue,
        };
        parts.push(part);
    }
    format!(
        "{} Missiles {} of {}. {}.",
        sentence_start(&format!("{}.", run.colony.resources.words())),
        run.colony.missiles,
        colony::MISSILES_MAX,
        sentence_start(&parts.join("; "))
    )
}

/// The forecast with its columns when a scanner works.
pub fn forecast_words(run: &Run) -> String {
    let kinds = run.forecast_words();
    if run.colony.has(Building::Scanner) {
        let mut cols: Vec<u8> = run.forecast.iter().map(|a| a.col).collect();
        cols.sort_unstable();
        cols.dedup();
        let names: Vec<&str> = cols.iter().map(|c| column_name(*c)).collect();
        format!(
            "Night {}: {kinds}. The scanner has them entering at the {}.",
            run.night,
            join_and(&names)
        )
    } else {
        format!("Night {}: {kinds}.", run.night)
    }
}

/// "left", "left and centre", "left, centre and right".
pub fn join_and(parts: &[&str]) -> String {
    match parts {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// Columns with an open, empty slot.
fn free_slots(run: &Run) -> Vec<u8> {
    run.colony
        .open
        .iter()
        .copied()
        .filter(|col| run.colony.slot(*col).is_none())
        .collect()
}

// --- the colony menu -------------------------------------------------------

pub struct ColonyScreen {
    menu: MenuCore<Self, Parlor>,
    run: Shared,
    notices: Vec<String>,
    /// Said after the title: what the night left.
    told: bool,
}

impl ColonyScreen {
    /// The colony phase of `run`, which has just opened it. Events the run
    /// holds become the notices said after the title.
    pub fn new(run: Shared) -> ColonyScreen {
        let notices = {
            let mut r = run.borrow_mut();
            let events = r.take_events();
            events.iter().flat_map(|e| notices(e, &r)).collect()
        };
        ColonyScreen {
            menu: MenuCore::new("Colony").with_intro_help(
                "Up and Down choose, Enter buys or places, Escape pauses. Ready for the night sends the raiders down.",
            ),
            run,
            notices,
            told: false,
        }
    }

    fn mode(&self) -> Mode {
        self.run.borrow().mode
    }

    /// Lines the night screen kept from the night's end, said after the
    /// title.
    pub fn with_notices(mut self, lines: Vec<String>) -> ColonyScreen {
        self.notices.extend(lines);
        self
    }

    pub fn run(&self) -> std::cell::Ref<'_, Watch> {
        self.run.borrow()
    }

    fn title_words(run: &Run) -> String {
        format!(
            "Before night {}. {}. Dome {}, shield {}.",
            run.night,
            run.colony.resources.words(),
            run.colony.dome,
            run.colony.shield
        )
    }

    /// Applies one input and says what it did, or why not.
    fn apply(&mut self, ctx: &mut Ctx<Parlor>, input: Input) {
        let outcome = self.run.borrow_mut().apply(input);
        match outcome {
            Ok(()) => {
                let lines: Vec<String> = {
                    let mut r = self.run.borrow_mut();
                    let events = r.take_events();
                    events.iter().filter_map(|e| done_words(e, &r)).collect()
                };
                ctx.audio.play(sound::BUY, Bus::EFFECTS);
                self.run.borrow().keep(ctx);
                self.refresh(ctx, true);
                ctx.say(lines.join(" "));
                let current = self.current_text(ctx);
                ctx.say_queued(current);
            }
            Err(refusal) => {
                ctx.audio.play(sound::CANT, Bus::EFFECTS);
                let text = refusal_words(refusal, &self.run.borrow());
                ctx.say(text);
            }
        }
    }

    fn ready(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.mode() == Mode::Practice {
            ctx.say("The tutorial colony stays as it is.");
            return;
        }
        let outcome = self.run.borrow_mut().apply(Input::Ready);
        if let Err(refusal) = outcome {
            let text = refusal_words(refusal, &self.run.borrow());
            ctx.say(text);
            return;
        }
        ctx.audio.play(sound::READY, Bus::EFFECTS);
        let run = Rc::clone(&self.run);
        ctx.replace(NightScreen::new(run));
    }

    fn can_afford(run: &Run, metal: u32, power: u32) -> bool {
        run.colony.resources.covers(power, metal)
    }
}

impl Menu for ColonyScreen {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        COLONY_NAME
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::on_watch(ctx);
        self.run.borrow().keep(ctx);
        {
            let mut w = self.run.borrow_mut();
            let hungry = w.colony.hungry();
            w.bed.set_hungry(hungry);
            w.bed.start(&mut ctx.audio);
        }
        hearken::menu::menu_enter(self, ctx);
        if !self.told {
            self.told = true;
            for line in self.notices.clone() {
                ctx.say_queued(line);
            }
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.run.borrow_mut().bed.stop(&mut ctx.audio, 900);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        self.run.borrow_mut().play_bed(&mut ctx.audio, dt);
    }

    fn on_escape(&mut self, ctx: &mut Ctx<Parlor>) {
        let leave = self.mode().leave();
        menus::pause(ctx, HELP, leave);
    }

    /// The repeat key says the row again, as it does on the night screen,
    /// instead of jumping to the Ready row by its first letter.
    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if ctx.action_for(event) == Some(ParlorAction::Repeat) {
            if !event.is_repeat() {
                let current = self.current_text(ctx);
                ctx.say(current);
            }
            return;
        }
        hearken::menu::menu_input(self, ctx, event);
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let run = self.run.borrow();
        self.menu.title = ColonyScreen::title_words(&run);
        let mut items = Vec::new();
        items.push(
            MenuItem::new(
                format!("Ready for night {}, {}", run.night, run.forecast_words()),
                |s: &mut ColonyScreen, ctx: &mut Ctx<Parlor>| s.ready(ctx),
            )
            .help(format!(
                "{} Nothing more can be bought once the night starts.",
                forecast_words(&run)
            )),
        );
        if run.colony.dome < colony::DOME_MAX {
            let ok = ColonyScreen::can_afford(&run, colony::DOME_REPAIR_COST, 0);
            items.push(
                MenuItem::new(
                    format!(
                        "Repair the dome, {} metal, dome {} of {}",
                        colony::DOME_REPAIR_COST,
                        run.colony.dome,
                        colony::DOME_MAX
                    ),
                    |s: &mut ColonyScreen, ctx: &mut Ctx<Parlor>| s.apply(ctx, Input::RepairDome),
                )
                .help(format!(
                    "Puts {} back on the dome, up to {}.{}",
                    colony::DOME_REPAIR,
                    colony::DOME_MAX,
                    if ok { "" } else { " Too dear for now." }
                )),
            );
        }
        for col in 0..COLS {
            let Some(slot) = run.colony.slot(col) else {
                continue;
            };
            if !slot.dark {
                continue;
            }
            items.push(
                MenuItem::new(
                    format!(
                        "Repair the {} under the {}, {} metal",
                        slot.building.name(),
                        column_name(col),
                        colony::BUILDING_REPAIR_COST
                    ),
                    move |s: &mut ColonyScreen, ctx: &mut Ctx<Parlor>| {
                        s.apply(ctx, Input::RepairBuilding { col })
                    },
                )
                .help("A dark building does nothing until it is repaired."),
            );
        }
        if run.colony.missiles < colony::MISSILES_MAX {
            let (power, metal) = run.colony.missile_cost();
            items.push(
                MenuItem::new(
                    format!(
                        "Make a missile, {}, {} of {} in the rack",
                        cost_words(metal, power),
                        run.colony.missiles,
                        colony::MISSILES_MAX
                    ),
                    |s: &mut ColonyScreen, ctx: &mut Ctx<Parlor>| s.apply(ctx, Input::MakeMissile),
                )
                .help(format!(
                    "A missile hits any raider on the grid for {} damage, from any column. A workshop halves the cost.",
                    colony::MISSILE_DAMAGE
                )),
            );
        }
        let free = free_slots(&run);
        if !free.is_empty() {
            for building in Building::ALL {
                let (metal, power) = building.cost();
                let free = free.clone();
                items.push(
                    MenuItem::new(
                        format!("Build {}, {}", building.a(), cost_words(metal, power)),
                        move |s: &mut ColonyScreen, ctx: &mut Ctx<Parlor>| {
                            let (metal, power) = building.cost();
                            let short = {
                                let r = s.run.borrow();
                                (!ColonyScreen::can_afford(&r, metal, power)).then(|| {
                                    refusal_words(Refusal::CantAfford { power, metal }, &r)
                                })
                            };
                            if let Some(text) = short {
                                ctx.audio.play(sound::CANT, Bus::EFFECTS);
                                ctx.say(text);
                            } else if free.len() == 1 {
                                s.apply(
                                    ctx,
                                    Input::Buy {
                                        building,
                                        col: free[0],
                                    },
                                );
                            } else {
                                ctx.push(SlotPicker::new(Rc::clone(&s.run), building));
                            }
                        },
                    )
                    .help(building_help(building)),
                );
            }
        }
        for work in WorkKind::ALL {
            items.push(
                MenuItem::new(
                    format!("Place a {}, {} metal", work.name(), work.cost()),
                    move |s: &mut ColonyScreen, ctx: &mut Ctx<Parlor>| {
                        let short = {
                            let r = s.run.borrow();
                            (!ColonyScreen::can_afford(&r, work.cost(), 0)).then(|| {
                                refusal_words(
                                    Refusal::CantAfford {
                                        power: 0,
                                        metal: work.cost(),
                                    },
                                    &r,
                                )
                            })
                        };
                        if let Some(text) = short {
                            ctx.audio.play(sound::CANT, Bus::EFFECTS);
                            ctx.say(text);
                        } else {
                            ctx.push(Placer::new(Rc::clone(&s.run), work));
                        }
                    },
                )
                .help(work_help(work)),
            );
        }
        items.push(
            MenuItem::new("The grid", |s: &mut ColonyScreen, ctx: &mut Ctx<Parlor>| {
                let text = grid_words(&s.run.borrow());
                ctx.say(text);
            })
            .select_sound(None)
            .help("Reads every wall, flak post and mine on the grid, nearest row first."),
        );
        items.push(
            MenuItem::new(
                "The colony",
                |s: &mut ColonyScreen, ctx: &mut Ctx<Parlor>| {
                    let text = slots_words(&s.run.borrow());
                    ctx.say(text);
                },
            )
            .select_sound(None)
            .help("Reads the stores, the missiles and what stands in each slot."),
        );
        items
    }
}

// --- the slot picker -------------------------------------------------------

/// Which open slot a building goes in.
pub struct SlotPicker {
    menu: MenuCore<Self, Parlor>,
    run: Shared,
    building: Building,
}

impl SlotPicker {
    pub fn new(run: Shared, building: Building) -> SlotPicker {
        SlotPicker {
            menu: MenuCore::new(format!("Where to build the {}", building.name())).with_intro_help(
                "Up and Down choose a slot, Enter builds there, Escape goes back.",
            ),
            run,
            building,
        }
    }
}

impl Menu for SlotPicker {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        SLOTS_NAME
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        self.run.borrow_mut().play_bed(&mut ctx.audio, dt);
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let free = free_slots(&self.run.borrow());
        let mut items: Vec<MenuItem<Self, Parlor>> = free
            .into_iter()
            .map(|col| {
                MenuItem::new(
                    format!("The {} slot", column_name(col)),
                    move |s: &mut SlotPicker, ctx: &mut Ctx<Parlor>| {
                        let building = s.building;
                        let outcome = s.run.borrow_mut().apply(Input::Buy { building, col });
                        match outcome {
                            Ok(()) => {
                                let line = {
                                    let mut r = s.run.borrow_mut();
                                    let events = r.take_events();
                                    events
                                        .iter()
                                        .filter_map(|e| done_words(e, &r))
                                        .collect::<Vec<_>>()
                                        .join(" ")
                                };
                                ctx.audio.play(sound::BUY, Bus::EFFECTS);
                                s.run.borrow().keep(ctx);
                                ctx.pop_with(line);
                            }
                            Err(refusal) => {
                                ctx.audio.play(sound::CANT, Bus::EFFECTS);
                                let text = refusal_words(refusal, &s.run.borrow());
                                ctx.say(text);
                            }
                        }
                    },
                )
                .help(format!(
                    "Builds the {} under the {} column.",
                    self.building.name(),
                    column_name(col)
                ))
            })
            .collect();
        items.push(
            MenuItem::new(
                labels::BACK,
                |_s: &mut SlotPicker, ctx: &mut Ctx<Parlor>| ctx.pop_with("Nothing built."),
            )
            .help("Back to the colony without building."),
        );
        items
    }
}

// --- the placer ------------------------------------------------------------

/// Walks the grid to put a work down.
pub struct Placer {
    run: Shared,
    work: WorkKind,
    at: Sector,
    last: String,
}

impl Placer {
    pub fn new(run: Shared, work: WorkKind) -> Placer {
        let at = Sector::new(crate::rules::grid::CENTRE, crate::rules::grid::CROSSING_ROW);
        Placer {
            run,
            work,
            at,
            last: String::new(),
        }
    }

    /// "Centre, row 4, open."
    fn here(&self) -> String {
        let run = self.run.borrow();
        format!(
            "{}, {}.",
            sentence_start(&self.at.words()),
            work_words(run.grid.get(self.at))
        )
    }

    fn tell(&mut self, ctx: &mut Ctx<Parlor>, text: String) {
        self.last = text.clone();
        ctx.say(text);
    }

    fn step(&mut self, ctx: &mut Ctx<Parlor>, dcol: i8, drow: i8) {
        let col = (self.at.col as i8 + dcol).clamp(0, COLS as i8 - 1) as u8;
        let row = (self.at.row as i8 + drow).clamp(*WORK_ROWS.start() as i8, *WORK_ROWS.end() as i8)
            as u8;
        let next = Sector::new(col, row);
        if next == self.at {
            let text = format!("The grid ends there. {}", self.here());
            self.tell(ctx, text);
            return;
        }
        self.at = next;
        sound::at(ctx, sound::SCAN, self.at);
        let text = self.here();
        self.tell(ctx, text);
    }

    fn place(&mut self, ctx: &mut Ctx<Parlor>) {
        let outcome = self.run.borrow_mut().apply(Input::Place {
            work: self.work,
            at: self.at,
        });
        match outcome {
            Ok(()) => {
                let line = {
                    let mut r = self.run.borrow_mut();
                    let events = r.take_events();
                    events
                        .iter()
                        .filter_map(|e| done_words(e, &r))
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                sound::at(ctx, sound::BUY, self.at);
                self.run.borrow().keep(ctx);
                ctx.pop_with(line);
            }
            Err(refusal) => {
                ctx.audio.play(sound::CANT, Bus::EFFECTS);
                let text = refusal_words(refusal, &self.run.borrow());
                self.tell(ctx, text);
            }
        }
    }

    /// Where the forecast's raiders come in.
    fn entries(&self) -> String {
        let run = self.run.borrow();
        if !run.colony.has(Building::Scanner) {
            return format!(
                "{} A scanner would say where they enter.",
                forecast_words(&run)
            );
        }
        forecast_words(&run)
    }
}

impl State<Parlor> for Placer {
    fn name(&self) -> &'static str {
        PLACER_NAME
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        self.run.borrow_mut().play_bed(&mut ctx.audio, dt);
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        let left = ctx.control_name(ParlorAction::Left);
        let right = ctx.control_name(ParlorAction::Right);
        let up = ctx.control_name(ParlorAction::Up);
        let down = ctx.control_name(ParlorAction::Down);
        let text = format!(
            "Place a {}. {} and {right} choose the column, {up} and {down} the row, Enter places it. {}",
            self.work.name(),
            sentence_start(&left),
            self.here()
        );
        sound::at(ctx, sound::SCAN, self.at);
        self.tell(ctx, text);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() {
            return;
        }
        let action = ctx.action_for(event);
        if action.is_none() {
            match event.pressed() {
                Some(Key::Escape) => ctx.pop_with("Nothing placed."),
                Some(Key::Return | Key::KpEnter) => self.place(ctx),
                _ => {}
            }
            return;
        }
        match action {
            Some(ParlorAction::Left) => self.step(ctx, -1, 0),
            Some(ParlorAction::Right) => self.step(ctx, 1, 0),
            Some(ParlorAction::Up) => self.step(ctx, 0, 1),
            Some(ParlorAction::Down) => self.step(ctx, 0, -1),
            Some(ParlorAction::Primary) => self.place(ctx),
            Some(ParlorAction::Status) => {
                let run = self.run.borrow();
                let text = format!(
                    "A {} costs {} metal. {}.",
                    self.work.name(),
                    self.work.cost(),
                    run.colony.resources.words()
                );
                drop(run);
                self.tell(ctx, text);
            }
            Some(ParlorAction::Hint) => {
                let text = self.entries();
                self.tell(ctx, text);
            }
            Some(ParlorAction::Repeat) => {
                let text = self.last.clone();
                ctx.say(text);
            }
            Some(ParlorAction::Secondary) | None => {}
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        vec![
            format!(
                "placing {} at {},{}",
                self.work.name(),
                self.at.col,
                self.at.row
            ),
            end_sentence(&self.here()),
        ]
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let left = ctx.control_name(ParlorAction::Left);
        let right = ctx.control_name(ParlorAction::Right);
        let up = ctx.control_name(ParlorAction::Up);
        let down = ctx.control_name(ParlorAction::Down);
        let status = ctx.control_name(ParlorAction::Status);
        let hint = ctx.control_name(ParlorAction::Hint);
        format!(
            "{} and {right} move along the row, {up} goes a row farther out, {down} a row nearer the dome. Enter places the {} where you are. {} says what it costs. {} says what is coming. Escape goes back without placing.",
            sentence_start(&left),
            self.work.name(),
            sentence_start(&status),
            sentence_start(&hint)
        )
    }

    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}
