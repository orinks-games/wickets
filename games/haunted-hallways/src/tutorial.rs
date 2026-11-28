//! The tutorial: a practice house of five rooms, played in the real game.
//! Each room teaches what the first floors bring: walking and status, the
//! three first puzzles, a ghost, a charm, a hint, a rotten board, a
//! spirit's offering and the shadow. Later dangers, such as the blind
//! hound, are taught the first time they are met. Nothing in it costs
//! courage, its clock stands still, and it ends before any floor is
//! cleared, so it never touches the records.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::haunt::Haunt;
use crate::rules::{
    Charm, CharmKind, Door, GhostKind, Item, Mode, Placed, Puzzle, Room, Run, Shadow, Side,
};
use crate::ID;

const SEED: u64 = 31;

/// A practice room, 5 cells square.
fn room(
    name: &'static str,
    entry: Door,
    exit: Door,
    puzzle: Puzzle,
    items: &[(Item, i32, i32)],
) -> Room {
    Room {
        name,
        w: 5,
        h: 5,
        entry,
        exit,
        puzzle,
        items: items
            .iter()
            .map(|&(item, x, y)| Placed {
                item,
                x,
                y,
                taken: false,
                mimic: false,
            })
            .collect(),
        solved: false,
        stage: 0,
        rotten: Vec::new(),
        holes: Vec::new(),
        dark: false,
        mirror: false,
        shadow: None,
        charm: None,
        pendulum: None,
        water: Vec::new(),
        stones: false,
        shifts: false,
    }
}

fn door(side: Side, at: i32) -> Door {
    Door { side, at }
}

/// The practice house's rooms.
pub fn rooms() -> Vec<Room> {
    use Side::*;
    // 1: a named thing to find.
    let foyer = room(
        "the foyer",
        door(South, 2),
        door(North, 2),
        Puzzle::Find(Item::Clock),
        &[(Item::Clock, 1, 3)],
    );
    // 2: a ghost, a charm, then a phonograph and a wrong thing.
    let mut library = room(
        "the library",
        door(South, 2),
        door(East, 3),
        Puzzle::Match(Item::Bell),
        &[(Item::Bell, 3, 3), (Item::Raven, 0, 3)],
    );
    library.charm = Some(Charm {
        kind: CharmKind::Salt,
        x: 4,
        y: 1,
        taken: false,
    });
    // 3: a rotten board beside the door, and a spirit who wants the clock.
    let mut hall = room(
        "the dining hall",
        door(West, 3),
        door(North, 3),
        Puzzle::Offering(Item::Clock),
        &[],
    );
    hall.rotten = vec![(1, 3)];
    // 4: the shadow walks while you find the candle. The lesson ends
    // when it is found.
    let mut parlour = room(
        "the parlour",
        door(South, 3),
        door(East, 2),
        Puzzle::Find(Item::Candle),
        &[(Item::Candle, 1, 2)],
    );
    parlour.shadow = Some(Shadow {
        x: 0,
        y: 4,
        home: (0, 4),
        interval: 2.0,
        step_in: 4.0,
    });
    vec![foyer, library, hall, parlour]
}

/// The thing the first room asks for, which the lesson names: the room's
/// own line is talked over by the lesson's opening.
fn first_find() -> &'static str {
    match rooms()[0].puzzle {
        Puzzle::Find(item) => item.name(),
        _ => "named thing",
    }
}

/// The practice house, as the lesson walks it.
pub fn house() -> Live<Haunt> {
    let mut haunt = Haunt::with_seed(Mode::Floor(1), SEED);
    haunt.run = Run::practice(rooms());
    Live::new(haunt)
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, house())
}

/// Sends ghosts of one kind, one after another, from a moment from now.
fn send_ghosts(house: &Live<Haunt>, kind: &'static [GhostKind]) -> impl FnMut(&mut Ctx<Parlor>) {
    let house = house.clone();
    move |_ctx| {
        house.with_mut(|h| {
            h.run.spec.ghosts = kind;
            h.run.spec.ghost_gap = (5.0, 7.0);
            h.run.ghosts_evaded = 0;
            if h.run.ghost.is_none() {
                h.run.ghost_in = 2.0;
            }
        });
    }
}

/// No more ghosts until a step sends them.
fn calm(house: &Live<Haunt>) -> impl FnMut(&mut Ctx<Parlor>) {
    let house = house.clone();
    move |_ctx| {
        house.with_mut(|h| {
            h.run.spec.ghost_gap = (1e9, 1e9);
            if h.run.ghost.is_none() {
                h.run.ghost_in = 1e9;
            }
        });
    }
}

/// Walk through the open door into room `to`, counting from 0. Until
/// this step the room's exit is held shut, so the player never walks
/// ahead of the lesson.
fn through(house: &Live<Haunt>, to: usize, done: &'static str) -> parlor_core::Step {
    let mut quiet = calm(house);
    let open = house.clone();
    house
        .step(
            "Follow the cold draught to the open door, and walk through it.",
            move |h: &Haunt, _| (h.run.room >= to).then(|| done.to_string()),
        )
        .start(move |ctx| {
            quiet(ctx);
            open.with_mut(|h| h.run.held = false);
        })
}

