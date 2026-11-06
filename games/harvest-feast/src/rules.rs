//! The kitchen's rules: stations, dishes, recipes, timing and scoring.
//!
//! Everything here is plain data updated by `dt`, so tests can cook a meal
//! frame by frame with no screen, sound or clock.

use std::cell::Cell;

use parlor_core::Rng;

/// Pressing the main action this long before a step is due still counts as
/// on time.
pub const EARLY_GRACE: f64 = 0.25;
/// What a fumble (acting before a dish is ready) costs the dish.
pub const FUMBLE_COST: u32 = 5;
/// How long a recipe line stays fresh, and then how long it can run late,
/// at speed 1.
pub const START_WINDOW: f64 = 4.0;
pub const START_BURN: f64 = 4.0;
/// No window or burn shrinks below this, however fast the course.
pub const MIN_WINDOW: f64 = 2.0;
/// At most this many dishes need the cook at once: another dish that would
/// come due simmers on until one of them is seen to, so one pair of hands
/// can keep up at the fastest courses.
pub const MAX_NEEDING: usize = 2;
/// The number of campaign courses.
pub const COURSES: u32 = 8;

thread_local! {
    static SEED: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Fixes the seed second helpings draw their meals from (tests), or with
/// `None` goes back to the clock.
pub fn fix_seed(seed: Option<u64>) {
    SEED.with(|s| s.set(seed));
}

/// The random source a new meal uses: the fixed seed, else the clock.
pub fn meal_rng() -> Rng {
    match SEED.with(|s| s.get()) {
        Some(seed) => Rng::seeded(seed),
        None => Rng::from_time(),
    }
}

/// The five stations, from far left to far right.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Station {
    LeftBurner,
    Board,
    Oven,
    Griddle,
    RightBurner,
}

impl Station {
    pub const ALL: [Station; 5] = [
        Station::LeftBurner,
        Station::Board,
        Station::Oven,
        Station::Griddle,
        Station::RightBurner,
    ];

    /// 0 for the far left to 4 for the far right.
    pub fn index(self) -> usize {
        Station::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    /// The number key that jumps to it, 1 to 5.
    pub fn number(self) -> u32 {
        self.index() as u32 + 1
    }

    pub fn from_number(n: u32) -> Option<Station> {
        Station::ALL.get((n as usize).checked_sub(1)?).copied()
    }

    /// "left burner".
    pub fn name(self) -> &'static str {
        match self {
            Station::LeftBurner => "left burner",
            Station::Board => "cutting board",
            Station::Oven => "oven",
            Station::Griddle => "griddle",
            Station::RightBurner => "right burner",
        }
    }

    /// "Left burner".
    pub fn title(self) -> &'static str {
        match self {
            Station::LeftBurner => "Left burner",
            Station::Board => "Cutting board",
            Station::Oven => "Oven",
            Station::Griddle => "Griddle",
            Station::RightBurner => "Right burner",
        }
    }

    /// "on the left burner", "in the oven".
    pub fn on(self) -> &'static str {
        match self {
            Station::LeftBurner => "on the left burner",
            Station::Board => "on the cutting board",
            Station::Oven => "in the oven",
            Station::Griddle => "on the griddle",
            Station::RightBurner => "on the right burner",
        }
    }

    /// Where it is around you.
    pub fn place(self) -> &'static str {
        match self {
            Station::LeftBurner => "far left",
            Station::Board => "front left",
            Station::Oven => "straight ahead",
            Station::Griddle => "front right",
            Station::RightBurner => "far right",
        }
    }

    /// What the station's dishes sound like, by stage.
    pub fn cooking_sound(self) -> &'static str {
        match self {
            Station::LeftBurner => "a slow, thick simmer",
            Station::Board => "nothing",
            Station::Oven => "a low fan hum",
            Station::Griddle => "a soft sizzle",
            Station::RightBurner => "light, quick little bubbles",
        }
    }

    pub fn due_sound(self) -> &'static str {
        match self {
            Station::LeftBurner => "thick bubbles coming fast and spitting",
            Station::Board => "a knife tip tapping the board",
            Station::Oven => "the oven timer ringing",
            Station::Griddle => "the sizzle turning sharp and crackly",
            Station::RightBurner => "a rolling boil with the lid rattling",
        }
    }

    pub fn burning_sound(self) -> &'static str {
        match self {
            Station::LeftBurner => "a harsh, crackling scorch",
            Station::Board => "fast, impatient knife taps",
            Station::Oven => "fast beeping and crackling smoke",
            Station::Griddle => "rough, popping smoke",
            Station::RightBurner => "water hissing on the flame and the lid clattering",
        }
    }

    /// "Bubbling up on the left burner: the gravy."
    fn due_line(self, dish: &str) -> String {
        match self {
            Station::LeftBurner => format!("Bubbling up on the left burner: the {dish}."),
            Station::Board => format!("Ready on the cutting board: the {dish}."),
            Station::Oven => format!("The oven timer rings for the {dish}."),
            Station::Griddle => format!("Sizzling sharp on the griddle: the {dish}."),
            Station::RightBurner => format!("Boiling hard on the right burner: the {dish}."),
        }
    }

    fn burning_line(self, dish: &str) -> String {
        match self {
            Station::LeftBurner => format!("Scorching on the left burner: the {dish}!"),
            Station::Board => format!("Still waiting on the cutting board: the {dish}. Hurry!"),
            Station::Oven => format!("Smoke from the oven: save the {dish}!"),
            Station::Griddle => format!("Smoking on the griddle: the {dish}!"),
            Station::RightBurner => format!("Boiling over on the right burner: the {dish}!"),
        }
    }

    fn ruined_word(self) -> &'static str {
        match self {
            Station::LeftBurner => "scorched",
            Station::Board => "went limp waiting",
            Station::Oven => "burned",
            Station::Griddle => "burned",
            Station::RightBurner => "boiled over",
        }
    }
}

