//! The menus opened from the hamlet: seeds, a building site, the barge,
//! the person at the gate, the job board and supper. Each works on the
//! shared hamlet and closes back to it, which then says what happened.
//! Also the sound check.

use std::cell::RefCell;
use std::rc::Rc;

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Vec3};
use parlor_core::{Pages, Parlor};

use crate::buildings::{good, Building, GOODS};
use crate::people::{Skill, INSTRUMENTS};
use crate::rules::{sentence_case, Caller, Hamlet, Job};
use crate::sound;

pub type Shared = Rc<RefCell<Hamlet>>;

macro_rules! menu_impl {
    ($t:ty, $name:expr) => {
        fn name(&self) -> &'static str {
            $name
        }
        fn menu(&self) -> &MenuCore<Self, Parlor> {
            &self.menu
        }
        fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
            &mut self.menu
        }
    };
}

// --- seeds -------------------------------------------------------------------

/// Which seed to sow in the dug plot you stand on.
pub struct SeedMenu {
    menu: MenuCore<Self, Parlor>,
    h: Shared,
}

impl SeedMenu {
    pub fn new(h: Shared) -> SeedMenu {
        SeedMenu {
            menu: MenuCore::new("Sow which seed?").with_intro_help(
                "Up and Down choose a seed, Enter sows it, Escape leaves the plot as it is.",
            ),
            h,
        }
    }
}

impl Menu for SeedMenu {
    type Game = Parlor;
    menu_impl!(SeedMenu, "larkfield_seeds");

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let h = self.h.borrow();
        let season = h.season();
        h.seed_list()
            .into_iter()
            .map(|(c, n)| {
                let when = if c.season == season {
                    format!("ripe in {} days", c.days)
                } else {
                    format!("a {} crop, slow now", c.season.name())
                };
                let again = if c.again.is_some() {
                    ", crops again"
                } else {
                    ""
                };
                let id = c.id;
                MenuItem::new(
                    format!(
                        "{}, {n} {}, {when}{again}",
                        sentence_case(c.name),
                        if n == 1 { "seed" } else { "seeds" }
                    ),
                    move |s: &mut SeedMenu, ctx: &mut Ctx<Parlor>| {
                        s.h.borrow_mut().sow(id);
                        ctx.pop();
                    },
                )
                .select_sound(None)
            })
            .collect()
    }
}

// --- a building site -------------------------------------------------------

/// A free building spot: what goes here, what it costs, and Build.
pub struct BuildMenu {
    menu: MenuCore<Self, Parlor>,
    h: Shared,
    b: Building,
}

impl BuildMenu {
    pub fn new(h: Shared, b: Building) -> BuildMenu {
        let (wood, stone, coins) = b.cost();
        let mut parts = Vec::new();
        if wood > 0 {
            parts.push(format!("{wood} wood"));
        }
        if stone > 0 {
            parts.push(format!("{stone} stone"));
        }
        match coins {
            0 => {}
            1 => parts.push("1 coin".to_string()),
            n => parts.push(format!("{n} coins")),
        }
        let title = format!(
            "A free spot for {}. {} It takes {}.",
            b.name(),
            b.about(),
            crate::rules::list_words(&parts)
        );
        BuildMenu {
            menu: MenuCore::new(title)
                .with_intro_help("Up and Down choose, Enter picks, Escape leaves the spot empty."),
            h,
            b,
        }
    }
}

impl Menu for BuildMenu {
    type Game = Parlor;
    menu_impl!(BuildMenu, "larkfield_build");

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let b = self.b;
        let first = match self.h.borrow().can_build(b) {
            Ok(()) => MenuItem::new(
                format!("Build {}", b.name()),
                move |s: &mut BuildMenu, ctx: &mut Ctx<Parlor>| {
                    s.h.borrow_mut().start_building(b);
                    ctx.pop();
                },
            )
            .help("The materials go now, and settlers on building jobs put it up."),
            Err(why) => MenuItem::new(
                format!("Not yet. {}", why.trim_end_matches('.')),
                |_s: &mut BuildMenu, ctx: &mut Ctx<Parlor>| {
                    ctx.say("Wood comes from the woods, stone from the quarry, and coins from the barge.")
                },
            )
            .select_sound(None),
        };
        vec![
            first,
            MenuItem::new(
                "Leave it for now",
                |_s: &mut BuildMenu, ctx: &mut Ctx<Parlor>| ctx.pop(),
            ),
        ]
    }
}

