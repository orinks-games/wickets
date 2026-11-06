//! Wicket's soundtrack: the launcher's own music and every game's, one
//! album each, opened from the lobby's Soundtrack row.
//!
//! [`tracks`] lists them (see `parlor_core::soundtrack` for how a track is
//! unlocked and why some are secret). Short stingers, a few seconds of a
//! win or a loss, are left out: the soundtrack is music to sit and hear.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Ctx, SoundtrackScreen, Track, UiSound, Variation};
use parlor_core::soundtrack::Album;
use parlor_core::Parlor;

/// The launcher's own album.
pub const PARLOR: Album = Album::new("parlor", parlor_core::NAME);

/// The lobby row's label, and the albums menu's title.
pub const ROW: &str = "Soundtrack";

/// Every track, album by album: the launcher's, then the games' in the
/// launcher's order, then the holiday specials'.
pub fn tracks() -> Vec<Track> {
    let mut out = parlor();
    for g in crate::games::all() {
        out.extend(game(g.id, g.name));
    }
    for (g, _) in crate::games::specials() {
        out.extend(game(g.id, g.name).into_iter().map(Track::secret));
    }
    // Every track plays the way the game plays it: no key or mix shift.
    out.into_iter()
        .map(|t| t.variation(Variation::NONE))
        .collect()
}

fn parlor() -> Vec<Track> {
    let a = PARLOR;
    let lobby = |path: &str, title: &str, hint: &str, len: f64| {
        a.piece(path, title, hint, len)
            .about("One of the lobby's pieces, which take turns through the day.")
    };
    // A piece for one part of the day: a player may never open the parlor
    // then, so it plays from this menu before it is heard, and playing it
    // here counts it as heard.
    let hour = |path: &str, title: &str, hint: &str, len: f64| {
        lobby(path, title, hint, len).heard_in_menu()
    };
    vec![
        lobby("music/lobby_lounge", "Lounge", "Sit in the lobby.", 48.0),
        lobby("music/lobby_bossa", "Bossa nova", "Sit in the lobby.", 32.0),
        hour(
            "music/lobby_rag",
            "Parlor rag",
            "Sit in the lobby by day.",
            40.0,
        ),
        lobby(
            "music/lobby_swing",
            "Light swing",
            "Sit in the lobby.",
            58.2,
        ),
        lobby(
            "music/lobby_waltz",
            "Musette waltz",
            "Sit in the lobby.",
            38.4,
        ),
        hour(
            "music/lobby_morning",
            "Morning",
            "Open the parlor in the morning.",
            73.8,
        ),
        hour(
            "music/lobby_afternoon",
            "Afternoon",
            "Open the parlor in the afternoon.",
            76.8,
        ),
        hour(
            "music/lobby_evening",
            "Evening",
            "Open the parlor in the evening.",
            109.7,
        ),
        hour(
            "music/lobby_small_hours",
            "Small hours",
            "Open the parlor after midnight.",
            64.0,
        ),
        a.piece(
            "music/challenges",
            "The Challenges board",
            "Open Challenges from the lobby.",
            35.6,
        )
        .about("The music on the Challenges board."),
        // No Pause theme: every game pauses to its own music, so the
        // parlor's shared pause loop is never heard and could never unlock.
    ]
}