/// The tutorial on `house`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, house: Live<Haunt>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (u, d, l, r, p, t, s, rep, h) = (
        k(ParlorAction::Up),
        k(ParlorAction::Down),
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Primary),
        k(ParlorAction::Secondary),
        k(ParlorAction::Status),
        k(ParlorAction::Repeat),
        k(ParlorAction::Hint),
    );
    let start = house.with(|h| (h.run.x, h.run.y));

    let walk = house.step(
        format!(
            "Every room is a grid of floorboards, and you always face north. {} walks north, {d} south, {l} west and {r} east. Take a couple of steps to get a feel for it.",
            sentence_start(&u)
        ),
        move |h: &Haunt, _| {
            let moved = (h.run.x - start.0).abs() + (h.run.y - start.1).abs();
            (moved >= 2).then(|| "Good, you are moving through the house.".to_string())
        },
    );

    let status = house.step(
        format!(
            "{} gives a quick status check: nearby dangers, the haunt clock, your courage and how far the exit door is. Press {s}, then press it a second time straight away.",
            sentence_start(&s)
        ),
        |h: &Haunt, _| {
            h.heard_more.then(|| {
                "That second press gives the full picture: the room, its puzzle and where you stand."
                    .to_string()
            })
        },
    );

    let find = house.step(
        format!(
            "Objects lie scattered around each room. This room wants the {}: walk onto it and press {p} to pick it up. {} repeats the clue whenever you need it.",
            first_find(),
            sentence_start(&rep)
        ),
        |h: &Haunt, _| {
            h.run.rooms[0]
                .solved
                .then(|| "Solving a room unlocks its exit, and a cold draught blows from the open door.".to_string())
        },
    );

    let door1 = through(
        &house,
        1,
        "The door slams shut behind you. There is no going back.",
    );

    let ghost = house
        .step(
            "Ghosts roam these halls, approaching from far off. When one is right beside you, freeze and do not take a step until it drifts away. One is coming now.",
            |h: &Haunt, _| {
                (h.run.ghosts_evaded >= 1).then(|| {
                    "It drifts away without spotting you. Every ghost you let pass scores points."
                        .to_string()
                })
            },
        )
        .start(send_ghosts(&house, &[GhostKind::Widow]));

    let charm = house
        .step(
            "There is a charm on the floor here. Walk onto it to pick it up.",
            |h: &Haunt, _| {
                (h.run.stats.charms >= 1).then(|| {
                    "That is salt, which wards off one catch. You will find charms in some rooms starting on floor 3."
                        .to_string()
                })
            },
        )
        .start(calm(&house));

    let matching = house.step(
        format!(
            "This room's clue plays on a phonograph without naming the object. Find the object that matches it and press {p} on it, or press {rep} to replay the clue. Watch out for the decoy here, because touching the wrong thing costs time.",
        ),
        |h: &Haunt, _| {
            h.run.rooms[1]
                .solved
                .then(|| "Riddle rooms work the same way, giving you a riddle instead of a phonograph clue.".to_string())
        },
    );

    let hint = house.step(
        format!(
            "{} gives you a hint pointing toward what you need, for a few seconds off the haunt clock. Try pressing {h} now.",
            sentence_start(&h)
        ),
        |h: &Haunt, _| {
            (h.run.hints >= 1).then(|| "Hints cost a little more time the deeper you go.".to_string())
        },
    );

    let door2 = through(&house, 2, "");

    let board = house.step(
        "A rotten board lies right beside you, and stepping on it drops you through the floor. Walk around it to the sealed exit door at column 4 of the north wall, where a spirit is waiting.",
        |h: &Haunt, _| {
            let room = &h.run.rooms[2];
            (h.run.room == 2 && room.inside(room.exit) == (h.run.x, h.run.y))
                .then(|| "You are at the sealed door.".to_string())
        },
    );

    let offering = house.step(
        format!(
            "The spirit wants something you already picked up. {} cycles through what you carry, and once you have the match, press {p} here to offer it. {} replays what the spirit wants.",
            sentence_start(&t),
            sentence_start(&rep)
        ),
        |h: &Haunt, _| {
            h.run.rooms[2]
                .solved
                .then(|| "The spirit accepts your offering, and the door unlocks.".to_string())
        },
    );

    let door3 = through(&house, 3, "");

    let shadow = house.step(
        format!(
            "A shadow is stalking this room, moving one cell at a time toward your footsteps. Stand still and it wanders off. Find the candle while keeping the shadow off you, and press {s} to check where it is.",
        ),
        |h: &Haunt, _| {
            h.run.rooms[3]
                .solved
                .then(|| "The shadow never follows you out of its room.".to_string())
        },
    );

    Lesson::new(
        ID,
        "Nevermore Manor",
        "The doors of Nevermore Manor have locked behind you. Each floor is a chain of rooms, and solving each room's puzzle unlocks the exit. Try the basics in this safe practice house",
    )
    .step(walk)
    .step(status)
    .step(find)
    .step(door1)
    .step(ghost)
    .step(charm)
    .step(matching)
    .step(hint)
    .step(door2)
    .step(board)
    .step(offering)
    .step(door3)
    .step(shadow)
    .outro(
        "Deeper floors bring new dangers, and the house introduces each one the first time you meet it. Press F1 anytime you need a reminder, and choose Play when you are ready to start your climb.",
    )
}