/// What the main action does at a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Start,
    Stir,
    Whisk,
    Flip,
    Baste,
    Rotate,
    Pull,
    TakeOff,
    Drain,
    Chop,
    Plate,
}

impl Action {
    /// "Stir it", the call-out's command; "them" for a plural dish.
    pub fn command(self, plural: bool) -> String {
        pronoun(self.command_one(), plural)
    }

    fn command_one(self) -> &'static str {
        match self {
            Action::Start => "Start it",
            Action::Stir => "Stir it",
            Action::Whisk => "Whisk it",
            Action::Flip => "Flip it",
            Action::Baste => "Baste it",
            Action::Rotate => "Turn the tray",
            Action::Pull => "Pull it out",
            Action::TakeOff => "Take it off the heat",
            Action::Drain => "Drain it",
            Action::Chop => "Chop it",
            Action::Plate => "Plate it",
        }
    }

    /// "stir", for "next: stir"; "them" for a plural dish.
    pub fn verb(self, plural: bool) -> String {
        pronoun(self.verb_one(), plural)
    }

    fn verb_one(self) -> &'static str {
        match self {
            Action::Start => "start",
            Action::Stir => "stir",
            Action::Whisk => "whisk",
            Action::Flip => "flip",
            Action::Baste => "baste",
            Action::Rotate => "turn the tray",
            Action::Pull => "pull it out",
            Action::TakeOff => "take it off the heat",
            Action::Drain => "drain",
            Action::Chop => "chop",
            Action::Plate => "plate",
        }
    }

    /// "stirring", for "needs stirring"; "their" for a plural dish.
    pub fn gerund(self, plural: bool) -> String {
        pronoun(self.gerund_one(), plural)
    }

    fn gerund_one(self) -> &'static str {
        match self {
            Action::Start => "starting",
            Action::Stir => "stirring",
            Action::Whisk => "whisking",
            Action::Flip => "flipping",
            Action::Baste => "basting",
            Action::Rotate => "its tray turned",
            Action::Pull => "pulling out",
            Action::TakeOff => "taking off the heat",
            Action::Drain => "draining",
            Action::Chop => "chopping",
            Action::Plate => "plating",
        }
    }
}

