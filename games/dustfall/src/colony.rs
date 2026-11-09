//! The colony phase: between nights the stores are spent, buildings go
//! into the slots and mines into the lanes, then Ready sends the night
//! down.
//!
//! Two screens share one run: the colony menu, and the lane picker for a
//! building or a mine.

use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent};
use parlor_core::menus::{self, labels, Leave};
use parlor_core::{Parlor, ParlorAction, Period};

use crate::challenge::CHALLENGES_NAME;
use crate::music::Conductor;
use crate::night::NightScreen;
use crate::progress::Progress;
use crate::rules::lanes::{lane_name, LANES};
use crate::rules::{colony, orders, Building, Event, Input, Order, Refusal, Run};
use crate::sound;
use crate::{HELP, TITLE_NAME};

pub const COLONY_NAME: &str = "dustfall_colony";
pub const SLOTS_NAME: &str = "dustfall_slots";

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
        Refusal::LaneMined => "A mine lies in that lane already.".to_string(),
        Refusal::MinesLaid => match run.colony.mines_a_night {
            1 => "One mine a night.".to_string(),
            n => format!("{n} mines a night."),
        },
        Refusal::NothingThere => "Nothing stands there.".to_string(),
        Refusal::NotDark => "It is working.".to_string(),
        Refusal::DomeWhole => "The dome is whole.".to_string(),
        Refusal::MissilesFull => format!("The rack holds {} already.", run.colony.missiles_max),
        Refusal::NoSuchOrder => "That order is not on offer tonight.".to_string(),
        Refusal::OrderTaken => "An order is taken already tonight.".to_string(),
        Refusal::WrongPhase => "Not now.".to_string(),
        Refusal::Edge => "The rail ends there.".to_string(),
        Refusal::Reloading => "Reloading.".to_string(),
        Refusal::NoMissiles => "No missiles.".to_string(),
        Refusal::NoTarget => "Nothing to fire at.".to_string(),
    }
}

/// What a colony-phase event did, with the stores after it.
fn done_words(event: &Event, run: &Run) -> Option<String> {
    let r = run.colony.resources;
    let metal = format!("Metal {}.", r.metal);
    Some(match event {
        Event::Bought { building, lane } => {
            let (_, power) = building.cost();
            let stores = if power > 0 {
                format!("Metal {}, power {}.", r.metal, r.power)
            } else {
                metal
            };
            format!(
                "{} built under the {}. {stores}",
                building.title(),
                lane_name(*lane)
            )
        }
        Event::MineLaid { lane } => {
            format!("Mine laid in the {} lane. {metal}", lane_name(*lane))
        }
        Event::DomeRepaired { dome } => format!("Dome {dome}. {metal}"),
        Event::BuildingRepaired { building, lane } => format!(
            "{} under the {} repaired. {metal}",
            building.title(),
            lane_name(*lane)
        ),
        Event::MissileMade { missiles } => format!(
            "Missile made, {missiles} of {}. Power {}, metal {}.",
            run.colony.missiles_max, r.power, r.metal
        ),
        Event::OrderTaken { order } => match order {
            Order::Overclock => format!("Overclock taken. Shield {}.", run.colony.shield),
            Order::Rationing => format!("Rationing taken. Food {}.", r.food),
            Order::Salvage => {
                "Salvage taken. Twice the metal from kills tonight, and the skitters come faster."
                    .to_string()
            }
        },
        _ => return None,
    })
}

/// F1 on an order's row.
pub fn order_help(order: Order) -> String {
    match order {
        Order::Overclock => format!(
            "Puts {} on the shield now, up to its cap. After the night the colony pays {} power.",
            orders::OVERCLOCK_SHIELD,
            orders::OVERCLOCK_POWER
        ),
        Order::Rationing => format!(
            "{} food into store now. Tonight's waves come closer together, and the raiders in them.",
            orders::RATIONING_FOOD
        ),
        Order::Salvage => {
            "Every kill tonight drops twice the metal. The skitters fall half as fast again."
                .to_string()
        }
    }
}