/// A game's tracks, by its id. Lengths are the files' own, in seconds.
fn game(id: &'static str, name: &'static str) -> Vec<Track> {
    let a = Album::new(id, name);
    let play = format!("Play {name}.");
    match id {
        "basket-run" => vec![a.title(36.9), a.score(36.9, &play), a.pause(22.9)],
        "belfry" => vec![a.title(26.7), a.pause(34.3)],
        "birdsong-trail" => vec![a.title(14.5), a.pause(29.1)],
        "clover-jig" => vec![a.title(9.2), a.pause(17.1)],
        "cupids-quiver" => vec![
            a.title(21.4),
            a.piece("music/roses", "Roses in the sky", "", 21.4)
                .secret()
                .about("The title in the Valentine's season."),
            a.piece("music/valentine", "Valentine's Day", "", 25.3)
                .secret()
                .about("The title on Valentine's Day itself."),
            a.pause(26.7),
        ],
        "dragon-dance" => vec![
            a.title(19.2),
            a.piece("music/festive", "New Year", "", 17.8)
                .secret()
                .about("The title in the run-up to New Year, with lion-dance drums and cymbals."),
            a.piece("music/street", "The street", "Dance down the street.", 23.0)
                .about("The soft bed under the street, so the drums lead."),
            a.piece("music/lantern_walk", "The lantern walk", "", 23.0)
                .secret()
                .about("The lantern walk's calm bed."),
            a.pause(27.4),
        ],
        "firefly-meadow" => vec![a.title(32.0), a.pause(24.0)],
        "fireside-patience" => vec![a.pause(29.1)],
        "freewheel" => vec![a.title(11.9), a.pause(22.9)],
        "graveyard-shuffle" => vec![a.title(90.0), a.pause(19.2)],
        "harvest-feast" => vec![a.title(36.9), a.score(36.9, &play), a.pause(22.9)],
        "haunted-hallways" => vec![a.title(45.7), a.score(45.7, &play), a.pause(29.1)],
        "lamplighter" => vec![a.title(29.1), a.pause(16.0)],
        "lantern-lake" => lantern_lake::soundtrack::tracks(),
        "liberty-lights" => vec![a.title(32.0), a.score(32.0, &play), a.pause(25.3)],
        "midnight-lanes" => vec![a.title_at("title-music", 11.5), a.pause(25.3)],
        "neon-rail" => vec![
            a.title(40.0),
            a.score(32.5, "Fly a run."),
            a.piece(
                "music/intermission",
                "Intermission",
                "Reach an intermission between sectors.",
                64.0,
            )
            .about("The music between sectors, while you choose an upgrade and a route."),
            a.piece(
                "music/workshop",
                "The workshop",
                "Visit the workshop.",
                21.8,
            )
            .about("The music in the workshop."),
            a.pause(24.0),
        ],
        "pantry-mouse" => vec![a.title_at("title-music", 9.5), a.pause(21.8)],
        "riverboat-dice" => vec![a.title_at("title-music", 10.0), a.pause(26.7)],
        "rooftop-garden" => rooftop_garden::soundtrack::tracks(),
        "larkfield" => larkfield::soundtrack::tracks(),
        "little-rapids" => little_rapids::soundtrack::tracks(),
        "safecracker" => vec![a.title(41.7), a.score(38.4, "Crack a safe."), a.pause(25.3)],
        "signal-box" => vec![a.title(12.8), a.pause(27.4)],
        "slow-kettle" => vec![
            a.title(26.7),
            a.piece("music/shop", "The shop", "Visit the shop.", 18.0)
                .about("The music in the tea shop."),
            a.piece(
                "musicbox/rain",
                "Rain on the roof",
                "Buy the music box in the shop.",
                25.7,
            )
            .about("The music box's first tune: a slow waltz."),
            a.piece(
                "musicbox/river",
                "The slow river",
                "Buy this tune for the music box in the shop.",
                30.0,
            )
            .about("A music box tune, low and rolling."),
            a.piece(
                "musicbox/lanterns",
                "Lanterns at dusk",
                "Buy this tune for the music box in the shop.",
                22.5,
            )
            .about("A music box tune, bright and twinkling."),
            a.pause(29.1),
        ],
        "soft-landing" => vec![a.title(19.2), a.pause(34.3)],
        "sonar-dive" => vec![
            a.title(60.0),
            a.score(60.0, "Dive."),
            a.piece(
                "music/surface",
                "On the boat",
                "Surface with treasure.",
                32.0,
            )
            .about("The music on the boat between dives."),
            a.pause(32.0),
        ],
        "stargazer" => vec![
            a.title(15.0),
            a.piece(
                "music/shower",
                "Meteor shower",
                "Watch a meteor shower on its peak night.",
                18.5,
            )
            .about("The music under a meteor shower."),
            a.pause(34.3),
        ],
        "the-echo" => vec![a.title(32.0), a.pause(19.2)],
        "tide-pools" => vec![a.title(58.2), a.score(53.3, &play), a.pause(30.0)],
        "tightrope" => vec![
            a.title(43.6),
            a.score(48.0, "Walk the wire."),
            a.pause(15.0),
        ],
        "winter-wonderland" => vec![
            a.title(91.0),
            a.score(34.3, "Play Gift Sort."),
            a.pause(26.7),
        ],
        _ => Vec::new(),
    }
}