/// `text` with "it" and "its" as "them" and "their" for a plural dish.
fn pronoun(text: &str, plural: bool) -> String {
    if !plural {
        return text.to_string();
    }
    text.split(' ')
        .map(|w| match w {
            "it" => "them",
            "its" => "their",
            w => w,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// One step of a dish after it is started.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepSpec {
    pub action: Action,
    /// Seconds of cooking, from the last step, until this one is due.
    pub wait: f64,
    /// Seconds it stays good once due.
    pub window: f64,
    /// Seconds it can then burn before it is ruined.
    pub burn: f64,
    /// Presses the step takes (chopping, whisking cream); 1 for most.
    pub strokes: u32,
}

const fn step(action: Action, wait: f64) -> StepSpec {
    StepSpec {
        action,
        wait,
        window: 3.0,
        burn: 3.0,
        strokes: 1,
    }
}

const fn quick(action: Action, wait: f64, window: f64) -> StepSpec {
    StepSpec {
        action,
        wait,
        window,
        burn: window,
        strokes: 1,
    }
}

const fn strokes(action: Action, n: u32, window: f64) -> StepSpec {
    StepSpec {
        action,
        wait: 0.0,
        window,
        burn: 4.0,
        strokes: n,
    }
}

/// Every dish in the cookbook.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DishKind {
    CranberrySauce,
    Gravy,
    Caramel,
    GreenBeans,
    Potatoes,
    Cider,
    Rolls,
    Stuffing,
    Turkey,
    PumpkinPie,
    CornCakes,
    Sprouts,
    Fritters,
    Celery,
    Herbs,
    Apples,
    Cream,
    /// Thanksgiving dinner's own dishes, cooked only on the day.
    SweetPotatoes,
    MapleCarrots,
    Cornbread,
    Carving,
    PecanPie,
}

impl DishKind {
    /// The dishes made only for Thanksgiving dinner. They are not in
    /// [`DishKind::ALL`], so no other meal draws them.
    pub const THANKSGIVING: [DishKind; 5] = [
        DishKind::SweetPotatoes,
        DishKind::MapleCarrots,
        DishKind::Cornbread,
        DishKind::Carving,
        DishKind::PecanPie,
    ];

    pub const ALL: [DishKind; 17] = [
        DishKind::CranberrySauce,
        DishKind::Gravy,
        DishKind::Caramel,
        DishKind::GreenBeans,
        DishKind::Potatoes,
        DishKind::Cider,
        DishKind::Rolls,
        DishKind::Stuffing,
        DishKind::Turkey,
        DishKind::PumpkinPie,
        DishKind::CornCakes,
        DishKind::Sprouts,
        DishKind::Fritters,
        DishKind::Celery,
        DishKind::Herbs,
        DishKind::Apples,
        DishKind::Cream,
    ];

    pub fn name(self) -> &'static str {
        match self {
            DishKind::CranberrySauce => "cranberry sauce",
            DishKind::Gravy => "gravy",
            DishKind::Caramel => "caramel",
            DishKind::GreenBeans => "green beans",
            DishKind::Potatoes => "potatoes",
            DishKind::Cider => "hot cider",
            DishKind::Rolls => "dinner rolls",
            DishKind::Stuffing => "stuffing",
            DishKind::Turkey => "turkey",
            DishKind::PumpkinPie => "pumpkin pie",
            DishKind::CornCakes => "corn cakes",
            DishKind::Sprouts => "Brussels sprouts",
            DishKind::Fritters => "apple fritters",
            DishKind::Celery => "celery and onions",
            DishKind::Herbs => "fresh herbs",
            DishKind::Apples => "apples",
            DishKind::Cream => "whipped cream",
            DishKind::SweetPotatoes => "sweet potato casserole",
            DishKind::MapleCarrots => "maple carrots",
            DishKind::Cornbread => "skillet cornbread",
            DishKind::Carving => "carved turkey",
            DishKind::PecanPie => "pecan pie",
        }
    }

    /// "it" or "them", for the dish.
    pub fn it(self) -> &'static str {
        if self.plural() {
            "them"
        } else {
            "it"
        }
    }

    /// "it" or "they", as a subject.
    pub fn it_subject(self) -> &'static str {
        if self.plural() {
            "they"
        } else {
            "it"
        }
    }

    /// "is" or "are", for the dish.
    pub fn is(self) -> &'static str {
        if self.plural() {
            "are"
        } else {
            "is"
        }
    }

    /// "needs" or "need", for the dish.
    pub fn needs(self) -> &'static str {
        if self.plural() {
            "need"
        } else {
            "needs"
        }
    }

    /// Whether the dish's name is plural: "the rolls burned; someone
    /// rescued them".
    pub fn plural(self) -> bool {
        matches!(
            self,
            DishKind::GreenBeans
                | DishKind::Potatoes
                | DishKind::Rolls
                | DishKind::CornCakes
                | DishKind::Sprouts
                | DishKind::Fritters
                | DishKind::Celery
                | DishKind::Herbs
                | DishKind::Apples
                | DishKind::MapleCarrots
        )
    }

    pub fn station(self) -> Station {
        match self {
            DishKind::CranberrySauce | DishKind::Gravy | DishKind::Caramel => Station::LeftBurner,
            DishKind::GreenBeans
            | DishKind::Potatoes
            | DishKind::Cider
            | DishKind::MapleCarrots => Station::RightBurner,
            DishKind::Rolls
            | DishKind::Stuffing
            | DishKind::Turkey
            | DishKind::PumpkinPie
            | DishKind::SweetPotatoes
            | DishKind::PecanPie => Station::Oven,
            DishKind::CornCakes | DishKind::Sprouts | DishKind::Fritters | DishKind::Cornbread => {
                Station::Griddle
            }
            DishKind::Celery
            | DishKind::Herbs
            | DishKind::Apples
            | DishKind::Cream
            | DishKind::Carving => Station::Board,
        }
    }

    /// What the recipe says to start it: "put the cranberries".
    pub fn start_phrase(self) -> &'static str {
        match self {
            DishKind::CranberrySauce => "put the cranberries",
            DishKind::Gravy => "put the gravy",
            DishKind::Caramel => "melt the sugar for caramel",
            DishKind::GreenBeans => "boil the green beans",
            DishKind::Potatoes => "boil the potatoes",
            DishKind::Cider => "warm the cider",
            DishKind::Rolls => "slide the dinner rolls",
            DishKind::Stuffing => "put the stuffing",
            DishKind::Turkey => "put the turkey",
            DishKind::PumpkinPie => "put the pumpkin pie",
            DishKind::CornCakes => "pour the corn cake batter",
            DishKind::Sprouts => "lay the Brussels sprouts",
            DishKind::Fritters => "drop the apple fritters",
            DishKind::Celery => "lay out the celery and onions",
            DishKind::Herbs => "lay out the herbs",
            DishKind::Apples => "lay out the apples",
            DishKind::Cream => "pour the cream into a bowl",
            DishKind::SweetPotatoes => "slide the sweet potato casserole",
            DishKind::MapleCarrots => "glaze the carrots with maple",
            DishKind::Cornbread => "pour the cornbread batter",
            DishKind::Carving => "bring the turkey to the board for carving",
            DishKind::PecanPie => "put the pecan pie",
        }
    }

    /// The steps after starting it, at speed 1.
    pub fn steps(self) -> Vec<StepSpec> {
        use Action::*;
        match self {
            DishKind::CranberrySauce => vec![step(Stir, 6.0), step(Stir, 6.0), step(TakeOff, 5.0)],
            DishKind::Gravy => vec![
                step(Whisk, 5.0),
                step(Whisk, 5.0),
                step(Whisk, 4.0),
                step(TakeOff, 4.0),
            ],
            DishKind::Caramel => vec![
                quick(Stir, 5.0, 2.5),
                quick(Stir, 4.0, 2.5),
                quick(TakeOff, 3.0, 2.5),
            ],
            DishKind::GreenBeans => vec![step(Stir, 7.0), step(Drain, 6.0)],
            DishKind::Potatoes => vec![step(Stir, 8.0), step(Stir, 5.0), step(Drain, 6.0)],
            DishKind::Cider => vec![step(Stir, 6.0), step(Stir, 6.0), step(TakeOff, 5.0)],
            DishKind::Rolls => vec![step(Rotate, 8.0), step(Pull, 6.0)],
            DishKind::Stuffing => vec![step(Rotate, 9.0), step(Pull, 8.0)],
            DishKind::Turkey => vec![
                step(Baste, 8.0),
                step(Baste, 8.0),
                step(Baste, 7.0),
                step(Pull, 6.0),
            ],
            DishKind::PumpkinPie => vec![step(Rotate, 9.0), step(Pull, 7.0)],
            DishKind::CornCakes => vec![quick(Flip, 4.0, 2.5), quick(Plate, 4.0, 2.5)],
            DishKind::Sprouts => vec![
                quick(Flip, 5.0, 2.5),
                quick(Flip, 4.0, 2.5),
                quick(Plate, 3.0, 2.5),
            ],
            DishKind::Fritters => vec![
                quick(Flip, 4.0, 2.5),
                quick(Flip, 4.0, 2.5),
                quick(Plate, 3.0, 2.5),
            ],
            DishKind::Celery => vec![strokes(Chop, 6, 5.0)],
            DishKind::Herbs => vec![strokes(Chop, 4, 4.0)],
            DishKind::Apples => vec![strokes(Chop, 6, 5.0)],
            DishKind::Cream => vec![strokes(Whisk, 8, 6.0)],
            DishKind::SweetPotatoes => vec![step(Rotate, 8.0), step(Pull, 7.0)],
            DishKind::MapleCarrots => vec![step(Stir, 6.0), step(Stir, 5.0), step(Drain, 5.0)],
            DishKind::Cornbread => vec![quick(Flip, 5.0, 3.0), quick(Plate, 4.0, 3.0)],
            DishKind::Carving => vec![strokes(Chop, 8, 6.0)],
            DishKind::PecanPie => vec![step(Rotate, 9.0), step(Pull, 7.0)],
        }
    }
}

