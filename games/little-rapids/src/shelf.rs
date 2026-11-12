//! The keepsake shelf: the trinkets you fish out of the streams.
//!
//! Each stream has its own trinkets. A few belong to a set, and finding
//! every piece of a set tells its little story.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::Ctx;
use parlor_core::Parlor;

use crate::progress::Progress;
use crate::rules::Stream;

/// One trinket.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trinket {
    pub id: &'static str,
    /// As it is said when caught: "A brass button."
    pub name: &'static str,
    pub stream: Stream,
    /// Its line on the shelf.
    pub line: &'static str,
    pub set: Option<&'static str>,
}

const fn t(
    id: &'static str,
    name: &'static str,
    stream: Stream,
    line: &'static str,
    set: Option<&'static str>,
) -> Trinket {
    Trinket {
        id,
        name,
        stream,
        line,
        set,
    }
}

use Stream::{GardenBrook as G, MarketCanal as C, MillRace as M, RainGutter as R};

/// Every trinket, stream by stream.
pub const TRINKETS: &[Trinket] = &[
    t(
        "marble",
        "A glass marble",
        G,
        "Green, with a twist of white inside.",
        None,
    ),
    t(
        "thimble",
        "A silver thimble",
        G,
        "Dented on one side, from years of pushing needles.",
        None,
    ),
    t(
        "button_1",
        "A horn button",
        G,
        "Four holes, and a thread still through two of them.",
        Some("coat"),
    ),
    t(
        "acorn_cup",
        "An acorn cup",
        G,
        "Somebody painted it gold.",
        None,
    ),
    t(
        "seed_packet",
        "A seed packet",
        G,
        "Sweet peas. Still a few seeds rattling inside.",
        None,
    ),
    t(
        "comb_1",
        "A music box comb",
        G,
        "A row of tiny steel teeth, each a different note.",
        Some("music box"),
    ),
    t(
        "hair_ribbon",
        "A blue hair ribbon",
        G,
        "Tied in a bow that never came undone.",
        None,
    ),
    t(
        "bottle_cap",
        "A bottle cap",
        R,
        "Ginger beer, from a shop that closed long ago.",
        None,
    ),
    t(
        "key_ribbon",
        "A key with a ribbon",
        R,
        "Too small for a door. Maybe a diary.",
        None,
    ),
    t(
        "button_2",
        "A second horn button",
        R,
        "The same four holes. The same coat, surely.",
        Some("coat"),
    ),
    t(
        "tin_whistle",
        "A tin whistle",
        R,
        "It still plays two notes, if you blow gently.",
        None,
    ),
    t(
        "drum_2",
        "A music box drum",
        R,
        "A brass cylinder covered in pins.",
        Some("music box"),
    ),
    t(
        "postcard",
        "A postcard",
        R,
        "Wish you were here. The rest has washed away.",
        None,
    ),
    t(
        "chess_knight",
        "A wooden chess knight",
        R,
        "Black, with one ear chipped.",
        None,
    ),
    t(
        "ring",
        "A ring with no stone",
        M,
        "Plain gold, worn thin on the inside.",
        None,
    ),
    t(
        "flour_scoop",
        "A tiny flour scoop",
        M,
        "From the mill, a long time ago.",
        None,
    ),
    t(
        "button_3",
        "A third horn button",
        M,
        "Three buttons. The coat must be whole now.",
        Some("coat"),
    ),
    t(
        "key_3",
        "A music box key",
        M,
        "A little brass key, the kind you wind.",
        Some("music box"),
    ),
    t(
        "toy_soldier",
        "A tin soldier",
        M,
        "His drum is painted red and gold.",
        None,
    ),
    t(
        "spinning_top",
        "A spinning top",
        M,
        "It still spins, if you give it a good twist.",
        None,
    ),
    t(
        "lucky_penny",
        "A lucky penny",
        M,
        "Heads up, the way pennies should be found.",
        None,
    ),
    t(
        "pocket_watch",
        "A pocket watch",
        M,
        "Stopped at ten past four.",
        None,
    ),
    t(
        "cork_float",
        "A fishing float",
        C,
        "Red and white, and still bobbing.",
        None,
    ),
    t(
        "lid_4",
        "A music box lid",
        C,
        "Painted with a little boat on a river.",
        Some("music box"),
    ),
    t(
        "teacup",
        "A doll's teacup",
        C,
        "No bigger than a thumbnail.",
        None,
    ),
    t("ticket", "A ferry ticket", C, "Return, never used.", None),
    t(
        "brooch",
        "A dragonfly brooch",
        C,
        "Its wings are green glass.",
        None,
    ),
    t(
        "harmonica",
        "A small harmonica",
        C,
        "Rusty, but the low notes still sing.",
        None,
    ),
    t(
        "compass",
        "A brass compass",
        C,
        "It points the way the water goes.",
        None,
    ),
    t(
        "medal",
        "A swimming medal",
        C,
        "First place, one length, 1952.",
        None,
    ),
];