/// What the night left for the colony to hear: hunger, posts without
/// power, dark buildings.
pub fn notices(event: &Event, run: &Run) -> Vec<String> {
    if let Event::OrderPaid { order } = event {
        return vec![format!(
            "{}: {} power spent. Power {}.",
            order.name(),
            orders::OVERCLOCK_POWER,
            run.colony.resources.power
        )];
    }
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
        lines.push(
            "The colony is hungry: the gun reloads at half speed until there is food.".to_string(),
        );
    }
    for lane in unpowered {
        if let Some(slot) = run.colony.slot(*lane) {
            lines.push(format!(
                "No power for the {} under the {}.",
                slot.building.name(),
                lane_name(*lane)
            ));
        }
    }
    for lane in dark {
        if let Some(slot) = run.colony.slot(*lane) {
            lines.push(format!(
                "The {} under the {} is dark.",
                slot.building.name(),
                lane_name(*lane)
            ));
        }
    }
    lines
}

/// F1 on a building's row.
pub fn building_help(building: Building) -> &'static str {
    match building {
        Building::Solar => "Makes 3 power a night.",
        Building::Hydroponics => "Grows 2 food a night. The colony eats 2 a night, and a hungry colony's gun reloads at half speed. Sappers head for it.",
        Building::Scanner => "Tells the forecast which lanes the raiders enter at, and the sky which lane the next raider takes. Draws 1 power a night.",
        Building::Workshop => "Halves the cost of a missile.",
        Building::ShieldCoil => "Adds 10 shield a night, and recharges it twice as fast in the night. Draws 2 power a night.",
        Building::Flak => "Fires up its own lane through the night, once every few seconds, at the nearest raider short of the horizon. Draws 1 power a night. Silent under the dust.",
    }
}

/// F1 on the mine's row.
pub fn mine_help(run: &Run) -> String {
    format!(
        "Blows up under the first raider past mid range in its lane, for {} damage. {} It is spent when it goes off.",
        colony::MINE_DAMAGE,
        match run.colony.mines_a_night {
            1 => "One may be laid a night.".to_string(),
            n => format!("{n} may be laid a night."),
        }
    )
}

/// Every slot: what stands in it, or that it is open or closed; then the
/// mines.
pub fn slots_words(run: &Run) -> String {
    let mut parts = Vec::new();
    for lane in 0..LANES {
        let name = lane_name(lane);
        let part = match run.colony.slot(lane) {
            Some(slot) if slot.dark => format!("{} under the {name}, dark", slot.building.name()),
            Some(slot) if !slot.powered => {
                format!("{} under the {name}, without power", slot.building.name())
            }
            Some(slot) => format!("{} under the {name}", slot.building.name()),
            None if run.colony.open.contains(&lane) => format!("the {name} slot open"),
            None => continue,
        };
        parts.push(part);
    }
    let mined = run.colony.mined_lanes();
    let mines = if mined.is_empty() {
        "No mines laid.".to_string()
    } else {
        let names: Vec<&str> = mined.iter().map(|l| lane_name(*l)).collect();
        format!(
            "{} in the {} lane{}.",
            if mined.len() == 1 { "A mine" } else { "Mines" },
            join_and(&names),
            if mined.len() == 1 { "" } else { "s" }
        )
    };
    format!(
        "{} Missiles {} of {}. {}. {mines}",
        sentence_start(&format!("{}.", run.colony.resources.words())),
        run.colony.missiles,
        run.colony.missiles_max,
        sentence_start(&parts.join("; "))
    )
}

/// The forecast with its lanes when a scanner works or the lane unlock is
/// earned, and its quirks when a scanner works or the quirks unlock is.
pub fn forecast_words(run: &Run) -> String {
    let kinds = run.forecast_words();
    let scanner = run.colony.has(Building::Scanner);
    let mut line = if scanner || run.perks.lane_forecast {
        let mut lanes: Vec<u8> = run.forecast.iter().map(|a| a.lane).collect();
        lanes.sort_unstable();
        lanes.dedup();
        let names: Vec<&str> = lanes.iter().map(|l| lane_name(*l)).collect();
        format!(
            "Night {}: {kinds}. {} entering at the {}.",
            run.night,
            if scanner {
                "The scanner has them"
            } else {
                "They come"
            },
            join_and(&names)
        )
    } else {
        format!("Night {}: {kinds}.", run.night)
    };
    if scanner || run.perks.forecast_quirks {
        if let Some(quirks) = run.quirk_words() {
            line.push(' ');
            line.push_str(&quirks);
        }
    }
    line
}