// --- the barge -----------------------------------------------------------------

/// The barge at the dock: sell from the larder, buy seeds and animals.
pub struct BargeMenu {
    menu: MenuCore<Self, Parlor>,
    h: Shared,
}

impl BargeMenu {
    pub fn new(h: Shared) -> BargeMenu {
        let coins = h.borrow().coins;
        BargeMenu {
            menu: MenuCore::new(format!("The barge. You have {coins} coins.")).with_intro_help(
                "Up and Down choose, Enter buys or opens, Escape leaves the barge.",
            ),
            h,
        }
    }
}

impl Menu for BargeMenu {
    type Game = Parlor;
    menu_impl!(BargeMenu, "larkfield_barge");

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let h = self.h.borrow();
        let fair = if h.fair_day {
            ", a coin more today"
        } else {
            ""
        };
        let mut rows = vec![MenuItem::new(
            format!("Sell from the larder{fair}"),
            |s: &mut BargeMenu, ctx: &mut Ctx<Parlor>| ctx.push(SellMenu::new(s.h.clone())),
        )
        .help("Everything in the larder the barge will buy, and what it pays.")];
        for (id, what, price) in h.barge_stock() {
            rows.push(
                MenuItem::new(
                    format!(
                        "Buy {what}, {price} {}",
                        if price == 1 { "coin" } else { "coins" }
                    ),
                    move |s: &mut BargeMenu, ctx: &mut Ctx<Parlor>| {
                        let line = match s.h.borrow_mut().buy(&id) {
                            Ok(line) | Err(line) => line,
                        };
                        s.refresh(ctx, true);
                        ctx.say(line);
                    },
                )
                .select_sound(None),
            );
        }
        rows.push(MenuItem::new(
            "Done",
            |_s: &mut BargeMenu, ctx: &mut Ctx<Parlor>| ctx.pop(),
        ));
        rows
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        let coins = self.h.borrow().coins;
        self.menu.title = format!("The barge. You have {coins} coins.");
        self.refresh(ctx, true);
        self.announce(ctx);
    }
}

/// Selling to the barge.
pub struct SellMenu {
    menu: MenuCore<Self, Parlor>,
    h: Shared,
}

impl SellMenu {
    pub fn new(h: Shared) -> SellMenu {
        SellMenu {
            menu: MenuCore::new("Sell").with_intro_help(
                "Up and Down choose a good, Enter sells half of it, Escape goes back to the barge.",
            ),
            h,
        }
    }
}

impl Menu for SellMenu {
    type Game = Parlor;
    menu_impl!(SellMenu, "larkfield_sell");

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let h = self.h.borrow();
        let bonus = u32::from(h.fair_day);
        let mut rows = Vec::new();
        for g in GOODS.iter() {
            let n = h.has(g.id);
            if n == 0 || g.sells == 0 {
                continue;
            }
            let price = g.sells + bonus;
            let id = g.id;
            rows.push(MenuItem::new(
                format!(
                    "{}, {n}, {price} {} each",
                    sentence_case(g.name),
                    if price == 1 { "coin" } else { "coins" }
                ),
                move |s: &mut SellMenu, ctx: &mut Ctx<Parlor>| {
                    let sold = {
                        let mut h = s.h.borrow_mut();
                        let n = h.has(id).div_ceil(2);
                        let mut sold = 0;
                        for _ in 0..n {
                            if h.sell(id) {
                                sold += 1;
                            }
                        }
                        (sold, h.coins)
                    };
                    s.refresh(ctx, true);
                    let name = good(id).map_or(id, |g| g.name);
                    ctx.say(format!("Sold {} {name}. {} coins.", sold.0, sold.1));
                },
            ));
        }
        if rows.is_empty() {
            rows.push(MenuItem::inert("Nothing in the larder the barge will buy"));
        }
        rows
    }
}

// --- the gate -------------------------------------------------------------------

/// Someone at the gate asks to stay.
pub struct ArrivalMenu {
    menu: MenuCore<Self, Parlor>,
    h: Shared,
}