/// How a dish stands right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Its station is still busy with an earlier dish.
    Queued,
    /// Cooking (or, before it starts, waiting for the recipe to call it).
    Cooking,
    /// Needs you now: the best time to act.
    Due,
    /// Overdue: act fast to save it.
    Burning,
    /// Served.
    Done,
}

/// One dish on the menu, and how it is going.
#[derive(Clone, Debug)]
pub struct Dish {
    pub kind: DishKind,
    pub steps: Vec<StepSpec>,
    /// The dish that must leave this station first.
    pub after: Option<usize>,
    /// The step under way: 0 is starting it, then `steps[step - 1]`.
    pub step: usize,
    /// Seconds since the last step was done (or since the meal began, or
    /// since its station came free).
    pub clock: f64,
    /// Strokes done on a many-stroke step.
    pub strokes: u32,
    /// Each finished step's timing score, 0 to 100.
    pub scores: Vec<u32>,
    pub fumbles: u32,
    /// What each fumble costs this dish: `FUMBLE_COST`, unless a challenge
    /// says otherwise.
    pub fumble_cost: u32,
    pub ruined: u32,
    pub stage: Stage,
}

impl Dish {
    fn new(kind: DishKind, start_at: f64, after: Option<usize>, speed: f64) -> Dish {
        let scale = |s: StepSpec| StepSpec {
            wait: s.wait * speed,
            window: (s.window * speed).max(MIN_WINDOW),
            burn: (s.burn * speed).max(MIN_WINDOW),
            ..s
        };
        let mut steps = vec![scale(StepSpec {
            action: Action::Start,
            wait: start_at,
            window: START_WINDOW,
            burn: START_BURN,
            strokes: 1,
        })];
        steps.extend(kind.steps().into_iter().map(scale));
        Dish {
            kind,
            steps,
            after,
            step: 0,
            clock: 0.0,
            strokes: 0,
            scores: Vec::new(),
            fumbles: 0,
            fumble_cost: FUMBLE_COST,
            ruined: 0,
            stage: if after.is_some() {
                Stage::Queued
            } else {
                Stage::Cooking
            },
        }
    }

    pub fn name(&self) -> &'static str {
        self.kind.name()
    }

    pub fn station(&self) -> Station {
        self.kind.station()
    }

    pub fn done(&self) -> bool {
        self.stage == Stage::Done
    }

    pub fn started(&self) -> bool {
        self.step > 0
    }

    /// The step under way, if the dish is not done.
    pub fn current(&self) -> Option<StepSpec> {
        self.steps.get(self.step).copied()
    }

    /// Steps after starting, for "step 2 of 3".
    pub fn cooking_steps(&self) -> usize {
        self.steps.len() - 1
    }

    /// Seconds until the current step is due (negative once it is).
    pub fn due_in(&self) -> f64 {
        self.current().map_or(0.0, |s| s.wait - self.clock)
    }

    fn stage_now(&self) -> Stage {
        let Some(s) = self.current() else {
            return Stage::Done;
        };
        if self.clock < s.wait {
            Stage::Cooking
        } else if self.clock < s.wait + s.window {
            Stage::Due
        } else {
            Stage::Burning
        }
    }

    fn overdue(&self) -> bool {
        self.current()
            .is_some_and(|s| self.clock >= s.wait + s.window + s.burn)
    }

    /// The dish's quality: the mean of its step scores, less fumbles.
    pub fn quality(&self) -> u32 {
        if self.scores.is_empty() {
            return 0;
        }
        let mean = self.scores.iter().sum::<u32>() as f64 / self.scores.len() as f64;
        let q = mean.round() as i64 - (self.fumbles * self.fumble_cost) as i64;
        q.clamp(0, 100) as u32
    }

    fn advance(&mut self) {
        self.step += 1;
        self.clock = 0.0;
        self.strokes = 0;
        self.stage = if self.step >= self.steps.len() {
            Stage::Done
        } else {
            Stage::Cooking
        };
    }
}

