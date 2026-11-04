//! The bike shop, where dollars from parcels and deliveries buy bikes and districts,
//! and district select, which picks where the endless ride goes.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::text::sentence_start;
use hearken::{Ctx, UiSound};
use parlor_core::menus::labels;
use parlor_core::Parlor;

use crate::course::District;
use crate::progress::{dollar_words, Bought, Choices, Item, Progress};

/// Spend dollars on bikes and districts.
pub struct Shop {
    menu: MenuCore<Self, Parlor>,
}

impl Shop {
    pub fn new() -> Self {
        Shop {
            menu: MenuCore::new("Bike shop")
                .with_intro_help("Up and Down choose, Enter buys, Escape goes back."),
        }
    }
}

impl Default for Shop {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for Shop {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "freewheel_shop"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let progress = Progress::load(ctx);
        let mut items: Vec<MenuItem<Self, Parlor>> = vec![MenuItem::new(
            format!("Cash: {}", dollar_words(progress.cash)),
            |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.say("Earn cash from parcels on endless rides and from deliveries.");
            },
        )
        .help("Earn cash from parcels on endless rides and from deliveries.")];
        for item in Item::all() {
            let owned = progress.owns(item);
            let label = match (owned, item.perk()) {
                (true, _) => format!("{}, owned", item.row_name()),
                (false, "") => format!("{}, {}", item.row_name(), dollar_words(item.price())),
                (false, perk) => format!(
                    "{}, {}: {perk}",
                    item.row_name(),
                    dollar_words(item.price())
                ),
            };
            let kind = match item {
                Item::Bike(_) => "Bike.",
                Item::District(_) => "District for the endless ride.",
            };
            items.push(
                MenuItem::new(label, move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    let mut progress = Progress::load(ctx);
                    let line = match progress.buy(item) {
                        Bought::Done => {
                            ctx.play_ui(UiSound::Select);
                            progress.store(ctx);
                            let mut c = Choices::load(ctx);
                            match item {
                                Item::Bike(b) => c.bike = b,
                                Item::District(d) => c.district = d,
                            }
                            c.store(ctx);
                            let bought = match item {
                                Item::Bike(b) => {
                                    format!("Bought the {}. Riding it now.", b.name())
                                }
                                Item::District(d) => {
                                    format!("Bought {}. Riding there now.", d.name())
                                }
                            };
                            format!(
                                "{bought} {} left.",
                                sentence_start(&dollar_words(progress.cash))
                            )
                        }
                        Bought::Owned => {
                            ctx.play_ui(UiSound::Select);
                            format!("{} is already yours.", item.name())
                        }
                        Bought::Short(n) => {
                            // Not enough cash: the error sound, as a locked
                            // row gives.
                            ctx.play_ui(UiSound::Error);
                            format!("{} more needed.", sentence_start(&dollar_words(n)))
                        }
                    };
                    Menu::resume_with(s, ctx, &line);
                })
                .select_sound(None)
                .help(format!("{kind} {}", item.about())),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Freewheel title."),
        );
        items
    }
}

/// Choose where the endless ride goes.
pub struct DistrictSelect {
    menu: MenuCore<Self, Parlor>,
}

impl DistrictSelect {
    pub fn new() -> Self {
        DistrictSelect {
            menu: MenuCore::new("District select").with_intro_help(
                "Up and Down choose a district, Enter picks it, Escape goes back.",
            ),
        }
    }
}

impl Default for DistrictSelect {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for DistrictSelect {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "freewheel_districts"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let progress = Progress::load(ctx);
        let current = Choices::load(ctx).district;
        let mut items: Vec<MenuItem<Self, Parlor>> = Vec::new();
        for d in District::ALL {
            let owned = progress.owns_district(d);
            let label = match (owned, d == current) {
                (true, true) => format!("{}, riding now", d.name()),
                (true, false) => d.name().to_string(),
                (false, _) => format!("{}, locked", d.name()),
            };
            let help = if owned {
                d.about().to_string()
            } else {
                format!(
                    "{} Buy it in the bike shop for {}.",
                    d.about(),
                    dollar_words(d.price())
                )
            };
            items.push(
                MenuItem::new(label, move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    if !Progress::load(ctx).owns_district(d) {
                        ctx.say(format!(
                            "Locked. Buy it in the bike shop for {}.",
                            dollar_words(d.price())
                        ));
                        return;
                    }
                    let mut c = Choices::load(ctx);
                    c.district = d;
                    c.store(ctx);
                    ctx.pop_with(format!("Endless ride in {}", d.name()));
                })
                .help(help),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Freewheel title."),
        );
        items
    }
}
