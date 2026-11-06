//! Daily, weekly and monthly challenges: the same small hamlet for every
//! player in the period, with a goal and a few days to reach it.
//!
//! - Daily, "Market day": fill the barge's order within five days.
//! - Weekly, "Good spirits": three tired newcomers, all cheerful at once
//!   within six days.
//! - Monthly, "Founders": the mill, the bakery and a festival supper in a
//!   season's ten days.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Pages, Parlor, Period, Rng};

use crate::rules::{list_words, Goal, Hamlet, Kind, Order, Plot, COLS, ROWS};
use crate::screen::HamletScreen;
use crate::sound;
use crate::ID;

pub const CHALLENGES_NAME: &str = "larkfield_challenges";

pub fn challenge_name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Market day",
        Period::Weekly => "Good spirits",
        Period::Monthly => "Founders",
    }
}

pub fn goal_for(period: Period) -> Goal {
    match period {
        Period::Daily => Goal::Market,
        Period::Weekly => Goal::Cheer,
        Period::Monthly => Goal::Founders,
    }
}

/// The orders a market day can ask for.
const ORDERS: [&[(&str, u32)]; 4] = [
    &[("fish", 10), ("eggs", 6)],
    &[("turnip", 10), ("fish", 6)],
    &[("radish", 8), ("eggs", 4), ("fish", 4)],
    &[("potato", 6), ("berries", 4), ("fish", 6)],
];

/// The period's hamlet, the same for everyone.
pub fn hamlet(period: Period, seed: u64) -> Hamlet {
    let goal = goal_for(period);
    let mut h = Hamlet::bare(Kind::Challenge(goal), seed);
    // Dug from the lane in: the first row, then two of the second.
    for i in 0..6 {
        h.plots[(ROWS - 1) * COLS - 2 + i] = Plot::Tilled;
    }
    h.add_seeds("turnip", 4);
    h.add_seeds("radish", 4);
    h.larder.insert("bread".into(), 6);
    h.larder.insert("potato".into(), 6);
    h.coins = 10;
    h.hens = 2;
    let mut rng = Rng::seeded(seed);
    match goal {
        Goal::Market => {
            h.day_limit = Some(5);
            let order = ORDERS[rng.below(ORDERS.len() as u32) as usize];
            h.order = Some(Order {
                goods: order.iter().map(|(id, n)| (id.to_string(), *n)).collect(),
                filled: false,
            });
            h.larder.insert("wood".into(), 6);
            h.larder.insert("stone".into(), 4);
            for _ in 0..2 {
                let s = h.make_settler(None);
                h.welcome(s);
            }
        }
        Goal::Cheer => {
            h.day_limit = Some(6);
            h.larder.insert("wood".into(), 14);
            h.larder.insert("stone".into(), 8);
            h.coins = 20;
            for _ in 0..3 {
                let mut s = h.make_settler(None);
                s.think("tired from the road", -0.35, 3);
                s.comfort = 0.3;
                h.welcome(s);
            }
        }
        Goal::Founders => {
            h.day_limit = Some(10);
            h.larder.insert("wood".into(), 20);
            h.larder.insert("stone".into(), 18);
            h.larder.insert("grain".into(), 4);
            h.coins = 20;
            h.add_seeds("wheat", 3);
            for _ in 0..2 {
                let s = h.make_settler(None);
                h.welcome(s);
            }
        }
    }
    h.take_events();
    h.begin_day();
    h
}

/// What the challenge asks.
pub fn challenge_rule(ch: &Challenge) -> String {
    match ch.period {
        Period::Daily => {
            let h = hamlet(Period::Daily, ch.seed);
            let order = h.order.map_or(String::new(), |o| {
                list_words(
                    &o.goods
                        .iter()
                        .map(|(id, n)| crate::buildings::count(id, *n))
                        .collect::<Vec<_>>(),
                )
            });
            format!("The barge wants {order} in the larder within 5 days. Supper leaves the order alone. The sooner it is filled, the more points.")
        }
        Period::Weekly => "Three tired newcomers arrive together. Get all three cheerful at the same supper within 6 days.".to_string(),
        Period::Monthly => "Build the mill and the bakery, then hold the season's festival supper on day 10. Points for everything made and everyone cheerful.".to_string(),
    }
}

/// The pages read before a challenge, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let pages = vec![format!("{}: {}", challenge_name(period), challenge_rule(&ch))];
    Pages::briefing(pages).then(move |ctx| {
        let h = hamlet(ch.period, ch.seed);
        ctx.replace(HamletScreen::new(h, Some(ch.clone())))
    })
}

pub struct Hamlets;

pub type ChallengesMenu = menus::ChallengesMenu<Hamlets>;

impl ChallengeGame for Hamlets {
    const ID: &'static str = ID;
    const NAME: &'static str = "Larkfield";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        challenge_name(period).to_string()
    }

    fn tag(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        match period {
            Period::Daily => "fill the barge's order in 5 days".to_string(),
            Period::Weekly => "everyone cheerful in 6 days".to_string(),
            Period::Monthly => "the mill, the bakery and a festival in 10 days".to_string(),
        }
    }

    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        challenge_rule(&Challenge::current(ctx, ID, period))
    }

    fn open(ctx: &mut Ctx<Parlor>, period: Period) {
        let pages = briefing(ctx, period);
        ctx.push(pages);
    }

    fn music(ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn leave(ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }
}

/// Holds a challenge opened from the launcher, and goes back there after.
pub struct ChallengeHost {
    period: Period,
    opened: bool,
}

impl ChallengeHost {
    pub fn new(period: Period) -> ChallengeHost {
        ChallengeHost { period, opened: false }
    }
}

impl State<Parlor> for ChallengeHost {
    fn name(&self) -> &'static str {
        CHALLENGES_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.opened {
            self.opened = true;
            sound::title_music(ctx);
            let pages = briefing(ctx, self.period);
            ctx.push(pages);
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        ctx.audio.set_bed(None);
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
        "Opening the Larkfield challenge.".to_string()
    }
}