/// How well a step was timed, from its score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade {
    Perfect,
    Good,
    Late,
    Ruined,
}

impl Grade {
    pub fn for_score(score: u32) -> Grade {
        match score {
            90.. => Grade::Perfect,
            60.. => Grade::Good,
            1.. => Grade::Late,
            0 => Grade::Ruined,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Grade::Perfect => "Perfect.",
            Grade::Good => "Good.",
            Grade::Late => "Late, but saved.",
            Grade::Ruined => "Ruined.",
        }
    }
}

/// The score for acting `late` seconds after a step came due: full marks in
/// the first 40 percent of the window, sliding to 60 at its end, then from
/// 40 to 20 while it burns.
pub fn timing_score(late: f64, window: f64, burn: f64) -> u32 {
    let window = window.max(0.01);
    let burn = burn.max(0.01);
    let sweet = window * 0.4;
    let score = if late <= sweet {
        100.0
    } else if late <= window {
        100.0 - 40.0 * (late - sweet) / (window - sweet)
    } else if late <= window + burn {
        40.0 - 20.0 * (late - window) / burn
    } else {
        20.0
    };
    score.round().clamp(0.0, 100.0) as u32
}

/// Something that happened in the kitchen, for the screen to voice.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The recipe calls for a dish to be started.
    Called(usize),
    /// A started dish needs you: its sound changed.
    Due(usize),
    /// A step is overdue: the dish is burning (or the call is going stale).
    Burning(usize),
    /// A step ran out; the helper did it, for no points.
    Ruined { dish: usize, action: Action },
    /// The player did a step.
    Done {
        dish: usize,
        action: Action,
        score: u32,
    },
    /// One stroke of a many-stroke step.
    Stroke { dish: usize, done: u32, of: u32 },
    /// Acted before the dish was ready.
    Fumble { dish: usize, action: Action },
    /// A dish is finished and served.
    Served(usize),
    /// Nothing to do at that station.
    Empty(Station),
    /// Every dish is served.
    MealOver,
}

/// A dish on a course's menu: what, when the recipe calls it, and which
/// earlier dish must leave its station first.
#[derive(Clone, Copy, Debug)]
pub struct Plan {
    pub kind: DishKind,
    pub start_at: f64,
    pub after: Option<usize>,
}

const fn at(kind: DishKind, start_at: f64) -> Plan {
    Plan {
        kind,
        start_at,
        after: None,
    }
}

const fn then(kind: DishKind, start_at: f64, after: usize) -> Plan {
    Plan {
        kind,
        start_at,
        after: Some(after),
    }
}

/// One course of the campaign.
#[derive(Clone, Debug)]
pub struct Course {
    pub number: u32,
    pub name: &'static str,
    pub blurb: &'static str,
    pub speed: f64,
    pub plan: Vec<Plan>,
}

/// Course `n`, 1 to 8.
pub fn course(n: u32) -> Course {
    use DishKind::*;
    let n = n.clamp(1, COURSES);
    let (name, blurb, speed, plan) = match n {
        1 => (
            "Cranberry sauce",
            "One saucepan on the left burner, to learn the kitchen.",
            1.0,
            vec![at(CranberrySauce, 1.0)],
        ),
        2 => (
            "Rolls and cranberries",
            "The saucepan and the oven together.",
            1.0,
            vec![at(CranberrySauce, 1.0), at(Rolls, 4.0)],
        ),
        3 => (
            "The side dishes",
            "Three stations at once.",
            0.95,
            vec![
                at(Celery, 1.0),
                at(GreenBeans, 4.0),
                at(CranberrySauce, 7.0),
            ],
        ),
        4 => (
            "Stuffing and gravy",
            "Four stations, and the gravy wants whisking often.",
            0.9,
            vec![
                at(Herbs, 1.0),
                at(Stuffing, 3.0),
                at(Gravy, 6.0),
                at(Potatoes, 9.0),
            ],
        ),
        5 => (
            "Off the griddle",
            "The griddle joins in: corn cakes flip fast.",
            0.85,
            vec![
                at(CornCakes, 1.0),
                at(Gravy, 3.0),
                at(GreenBeans, 6.0),
                at(Rolls, 8.0),
            ],
        ),
        6 => (
            "The turkey",
            "All five stations, with the turkey to baste three times.",
            0.8,
            vec![
                at(Turkey, 1.0),
                at(Celery, 3.0),
                at(Potatoes, 6.0),
                at(Sprouts, 9.0),
                at(Gravy, 12.0),
            ],
        ),
        7 => (
            "Dessert",
            "Pie, caramel, cider, fritters and cream, quicker still.",
            0.75,
            vec![
                at(PumpkinPie, 1.0),
                at(Apples, 3.0),
                at(Caramel, 6.0),
                at(Cider, 8.0),
                at(Fritters, 11.0),
            ],
        ),
        _ => (
            "The full feast",
            "Eight dishes on five stations, as fast as it gets.",
            0.7,
            vec![
                at(Turkey, 1.0),
                at(Celery, 2.0),
                at(Gravy, 5.0),
                at(Potatoes, 7.0),
                at(CornCakes, 9.0),
                then(Rolls, 2.0, 0),
                then(Cream, 3.0, 1),
                then(CranberrySauce, 2.0, 2),
            ],
        ),
    };
    Course {
        number: n,
        name,
        blurb,
        speed,
        plan,
    }
}