/// What a set says once every piece is on the shelf.
pub fn set_story(set: &str) -> &'static str {
    match set {
        "coat" => "All three horn buttons. Somebody's good coat can be buttoned up again.",
        "music box" => {
            "Comb, drum, key and lid. The music box plays again: a little tune about a river."
        }
        _ => "A set, complete.",
    }
}

pub fn get(id: &str) -> Option<&'static Trinket> {
    TRINKETS.iter().find(|t| t.id == id)
}

/// The trinkets of `stream`.
pub fn of(stream: Stream) -> impl Iterator<Item = &'static Trinket> {
    TRINKETS.iter().filter(move |t| t.stream == stream)
}

/// Sets that `found` completes, which `before` did not.
pub fn sets_completed(before: &[String], found: &[String]) -> Vec<&'static str> {
    let mut out = Vec::new();
    for set in ["coat", "music box"] {
        let pieces: Vec<&str> = TRINKETS
            .iter()
            .filter(|t| t.set == Some(set))
            .map(|t| t.id)
            .collect();
        let had = pieces.iter().all(|p| before.iter().any(|b| b == p));
        let has = pieces.iter().all(|p| found.iter().any(|b| b == p));
        if has && !had {
            out.push(set);
        }
    }
    out
}

/// The shelf: every trinket found, stream by stream, with how many are
/// still out there.
pub struct Shelf {
    menu: MenuCore<Self, Parlor>,
}

impl Shelf {
    pub fn new() -> Shelf {
        Shelf {
            menu: MenuCore::new("Keepsake shelf").with_intro_help(
                "Up and Down go along the shelf, Enter reads a trinket, Escape goes back.",
            ),
        }
    }
}

impl Default for Shelf {
    fn default() -> Self {
        Shelf::new()
    }
}

impl Menu for Shelf {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "little_rapids_shelf"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let progress = Progress::load(ctx);
        let mut items = Vec::new();
        for stream in Stream::ALL {
            let found: Vec<&Trinket> = of(stream)
                .filter(|t| progress.trinkets.iter().any(|f| f == t.id))
                .collect();
            let total = of(stream).count();
            for t in &found {
                let line = t.line;
                let label = format!("{}, {}", t.name, stream.name());
                items.push(
                    MenuItem::new(label, move |_s: &mut Shelf, ctx: &mut Ctx<Parlor>| {
                        ctx.say(line)
                    })
                    .help(line),
                );
            }
            if found.len() < total {
                let left = total - found.len();
                let label = format!(
                    "{}: {} still in the water",
                    stream.name(),
                    if left == 1 {
                        "1 trinket".to_string()
                    } else {
                        format!("{left} trinkets")
                    }
                );
                items.push(
                    MenuItem::new(label, |_s: &mut Shelf, ctx: &mut Ctx<Parlor>| {
                        ctx.say("Not found yet. Trinkets float down that stream now and then.")
                    })
                    .select_sound(None)
                    .help("Trinkets are rare. You find new ones before ones you already have."),
                );
            }
        }
        for set in ["coat", "music box"] {
            let done = TRINKETS
                .iter()
                .filter(|t| t.set == Some(set))
                .all(|t| progress.trinkets.iter().any(|f| f == t.id));
            if done {
                let story = set_story(set);
                items.push(
                    MenuItem::new(
                        format!("The {set} set, complete"),
                        move |_s: &mut Shelf, ctx: &mut Ctx<Parlor>| {
                            if set == "music box" {
                                ctx.audio
                                    .play(crate::sound::MUSIC_BOX, hearken::Bus::EFFECTS);
                            }
                            ctx.say(story)
                        },
                    )
                    .select_sound(None)
                    .help(story),
                );
            }
        }
        items.push(
            MenuItem::new("Back", |_s: &mut Shelf, ctx: &mut Ctx<Parlor>| ctx.pop())
                .help("Return to the Little Rapids title."),
        );
        items
    }
}
