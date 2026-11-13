//! The shop where tips become new teas, cups and music, and the pages of
//! keepsakes and the guest book.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::text::sentence_start;
use hearken::{Ctx, UiSound};
use parlor_core::Parlor;

use crate::days::Occasion;
use crate::guests::REGULARS;
use crate::save::House;
use crate::sound;
use crate::talk::coins_words;
use crate::teas::{CUPS, SAND_TIMER_PRICE, TEAS, TUNES};

/// The shop's `State::name`.
pub const SHOP_NAME: &str = "slow_kettle_shop";

/// One thing the shop sells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Tea(usize),
    Cup(usize),
    Tune(usize),
    SandTimer,
}

impl Item {
    /// Everything, teas first.
    pub fn all() -> Vec<Item> {
        let mut out: Vec<Item> = TEAS
            .iter()
            .enumerate()
            .filter(|(_, t)| t.price().is_some())
            .map(|(i, _)| Item::Tea(i))
            .collect();
        out.extend((1..CUPS.len()).map(Item::Cup));
        out.extend((0..TUNES.len()).map(Item::Tune));
        out.push(Item::SandTimer);
        out
    }

    pub fn name(self) -> &'static str {
        match self {
            Item::Tea(i) => TEAS[i].name,
            Item::Cup(i) => CUPS[i].name,
            Item::Tune(0) => "Music box",
            Item::Tune(i) => TUNES[i].name,
            Item::SandTimer => "Sand timer",
        }
    }

    pub fn about(self) -> String {
        match self {
            Item::Tea(i) => format!("{} {}", TEAS[i].about, TEAS[i].needs()),
            Item::Cup(i) => CUPS[i].about.to_string(),
            Item::Tune(i) => TUNES[i].about.to_string(),
            Item::SandTimer => "Marks when a steep reaches what its tea wants.".to_string(),
        }
    }

    pub fn price(self) -> u32 {
        match self {
            Item::Tea(i) => TEAS[i].price().unwrap_or(0),
            Item::Cup(i) => CUPS[i].price,
            Item::Tune(i) => TUNES[i].price,
            Item::SandTimer => SAND_TIMER_PRICE,
        }
    }

    pub fn owned(self, house: &House) -> bool {
        match self {
            Item::Tea(i) => house.owns_tea(TEAS[i].id),
            Item::Cup(i) => house.owns_cup(CUPS[i].id),
            Item::Tune(i) => house.tunes.contains(TUNES[i].id),
            Item::SandTimer => house.sand_timer,
        }
    }

    /// A tune needs the music box first.
    pub fn needs_box(self, house: &House) -> bool {
        matches!(self, Item::Tune(i) if i > 0) && !house.has_music_box()
    }

    /// "Jasmine pearls, 12 coins" or "Jasmine pearls, owned".
    pub fn label(self, house: &House) -> String {
        if self.owned(house) {
            format!("{}, owned", self.name())
        } else {
            format!("{}, {}", self.name(), coins_words(self.price()))
        }
    }
}

/// What buying `item` says, and whether it was bought.
pub fn buy(house: &mut House, item: Item) -> (bool, String) {
    let name = item.name();
    if item.owned(house) {
        return (false, format!("{name} is already yours."));
    }
    if item.needs_box(house) {
        return (
            false,
            format!("{name} is a tune for the music box. Buy the music box first."),
        );
    }
    let price = item.price();
    if house.coins < price {
        return (
            false,
            format!(
                "{name} costs {}. You have {}. Tips from perfect cups add up.",
                coins_words(price),
                coins_words(house.coins)
            ),
        );
    }
    house.coins -= price;
    match item {
        Item::Tea(i) => {
            house.teas.insert(TEAS[i].id.to_string());
        }
        Item::Cup(i) => {
            house.cups.insert(CUPS[i].id.to_string());
        }
        Item::Tune(i) => {
            house.tunes.insert(TUNES[i].id.to_string());
            house.tune = Some(TUNES[i].id.to_string());
        }
        Item::SandTimer => house.sand_timer = true,
    }
    let left = coins_words(house.coins);
    let what = match item {
        Item::Tea(_) => " It is on your shelf now.",
        Item::Cup(_) => " It is in your cupboard now.",
        Item::Tune(_) => " The music box plays it in the tea house.",
        Item::SandTimer => " It marks each steep.",
    };
    (
        true,
        format!("Bought {name}.{what} {} left.", sentence_start(&left)),
    )
}