/// A second-helpings round: a random dish on each of three to five
/// stations, called in a random order, faster every round.
pub fn helping(round: u32, rng: &mut Rng) -> Course {
    let round = round.max(1);
    let count = (2 + round).min(5) as usize;
    let mut stations = Station::ALL.to_vec();
    rng.shuffle(&mut stations);
    stations.truncate(count);
    let mut start = 1.0;
    let plan = stations
        .iter()
        .map(|&station| {
            let pool: Vec<DishKind> = DishKind::ALL
                .iter()
                .copied()
                .filter(|k| k.station() == station)
                .collect();
            let kind = *rng.pick(&pool);
            let p = at(kind, start);
            start += 2.0 + rng.below(3) as f64;
            p
        })
        .collect();
    Course {
        number: 0,
        name: "Second helpings",
        blurb: "A random meal, faster every round.",
        speed: (0.9 - 0.05 * round as f64).max(0.5),
        plan,
    }
}

/// A meal being cooked.
#[derive(Clone, Debug)]
pub struct Meal {
    pub dishes: Vec<Dish>,
    pub elapsed: f64,
    pub over: bool,
    /// No helper steps in: a burning dish waits for the player. The
    /// tutorial cooks this way, so only the player can finish it.
    pub patient: bool,
}

impl Meal {
    pub fn new(course: &Course) -> Meal {
        let dishes = course
            .plan
            .iter()
            .map(|p| Dish::new(p.kind, p.start_at * course.speed, p.after, course.speed))
            .collect();
        Meal {
            dishes,
            elapsed: 0.0,
            over: false,
            patient: false,
        }
    }

    /// The stations this meal uses, left to right.
    pub fn stations(&self) -> Vec<Station> {
        let mut s: Vec<Station> = self.dishes.iter().map(|d| d.station()).collect();
        s.sort();
        s.dedup();
        s
    }

    pub fn uses(&self, station: Station) -> bool {
        self.dishes.iter().any(|d| d.station() == station)
    }

    fn blocked(&self, i: usize) -> bool {
        self.dishes[i]
            .after
            .is_some_and(|j| self.dishes.get(j).is_some_and(|d| !d.done()))
    }

    /// The dish now at `station`: the first one there not served and not
    /// waiting for the station to come free.
    pub fn active_at(&self, station: Station) -> Option<usize> {
        (0..self.dishes.len()).find(|&i| {
            let d = &self.dishes[i];
            d.station() == station && !d.done() && !self.blocked(i)
        })
    }

    /// A dish still to come at `station` once the one there is served.
    pub fn queued_at(&self, station: Station) -> Option<usize> {
        (0..self.dishes.len()).find(|&i| {
            let d = &self.dishes[i];
            d.station() == station && !d.done() && self.blocked(i)
        })
    }

    pub fn served(&self) -> usize {
        self.dishes.iter().filter(|d| d.done()).count()
    }

    /// Runs the kitchen for `dt` seconds.
    pub fn update(&mut self, dt: f64) -> Vec<Event> {
        let mut events = Vec::new();
        if self.over {
            return events;
        }
        self.elapsed += dt;
        let mut needing = self
            .dishes
            .iter()
            .enumerate()
            .filter(|&(i, d)| !d.done() && !self.blocked(i))
            .filter(|(_, d)| matches!(d.stage, Stage::Due | Stage::Burning))
            .count();
        for i in 0..self.dishes.len() {
            if self.dishes[i].done() {
                continue;
            }
            if self.blocked(i) {
                self.dishes[i].stage = Stage::Queued;
                continue;
            }
            let d = &mut self.dishes[i];
            if d.stage == Stage::Queued {
                d.stage = Stage::Cooking;
            } else {
                d.clock += dt;
            }
            // Hands full: a dish about to come due waits its turn.
            if d.stage == Stage::Cooking && needing >= MAX_NEEDING {
                if let Some(s) = d.current() {
                    d.clock = d.clock.min(s.wait - 1e-6);
                }
            }
            if d.overdue() && !self.patient {
                let action = d.current().map_or(Action::Start, |s| s.action);
                d.scores.push(0);
                d.ruined += 1;
                d.advance();
                events.push(Event::Ruined { dish: i, action });
                if d.done() {
                    events.push(Event::Served(i));
                }
                continue;
            }
            let now = d.stage_now();
            if now != d.stage {
                if d.stage == Stage::Cooking && matches!(now, Stage::Due | Stage::Burning) {
                    needing += 1;
                }
                d.stage = now;
                match now {
                    Stage::Due if d.step == 0 => events.push(Event::Called(i)),
                    Stage::Due => events.push(Event::Due(i)),
                    Stage::Burning => events.push(Event::Burning(i)),
                    _ => {}
                }
            }
        }
        if self.dishes.iter().all(Dish::done) {
            self.over = true;
            events.push(Event::MealOver);
        }
        events
    }

    /// The main action at `station`.
    pub fn act(&mut self, station: Station) -> Vec<Event> {
        let mut events = Vec::new();
        if self.over {
            return events;
        }
        let Some(i) = self.active_at(station) else {
            events.push(Event::Empty(station));
            return events;
        };
        let d = &mut self.dishes[i];
        let Some(s) = d.current() else {
            events.push(Event::Empty(station));
            return events;
        };
        if d.clock < s.wait - EARLY_GRACE {
            d.fumbles += 1;
            events.push(Event::Fumble {
                dish: i,
                action: s.action,
            });
            return events;
        }
        d.strokes += 1;
        if d.strokes < s.strokes {
            events.push(Event::Stroke {
                dish: i,
                done: d.strokes,
                of: s.strokes,
            });
            return events;
        }
        let score = timing_score(d.clock - s.wait, s.window, s.burn);
        d.scores.push(score);
        d.advance();
        events.push(Event::Done {
            dish: i,
            action: s.action,
            score,
        });
        if d.done() {
            events.push(Event::Served(i));
        }
        if self.dishes.iter().all(Dish::done) {
            self.over = true;
            events.push(Event::MealOver);
        }
        events
    }

