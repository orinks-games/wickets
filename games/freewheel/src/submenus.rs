//! The title's three folders: More rides, Events and Garage. Each is its
//! own screen with a name of its own, so the title stays short and a
//! player, a test or an agent always knows which list is open.
//!
//! A ride started from a folder returns to the Freewheel title when it
//! ends, as every ride does.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::Ctx;
use parlor_core::menus::labels;
use parlor_core::{Parlor, Rng};

use crate::course::COURSES;
use crate::progress::{dollar_words, Choices, Progress};
use crate::{clock, events, ride, shop, BikeSelect, CourseSelect, Ride};

/// The More rides folder's screen name.
pub const MORE_RIDES_NAME: &str = "freewheel_more_rides";
/// The Events folder's screen name.
pub const EVENTS_NAME: &str = "freewheel_events";
/// The Garage folder's screen name.
pub const GARAGE_NAME: &str = "freewheel_garage";

/// The title row that opens More rides.
pub const MORE_RIDES: &str = "More rides";
/// The title row that opens Events.
pub const EVENTS: &str = "Events";
/// The title row that opens the Garage.
pub const GARAGE: &str = "Garage";

/// Back from a folder to the title.
fn back<S: Menu<Game = Parlor> + 'static>() -> MenuItem<S, Parlor> {
    MenuItem::new(labels::BACK, |_s: &mut S, ctx: &mut Ctx<Parlor>| ctx.pop())
        .help("Return to the Freewheel title.")
}

/// The events open today, by name.
pub fn open_events(today: parlor_core::Date) -> Vec<&'static str> {
    let mut open = Vec::new();
    if events::leaf_open(today) {
        open.push(events::LEAF_SEASON);
    }
    if events::sunday_course(today).is_some() {
        open.push(events::SUNDAY);
    }
    open
}

/// "Events", "Events, Car-free Sunday open" while one is open, or
/// "Events, 2 open".
pub fn events_label(today: parlor_core::Date) -> String {
    let open = open_events(today);
    match open.as_slice() {
        [] => EVENTS.to_string(),
        [one] => format!("{EVENTS}, {one} open"),
        more => format!("{EVENTS}, {} open", more.len()),
    }
}

/// The time trials, the ride against the clock (alpha) and the free ride.
pub struct MoreRides {
    menu: MenuCore<Self, Parlor>,
}

impl MoreRides {
    pub fn new() -> Self {
        MoreRides {
            menu: MenuCore::new(MORE_RIDES),
        }
    }
}

impl Default for MoreRides {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for MoreRides {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        MORE_RIDES_NAME
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let c = Choices::load(ctx);
        let mut items =
            vec![
                MenuItem::new("Time trial", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.push(CourseSelect::new())
                })
                .help(format!(
                    "Race the clock on one of {} Downtown courses, on the {}. Opens on {}, {}.",
                    COURSES.len(),
                    c.bike.name(),
                    COURSES[c.course].name,
                    crate::best_words(ctx, c.course, c.bike)
                )),
            ];
        if clock::offered() {
            let district = c.district;
            let best = crate::row_best(ctx, &clock::board(district), |n| {
                clock::drop_words(n as u32)
            });
            items.push(
                MenuItem::new(
                    format!("{}: {} on the {}", clock::NAME, district.name(), c.bike.name()),
                    move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        ctx.push(Ride::against_clock(
                            c.bike,
                            district,
                            Rng::from_time().next_u64(),
                        ))
                    },
                )
                .help(format!(
                    "Make drops to add time. Crashes cost {} seconds. The ride ends when the clock runs out.{best}",
                    clock::CRASH_S as u32
                )),
            );
        }
        items.push(
            MenuItem::new("Free ride", move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.push(Ride::free_ride(c.bike, Rng::from_time().next_u64()))
            })
            .help(format!(
                "{length} of Downtown with no clock, to learn the street.",
                length = parlor_core::units::route(ride::FREE_LENGTH)
            )),
        );
        items.push(back());
        items
    }
}

/// The special rides: Leaf season and Car-free Sunday. Each row keeps
/// its place all year, locked on its closed days, and says when it opens.
pub struct Events {
    menu: MenuCore<Self, Parlor>,
}

impl Events {
    pub fn new() -> Self {
        Events {
            menu: MenuCore::new(EVENTS),
        }
    }
}