impl ArrivalMenu {
    pub fn new(h: Shared, lead: String) -> ArrivalMenu {
        ArrivalMenu {
            menu: MenuCore::new(lead).with_intro_help(
                "Up and Down choose a reply, Enter gives it, Escape goes back and the caller waits at the gate.",
            ),
            h,
        }
    }
}

impl Menu for ArrivalMenu {
    type Game = Parlor;
    menu_impl!(ArrivalMenu, "larkfield_arrival");

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let name = match &self.h.borrow().caller {
            Some(Caller::Stranger(s)) => s.name.clone(),
            _ => "them".to_string(),
        };
        let mouths = self.h.borrow().mouths() + 1;
        vec![
            MenuItem::new(
                format!("Welcome {name}"),
                |s: &mut ArrivalMenu, ctx: &mut Ctx<Parlor>| {
                    s.h.borrow_mut().answer_arrival(true);
                    ctx.pop();
                },
            )
            .help(format!(
                "{name} moves into a cottage. That makes {mouths} to feed at supper."
            )),
            MenuItem::new("Not now", |s: &mut ArrivalMenu, ctx: &mut Ctx<Parlor>| {
                s.h.borrow_mut().answer_arrival(false);
                ctx.pop();
            })
            .help("They walk on, and someone else may come another day."),
        ]
    }
}

// --- the job board ----------------------------------------------------------------

/// Who does what first. Left and Right change a settler's job.
pub struct JobBoard {
    menu: MenuCore<Self, Parlor>,
    h: Shared,
}

impl JobBoard {
    pub fn new(h: Shared) -> JobBoard {
        JobBoard {
            menu: MenuCore::new("The job board").with_intro_help(
                "Up and Down choose a settler, Left and Right change their first job, Escape goes back to the lane.",
            ),
            h,
        }
    }
}

fn jobs() -> Vec<Job> {
    let mut out = vec![Job::Own];
    out.extend(Skill::ALL.into_iter().map(Job::Skill));
    out
}

impl Menu for JobBoard {
    type Game = Parlor;
    menu_impl!(JobBoard, "larkfield_jobs");

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let h = self.h.borrow();
        if h.settlers.is_empty() {
            return vec![MenuItem::inert("Nobody lives here yet")];
        }
        h.settlers
            .iter()
            .map(|st| {
                let name = st.name.clone();
                let best: Vec<String> = st
                    .best_skills()
                    .iter()
                    .take(2)
                    .map(|k| format!("{} {}", k.name(), st.skill(*k)))
                    .collect();
                let label = format!("{}, first job: {}. Best at {}", st.name, st.job.name(), best.join(" and "));
                let n1 = name.clone();
                let n2 = name.clone();
                MenuItem::new(label, move |s: &mut JobBoard, ctx: &mut Ctx<Parlor>| {
                    s.change(ctx, &n1, 1);
                })
                .adjustable(move |s: &mut JobBoard, ctx: &mut Ctx<Parlor>, d| {
                    s.change(ctx, &n2, d);
                })
                .select_sound(None)
                .help("Their first choice of work. When there is none of it, they find something else.")
            })
            .collect()
    }
}

/// How good someone is at a job, in words.
fn skill_words(level: u32) -> &'static str {
    match level {
        0 => "new to it",
        1..=3 => "a beginner",
        4..=6 => "handy at it",
        _ => "skilled",
    }
}

impl JobBoard {
    fn change(&mut self, ctx: &mut Ctx<Parlor>, name: &str, d: i32) {
        let all = jobs();
        let line = {
            let mut h = self.h.borrow_mut();
            let Some(st) = h.settlers.iter().find(|s| s.name == name) else {
                return;
            };
            let i = all.iter().position(|j| *j == st.job).unwrap_or(0) as i32;
            let job = all[(i + d).rem_euclid(all.len() as i32) as usize];
            h.set_job(name, job);
            match job {
                Job::Own => format!("{name}: {}.", job.name()),
                Job::Skill(k) => {
                    let level = h.settler(name).map_or(0, |s| s.skill(k));
                    format!("{name}: {}, {}.", job.name(), skill_words(level))
                }
            }
        };
        self.refresh(ctx, true);
        ctx.say(line);
    }
}

// --- supper -------------------------------------------------------------------------

/// The supper bell: where to sit.
pub struct SupperMenu {
    menu: MenuCore<Self, Parlor>,
    h: Shared,
}