    /// The dish that needs you most: the most overdue one burning, else
    /// the longest due.
    pub fn most_urgent(&self) -> Option<usize> {
        (0..self.dishes.len())
            .filter(|&i| matches!(self.dishes[i].stage, Stage::Due | Stage::Burning))
            .max_by(|&a, &b| {
                let late = |i: usize| -self.dishes[i].due_in();
                late(a).total_cmp(&late(b))
            })
    }

    /// The next dish to need you, and in how many seconds.
    pub fn next_up(&self) -> Option<(usize, f64)> {
        (0..self.dishes.len())
            .filter(|&i| self.dishes[i].stage == Stage::Cooking)
            .map(|i| (i, self.dishes[i].due_in()))
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// The meal's outcome so far.
    pub fn outcome(&self) -> Outcome {
        let qualities: Vec<(DishKind, u32)> =
            self.dishes.iter().map(|d| (d.kind, d.quality())).collect();
        let total: u32 = qualities.iter().map(|q| q.1).sum();
        let average = if qualities.is_empty() {
            0
        } else {
            (total as f64 / qualities.len() as f64).round() as u32
        };
        Outcome {
            stars: stars_for(average),
            qualities,
            average,
            fumbles: self.dishes.iter().map(|d| d.fumbles).sum(),
            ruined: self.dishes.iter().map(|d| d.ruined).sum(),
            seconds: self.elapsed,
        }
    }
}

/// Stars for an average dish quality: 3 at 90, 2 at 75, 1 at 55.
pub fn stars_for(average: u32) -> u32 {
    match average {
        90.. => 3,
        75.. => 2,
        55.. => 1,
        _ => 0,
    }
}

/// How a meal went.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    pub qualities: Vec<(DishKind, u32)>,
    pub average: u32,
    pub stars: u32,
    pub fumbles: u32,
    pub ruined: u32,
    pub seconds: f64,
}

impl Outcome {
    pub fn passed(&self) -> bool {
        self.stars >= 1
    }

    /// Quality times ten, and ten percent more cooked by ear alone.
    pub fn points(&self, by_ear: bool) -> u64 {
        let base: u64 = self.qualities.iter().map(|q| q.1 as u64 * 10).sum();
        if by_ear {
            base + base / 10
        } else {
            base
        }
    }
}

/// What the kitchen says about a dish's stage change.
pub fn called_line(kind: DishKind) -> String {
    format!("Recipe: {} {}.", kind.start_phrase(), kind.station().on())
}

pub fn due_line(d: &Dish) -> String {
    let Some(s) = d.current() else {
        return String::new();
    };
    let plural = d.kind.plural();
    let command = if s.strokes > 1 {
        format!("{}: {} strokes", s.action.command(plural), s.strokes)
    } else {
        s.action.command(plural)
    };
    format!("{} {command}.", d.station().due_line(d.name()))
}

pub fn burning_line(d: &Dish) -> String {
    if d.step == 0 {
        format!(
            "Hurry: the recipe called for the {} {}.",
            d.name(),
            d.station().on()
        )
    } else {
        d.station().burning_line(d.name())
    }
}

pub fn ruined_line(d: &Dish, action: Action) -> String {
    if action == Action::Start {
        format!(
            "Someone started the {} for you. No points for that step.",
            d.name()
        )
    } else {
        format!(
            "The {} {}. Someone rescued {}, for no points.",
            d.name(),
            d.station().ruined_word(),
            d.kind.it()
        )
    }
}

/// "Gravy, simmering, next: whisk." What the station in front of you holds.
pub fn describe_station(meal: &Meal, station: Station) -> String {
    if !meal.uses(station) {
        return "not in use this course".to_string();
    }
    let Some(i) = meal.active_at(station) else {
        return "everything here is served".to_string();
    };
    let d = &meal.dishes[i];
    let Some(s) = d.current() else {
        return format!("{}, served", d.name());
    };
    if d.step == 0 {
        return match d.stage {
            Stage::Due => format!("{}, start now", d.name()),
            Stage::Burning => format!("{}, late: start now", d.name()),
            _ => format!("{}, not called for yet", d.name()),
        };
    }
    let (plural, needs) = (d.kind.plural(), d.kind.needs());
    match d.stage {
        Stage::Due => format!("{}, {needs} {} now", d.name(), s.action.gerund(plural)),
        Stage::Burning => format!("{}, burning, {needs} {}", d.name(), s.action.gerund(plural)),
        _ => format!("{}, cooking, next: {}", d.name(), s.action.verb(plural)),
    }
}