/// "left", "left and centre", "left, centre and right".
pub fn join_and(parts: &[&str]) -> String {
    match parts {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// Lanes with an open, empty slot.
fn free_slots(run: &Run) -> Vec<u8> {
    run.colony
        .open
        .iter()
        .copied()
        .filter(|lane| run.colony.slot(*lane).is_none())
        .collect()
}

/// Lanes without a mine.
fn unmined(run: &Run) -> Vec<u8> {
    (0..LANES).filter(|l| !run.colony.mined(*l)).collect()
}

/// Whether another mine may be laid before this night.
fn mines_left(run: &Run) -> bool {
    run.mines_laid < run.colony.mines_a_night && !unmined(run).is_empty()
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
                "Up and Down choose, Enter buys, Escape pauses. Ready for the night sends the raiders down.",
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
        if run.colony.dome < run.colony.dome_max {
            let ok = ColonyScreen::can_afford(&run, colony::DOME_REPAIR_COST, 0);
            items.push(
                MenuItem::new(
                    format!(
                        "Repair the dome, {} metal, dome {} of {}",
                        colony::DOME_REPAIR_COST,
                        run.colony.dome,
                        run.colony.dome_max
                    ),
                    |s: &mut ColonyScreen, ctx: &mut Ctx<Parlor>| s.apply(ctx, Input::RepairDome),
                )
                .help(format!(
                    "Puts {} back on the dome, up to {}.{}",
                    colony::DOME_REPAIR,
                    run.colony.dome_max,
                    if ok { "" } else { " Too dear for now." }
                )),
            );
        }
        for lane in 0..LANES {
            let Some(slot) = run.colony.slot(lane) else {
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
                        lane_name(lane),
                        colony::BUILDING_REPAIR_COST
                    ),
                    move |s: &mut ColonyScreen, ctx: &mut Ctx<Parlor>| {
                        s.apply(ctx, Input::RepairBuilding { lane })
                    },
                )
                .help("A dark building does nothing until it is repaired."),
            );
        }
        if run.mode != Mode::Practice {
            match run.order {
                None => {
                    for order in run.offered.clone() {
                        items.push(
                            MenuItem::new(
                                format!("Order: {}, {}", order.name(), order.terms()),
                                move |s: &mut ColonyScreen, ctx: &mut Ctx<Parlor>| {
                                    s.apply(ctx, Input::TakeOrder(order))
                                },
                            )
                            .help(format!(
                                "{} One order a night, of the two on offer.",
                                order_help(order)
                            )),
                        );
                    }
                }
                Some(order) => items.push(
                    MenuItem::new(
                        format!("Order taken: {}", order.name()),
                        move |_s: &mut ColonyScreen, ctx: &mut Ctx<Parlor>| {
                            ctx.say(format!("{}: {}.", order.name(), order.terms()));
                        },
                    )
                    .select_sound(None)
                    .help("Tonight's order. It stands until the night is over."),
                ),
            }
        }
        if run.colony.missiles < run.colony.missiles_max {
            let (power, metal) = run.colony.missile_cost();
            items.push(
                MenuItem::new(
                    format!(
                        "Make a missile, {}, {} of {} in the rack",
                        cost_words(metal, power),
                        run.colony.missiles,
                        run.colony.missiles_max
                    ),
                    |s: &mut ColonyScreen, ctx: &mut Ctx<Parlor>| s.apply(ctx, Input::MakeMissile),
                )
                .help(format!(
                    "A missile hits the nearest raider in the sky for {} damage, whatever lane it is in. A workshop halves the cost.",
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
                                        lane: free[0],
                                    },
                                );
                            } else {
                                ctx.push(LanePicker::new(Rc::clone(&s.run), Pick::Build(building)));
                            }
                        },
                    )
                    .help(building_help(building)),
                );
            }
        }
        if mines_left(&run) {
            items.push(
                MenuItem::new(
                    format!(
                        "Lay a mine, {} metal, {} of {} tonight",
                        colony::MINE_COST,
                        run.mines_laid + 1,
                        run.colony.mines_a_night
                    ),
                    move |s: &mut ColonyScreen, ctx: &mut Ctx<Parlor>| {
                        let short = {
                            let r = s.run.borrow();
                            (!ColonyScreen::can_afford(&r, colony::MINE_COST, 0)).then(|| {
                                refusal_words(
                                    Refusal::CantAfford {
                                        power: 0,
                                        metal: colony::MINE_COST,
                                    },
                                    &r,
                                )
                            })
                        };
                        if let Some(text) = short {
                            ctx.audio.play(sound::CANT, Bus::EFFECTS);
                            ctx.say(text);
                        } else {
                            ctx.push(LanePicker::new(Rc::clone(&s.run), Pick::Mine));
                        }
                    },
                )
                .help(mine_help(&run)),
            );
        }
        items.push(
            MenuItem::new(
                "The colony",
                |s: &mut ColonyScreen, ctx: &mut Ctx<Parlor>| {
                    let text = slots_words(&s.run.borrow());
                    ctx.say(text);
                },
            )
            .select_sound(None)
            .help(
                "Reads the stores, the missiles, what stands in each slot and where the mines lie.",
            ),
        );
        items
    }
}