impl SupperMenu {
    pub fn new(h: Shared) -> SupperMenu {
        let title = {
            let h = h.borrow();
            if h.festival_today() {
                format!(
                    "{}. Supper is a feast. Who do you sit with?",
                    h.season().festival()
                )
            } else {
                "Supper. Who do you sit with?".to_string()
            }
        };
        SupperMenu {
            menu: MenuCore::new(title).with_intro_help(
                "Up and Down choose a seat, Enter sits down. Sitting with someone cheers them up.",
            ),
            h,
        }
    }

    fn eat(&mut self, ctx: &mut Ctx<Parlor>, seat: Option<String>) {
        let pages = self.h.borrow_mut().supper(seat.as_deref());
        let n = pages.len();
        let mut music = vec![Some(sound::SUPPER_MUSIC)];
        music.resize(n.max(1), None);
        ctx.replace(Pages::new(pages).with_music(music));
    }
}

impl Menu for SupperMenu {
    type Game = Parlor;
    menu_impl!(SupperMenu, "larkfield_supper");

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let h = self.h.borrow();
        let mut rows: Vec<MenuItem<Self, Parlor>> = h
            .settlers
            .iter()
            .filter(|s| !s.away())
            .map(|st| {
                let name = st.name.clone();
                MenuItem::new(
                    format!("Next to {}, {}", st.name, st.mood().name()),
                    move |s: &mut SupperMenu, ctx: &mut Ctx<Parlor>| s.eat(ctx, Some(name.clone())),
                )
            })
            .collect();
        rows.push(MenuItem::new(
            "Anywhere",
            |s: &mut SupperMenu, ctx: &mut Ctx<Parlor>| s.eat(ctx, None),
        ));
        rows
    }

    fn on_escape(&mut self, ctx: &mut Ctx<Parlor>) {
        self.eat(ctx, None);
    }
}

// --- the sound check ------------------------------------------------------------------

/// The sounds that carry news you have to act on, and the settlers'
/// instruments, so you can tell who is playing.
pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
}

impl SoundCheck {
    pub fn new() -> SoundCheck {
        SoundCheck {
            menu: MenuCore::new("Sound check")
                .with_intro_help("Up and Down choose a sound, Enter plays it, Escape goes back."),
        }
    }
}

impl Default for SoundCheck {
    fn default() -> Self {
        SoundCheck::new()
    }
}

fn plain(label: &str, key: impl Into<String>, help: &str) -> MenuItem<SoundCheck, Parlor> {
    let key = key.into();
    MenuItem::new(label, move |_s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        ctx.stop_speech();
        ctx.audio.play(&key, Bus::EFFECTS);
    })
    .select_sound(None)
    .help(help)
}

impl Menu for SoundCheck {
    type Game = Parlor;
    menu_impl!(SoundCheck, "larkfield_sounds");

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        hearken::menu::menu_enter(self, ctx);
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut rows = vec![
            plain("The line goes out", "larkfield/work/cast", "A swish and a plop at the fishing spot. Wait for the bite."),
            plain("A bite", sound::BITE, "A sharp tug and a little bell. Your main action reels it in, but only for a moment."),
            plain("A step on the lane", sound::STEP_LANE, "Gravel underfoot: you are on the lane."),
            plain("A step inside a place", sound::STEP_IN, "Soft grass: you are inside a place."),
            plain("A step in the fields", sound::STEP_FIELD, "Soft earth: you are in the fields, among the plots."),
            plain("The end of the way", sound::EDGE, "A knock on a fence post: the end of the lane, a place or the field."),
            plain("A fire", "larkfield/work/fire", "Crackling and a rung bell. Something is burning: follow the sound and throw water on it."),
            plain("A fox", "larkfield/work/fox", "A sharp yelp at the hen house. Chase the fox off before it takes a hen."),
        ];
        for (i, name) in INSTRUMENTS.iter().enumerate() {
            let key = crate::music::part_key(i, crate::music::Chord::G, 0);
            let order = ["first", "second", "third", "fourth", "fifth", "sixth"][i];
            rows.push(plain(
                &sentence_case(&format!("the {name}")),
                key,
                &format!("The {order} settler to arrive plays the {name}, from wherever they are working."),
            ));
        }
        rows
    }
}