/// "Gravy: step 2 of 4, cooking, next whisk in about 3 seconds."
pub fn detail(meal: &Meal, station: Station) -> String {
    if !meal.uses(station) {
        return format!("The {} is not in use this course.", station.name());
    }
    let Some(i) = meal.active_at(station) else {
        return format!("Everything {} is served.", station.on());
    };
    let d = &meal.dishes[i];
    let Some(s) = d.current() else {
        return format!("The {} {} served.", d.name(), d.kind.is());
    };
    let queued = meal
        .queued_at(station)
        .map(|j| format!(" After {}, the {}.", d.kind.it(), meal.dishes[j].name()))
        .unwrap_or_default();
    let quality = if d.scores.is_empty() && d.fumbles == 0 {
        String::new()
    } else {
        format!(" Quality so far {}.", d.quality())
    };
    let head = format!("{}, {}.", sentence(d.name()), station.on());
    let body = if d.step == 0 {
        match d.stage {
            Stage::Due | Stage::Burning => {
                format!(
                    " The recipe calls for {}: {} {}.",
                    d.kind.it(),
                    d.kind.start_phrase(),
                    station.on()
                )
            }
            _ => format!(
                " Not started; wait for the recipe to call for {}.",
                d.kind.it()
            ),
        }
    } else {
        let of = d.cooking_steps();
        let when = match d.stage {
            Stage::Due => "now".to_string(),
            Stage::Burning => format!("now, {} {} burning", d.kind.it_subject(), d.kind.is()),
            _ => format!("in about {}", seconds(d.due_in())),
        };
        let strokes = if s.strokes > 1 {
            format!(", {} strokes, {} done", s.strokes, d.strokes)
        } else {
            String::new()
        };
        format!(
            " Step {} of {of}: {}{strokes}, {when}.",
            d.step,
            s.action.verb(d.kind.plural())
        )
    };
    format!("{head}{body}{quality}{queued}")
}

/// "3 seconds", "1 second".
pub fn seconds(s: f64) -> String {
    let n = s.max(0.0).round() as i64;
    if n == 1 {
        "1 second".to_string()
    } else {
        format!("{n} seconds")
    }
}

/// Capitalizes the first letter.
pub fn sentence(text: &str) -> String {
    let mut c = text.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// "1 star", "3 stars".
pub fn stars_phrase(n: u32) -> String {
    match n {
        0 => "no stars".to_string(),
        1 => "1 star".to_string(),
        n => format!("{n} stars"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plural_dish_is_them_not_it() {
        assert_eq!(Action::Pull.command(true), "Pull them out");
        assert_eq!(Action::Drain.command(true), "Drain them");
        assert_eq!(Action::Rotate.gerund(true), "their tray turned");
        assert_eq!(Action::TakeOff.verb(true), "take them off the heat");
        assert_eq!(Action::Pull.command(false), "Pull it out");
        assert_eq!(Action::Rotate.gerund(false), "its tray turned");
        assert!(DishKind::Rolls.plural());
        assert_eq!(DishKind::Celery.needs(), "need");
        assert_eq!(DishKind::Gravy.is(), "is");
    }

    #[test]
    fn timing_slides_from_perfect_to_late() {
        assert_eq!(timing_score(-0.2, 3.0, 3.0), 100);
        assert_eq!(timing_score(1.2, 3.0, 3.0), 100);
        assert_eq!(timing_score(3.0, 3.0, 3.0), 60);
        assert_eq!(timing_score(6.0, 3.0, 3.0), 20);
        assert!(timing_score(2.0, 3.0, 3.0) < 100);
    }

    #[test]
    fn perfect_steps_keep_quality_when_lesson_skips_fumble_charges() {
        // Mirrors the tutorial: Perfect timings must not read as quality 0
        // just because Space was mashed early while a prompt held the clock.
        let mut meal = Meal::new(&course(1));
        meal.patient = true;
        let station = Station::LeftBurner;
        // Start when due.
        {
            let d = &mut meal.dishes[0];
            d.clock = d.current().unwrap().wait;
        }
        meal.act(station);
        assert_eq!(Grade::for_score(*meal.dishes[0].scores.last().unwrap()), Grade::Perfect);
        // Two stirs + take off, each on time.
        for _ in 0..3 {
            let d = &mut meal.dishes[0];
            d.clock = d.current().unwrap().wait;
            meal.act(station);
        }
        assert!(meal.dishes[0].done());
        assert_eq!(meal.dishes[0].quality(), 100);
        assert_eq!(Grade::for_score(*meal.dishes[0].scores.last().unwrap()), Grade::Perfect);
    }

    #[test]
    fn fumbles_can_zero_quality_even_after_perfect_steps() {
        let mut meal = Meal::new(&course(1));
        let station = Station::LeftBurner;
        // Stack early presses before the call.
        for _ in 0..25 {
            meal.act(station);
        }
        assert!(meal.dishes[0].fumbles >= 20);
        {
            let d = &mut meal.dishes[0];
            d.clock = d.current().unwrap().wait;
        }
        meal.act(station);
        for _ in 0..3 {
            let d = &mut meal.dishes[0];
            d.clock = d.current().unwrap().wait;
            meal.act(station);
        }
        assert!(meal.dishes[0].done());
        assert_eq!(Grade::for_score(*meal.dishes[0].scores.last().unwrap()), Grade::Perfect);
        assert_eq!(meal.dishes[0].quality(), 0);
    }


    #[test]
    fn every_course_has_one_dish_per_station_at_a_time() {
        for n in 1..=COURSES {
            let c = course(n);
            for (i, p) in c.plan.iter().enumerate() {
                let earlier = c.plan[..i]
                    .iter()
                    .enumerate()
                    .filter(|(_, q)| q.kind.station() == p.kind.station())
                    .count();
                if earlier > 0 {
                    let after = p.after.expect("a second dish at a station waits");
                    assert_eq!(c.plan[after].kind.station(), p.kind.station());
                }
            }
        }
    }
}