pub struct Shop {
    menu: MenuCore<Self, Parlor>,
}

impl Shop {
    pub fn new() -> Shop {
        Shop {
            menu: MenuCore::new("Shop").with_intro_help(
                "Up and Down choose, Enter buys, F1 describes it, Escape goes back. Teas come first, then cups, the music box and its tunes, and the sand timer.",
            ),
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
        SHOP_NAME
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        let house = House::load(ctx);
        self.menu.title = format!("Shop. You have {}", coins_words(house.coins));
        parlor_core::shop_music::start(ctx, sound::SHOP_MUSIC);
        hearken::menu::menu_enter(self, ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        parlor_core::shop_music::stop(ctx);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let house = House::load(ctx);
        let mut items: Vec<MenuItem<Self, Parlor>> = Item::all()
            .into_iter()
            .map(|item| {
                let help = item.about();
                MenuItem::new(
                    item.label(&house),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        let mut house = House::load(ctx);
                        let (bought, words) = buy(&mut house, item);
                        if bought {
                            house.store(ctx);
                            sound::play(ctx, sound::BUY);
                            s.menu.title = format!("Shop. You have {}", coins_words(house.coins));
                        } else {
                            sound::play(ctx, sound::NOPE);
                        }
                        s.refresh(ctx, true);
                        ctx.say(words);
                    },
                )
                .select_sound(None)
                .help(help)
            })
            .collect();
        items.push(
            MenuItem::new("Back", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.play_ui(UiSound::Back);
                ctx.pop()
            })
            .help("Return to where you were."),
        );
        items
    }
}

/// The keepsake shelf, as pages.
pub fn keepsake_pages(house: &House) -> Vec<String> {
    let kept = house.occasions_kept();
    let mut first = format!(
        "Keepsakes. {} of {} holidays have left you something.",
        kept.len(),
        Occasion::ALL.len()
    );
    if house.keepsakes.is_empty() {
        first.push_str(" None yet. On certain holidays a special guest comes in, and each one leaves a keepsake.");
    }
    let mut pages = vec![first];
    for k in &house.keepsakes {
        let when = Occasion::by_id(&k.occasion)
            .map(|o| sentence_start(o.name()))
            .unwrap_or_default();
        pages.push(format!(
            "{}, from {when} {}.",
            sentence_start(&k.words),
            k.year
        ));
    }
    let missing: Vec<String> = Occasion::ALL
        .into_iter()
        .filter(|o| !kept.contains(o.id()))
        .map(|o| o.name().to_string())
        .collect();
    if !missing.is_empty() {
        pages.push(format!("Still to come: {}.", missing.join(", ")));
    }
    pages
}

/// The guest book: each regular met, and their story so far.
pub fn guest_book_pages(house: &House) -> Vec<String> {
    let met: Vec<_> = REGULARS
        .iter()
        .filter(|r| house.last_seen.contains_key(r.id))
        .collect();
    let mut pages = vec![format!(
        "Guest book. {} of {} regulars met.",
        met.len(),
        REGULARS.len()
    )];
    if met.is_empty() {
        pages[0].push_str(" Regulars come back with the next part of their story each visit.");
    }
    for r in met {
        let told = house.chapters.get(r.id).copied().unwrap_or(0) as usize;
        let story: Vec<&str> = r.story.iter().take(told).copied().collect();
        let progress = if told >= r.story.len() {
            "Their story is told.".to_string()
        } else {
            format!("Part {told} of {}.", r.story.len())
        };
        pages.push(format!(
            "{}, {}. {progress} {}",
            r.name,
            r.who,
            story.join(" ")
        ));
    }
    pages
}