// --- the lane picker -------------------------------------------------------

/// What the lane picker puts down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Build(Building),
    Mine,
}

impl Pick {
    fn title(self) -> String {
        match self {
            Pick::Build(b) => format!("Where to build the {}", b.name()),
            Pick::Mine => "Which lane to mine".to_string(),
        }
    }

    /// The lanes on offer.
    fn lanes(self, run: &Run) -> Vec<u8> {
        match self {
            Pick::Build(_) => free_slots(run),
            Pick::Mine => unmined(run),
        }
    }

    fn input(self, lane: u8) -> Input {
        match self {
            Pick::Build(building) => Input::Buy { building, lane },
            Pick::Mine => Input::LayMine { lane },
        }
    }

    fn row(self, lane: u8) -> String {
        match self {
            Pick::Build(_) => format!("The {} slot", lane_name(lane)),
            Pick::Mine => format!("The {} lane", lane_name(lane)),
        }
    }

    fn row_help(self, lane: u8) -> String {
        match self {
            Pick::Build(b) => format!(
                "Builds the {} under the {} lane.",
                b.name(),
                lane_name(lane)
            ),
            Pick::Mine => format!("Lays the mine in the {} lane.", lane_name(lane)),
        }
    }

    fn nothing(self) -> &'static str {
        match self {
            Pick::Build(_) => "Nothing built.",
            Pick::Mine => "No mine laid.",
        }
    }
}

/// Which lane a building or a mine goes in.
pub struct LanePicker {
    menu: MenuCore<Self, Parlor>,
    run: Shared,
    pick: Pick,
}

impl LanePicker {
    pub fn new(run: Shared, pick: Pick) -> LanePicker {
        LanePicker {
            menu: MenuCore::new(pick.title()).with_intro_help(
                "Up and Down choose a lane, Enter puts it there, Escape goes back.",
            ),
            run,
            pick,
        }
    }
}

impl Menu for LanePicker {
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
        let pick = self.pick;
        let lanes = pick.lanes(&self.run.borrow());
        let mut items: Vec<MenuItem<Self, Parlor>> = lanes
            .into_iter()
            .map(|lane| {
                MenuItem::new(
                    pick.row(lane),
                    move |s: &mut LanePicker, ctx: &mut Ctx<Parlor>| {
                        let outcome = s.run.borrow_mut().apply(s.pick.input(lane));
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
                                sound::under(ctx, sound::BUY, lane);
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
                .help(pick.row_help(lane))
            })
            .collect();
        items.push(
            MenuItem::new(
                labels::BACK,
                move |_s: &mut LanePicker, ctx: &mut Ctx<Parlor>| ctx.pop_with(pick.nothing()),
            )
            .help("Back to the colony without spending."),
        );
        items
    }
}