impl Default for Events {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for Events {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        EVENTS_NAME
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let c = Choices::load(ctx);
        let today = parlor_core::challenges::today(ctx);
        let on = format!(" On the {}.", c.bike.name());
        let mut items = Vec::new();
        if events::leaf_open(today) {
            let help = format!(
                "{}{on}{}",
                events::leaf_help(),
                crate::event_best(ctx, &events::board(events::LEAF_BOARD, c.bike))
            );
            items.push(
                MenuItem::new(
                    events::LEAF_SEASON,
                    move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        let today = parlor_core::challenges::today(ctx);
                        ctx.push(Ride::leaf_season(c.bike, today))
                    },
                )
                .help(help),
            );
        } else {
            items.push(
                MenuItem::new(
                    labels::locked(events::LEAF_SEASON),
                    |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        let today = parlor_core::challenges::today(ctx);
                        ctx.play_ui(hearken::UiSound::Error);
                        ctx.say(events::leaf_locked_line(today))
                    },
                )
                .select_sound(None)
                .help(events::leaf_locked_help(today)),
            );
        }
        if let Some(course) = events::sunday_course(today) {
            let help = format!(
                "{}{on}{}",
                events::sunday_help(),
                crate::event_best(ctx, &events::board(events::SUNDAY_BOARD, c.bike))
            );
            items.push(
                MenuItem::new(
                    events::SUNDAY,
                    move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        ctx.push(Ride::sunday(c.bike, course.clone()))
                    },
                )
                .help(help),
            );
        } else {
            items.push(
                MenuItem::new(
                    labels::locked(events::SUNDAY),
                    |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        let today = parlor_core::challenges::today(ctx);
                        ctx.play_ui(hearken::UiSound::Error);
                        ctx.say(events::sunday_locked_line(today))
                    },
                )
                .select_sound(None)
                .help(events::sunday_locked_help(today)),
            );
        }
        items.push(back());
        items
    }
}

/// Your bike, your district and the bike shop.
pub struct Garage {
    menu: MenuCore<Self, Parlor>,
}

impl Garage {
    pub fn new() -> Self {
        Garage {
            menu: MenuCore::new(GARAGE),
        }
    }
}

impl Default for Garage {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for Garage {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        GARAGE_NAME
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let c = Choices::load(ctx);
        vec![
            MenuItem::new("Bike select", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.push(BikeSelect::new())
            })
            .help(format!("Choose your bike. Riding the {} now.", c.bike.name())),
            MenuItem::new("District select", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.push(shop::DistrictSelect::new())
            })
            .help({
                let name = c.district.name();
                if clock::offered() {
                    format!(
                        "Choose where the endless ride and the ride against the clock go. Riding in {name} now."
                    )
                } else {
                    format!("Choose where the endless ride goes. Riding in {name} now.")
                }
            }),
            MenuItem::new("Bike shop", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.push(shop::Shop::new())
            })
            .help(format!(
                "Spend cash on bikes and districts. You have {}.",
                dollar_words(Progress::load(ctx).cash)
            )),
            back(),
        ]
    }
}

/// The Garage row's help: what you ride and what you have to spend.
pub fn garage_help(ctx: &mut Ctx<Parlor>) -> String {
    let c = Choices::load(ctx);
    format!(
        "Bike select, District select and the bike shop. Riding the {} in {}, with {} to spend.",
        c.bike.name(),
        c.district.name(),
        dollar_words(Progress::load(ctx).cash)
    )
}

/// The Events row's help: each event and whether it is open.
pub fn events_help(today: parlor_core::Date) -> String {
    let leaf = if events::leaf_open(today) {
        format!("{} is open.", events::LEAF_SEASON)
    } else {
        events::leaf_locked_line(today)
    };
    let sunday = if events::sunday_course(today).is_some() {
        format!("{} is open today.", events::SUNDAY)
    } else {
        events::sunday_locked_line(today)
    };
    format!("Special rides on their own days. {leaf} {sunday}")
}

/// The More rides row's help.
pub fn more_rides_help() -> String {
    if clock::offered() {
        "Time trials, the ride against the clock and the free ride.".to_string()
    } else {
        "Time trials and the free ride.".to_string()
    }
}