/// Lists the tracks for Hearken. The launcher calls it before the app
/// starts.
pub fn register() {
    parlor_core::soundtrack::register(tracks);
    parlor_core::soundtrack::register_history(history);
}

/// The player's own songs, from the games that keep them.
pub fn history(ctx: &mut Ctx<Parlor>) -> Vec<Track> {
    let mut out = lantern_lake::soundtrack::history(ctx);
    out.extend(rooftop_garden::soundtrack::history(ctx));
    out
}

/// The albums with anything to list yet, in order, each with its unlocked
/// and listed counts.
fn albums(ctx: &mut Ctx<Parlor>) -> Vec<(String, usize, usize)> {
    let mut names: Vec<String> = Vec::new();
    for t in ctx.soundtrack.tracks() {
        if !names.contains(&t.album) {
            names.push(t.album.clone());
        }
    }
    names
        .into_iter()
        .filter_map(|name| {
            let ids: Vec<String> = ctx
                .soundtrack
                .listed(Some(&name))
                .iter()
                .map(|t| t.id.clone())
                .collect();
            if ids.is_empty() {
                return None;
            }
            let unlocked = ids.iter().filter(|id| ctx.track_unlocked(id)).count();
            Some((name, unlocked, ids.len()))
        })
        .collect()
}

/// The lobby's Soundtrack: every album, then Back. An album opens Hearken's
/// soundtrack menu for it. Nothing heard while it is open unlocks a track.
pub struct Albums {
    menu: MenuCore<Self, Parlor>,
}

impl Albums {
    pub fn new() -> Self {
        Albums {
            menu: MenuCore::new(ROW).with_intro_help(
                "Up and Down choose an album, Enter opens it, Escape goes back. \
                 Music unlocks as you hear it in play.",
            ),
        }
    }
}

impl Default for Albums {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for Albums {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "soundtrack_albums"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        // The player's newest nights and garden songs.
        parlor_core::soundtrack::refresh(ctx);
        parlor_core::soundtrack::quiet(ctx);
        hearken::menu::menu_enter(self, ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        parlor_core::soundtrack::listen(ctx);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items = Vec::new();
        let albums = albums(ctx);
        let unlocked: usize = albums.iter().map(|a| a.1).sum();
        if unlocked > 1 {
            items.push(
                MenuItem::new("Every album", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.push(SoundtrackScreen::<Parlor>::new().title("Every album"))
                })
                .help("Every track you have unlocked, from every game, with Play all."),
            );
        }
        for (name, unlocked, listed) in albums {
            let label = format!("{name}, {unlocked} of {listed} unlocked");
            let album = name.clone();
            items.push(
                MenuItem::new(label, move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.push(SoundtrackScreen::<Parlor>::new().album(album.clone()))
                })
                .help(format!(
                    "{name}'s music. A track unlocks the first time you hear it in play."
                )),
            );
        }
        items.push(
            MenuItem::new("Back", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.play_ui(UiSound::Back);
                ctx.pop()
            })
            .help("Return to the lobby."),
        );
        items
    }
}
