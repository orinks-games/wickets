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

/// Walk through the open door into room `to`, counting from 0.
fn through(house: &Live<Haunt>, to: usize, done: &'static str) -> parlor_core::Step {
    house
        .step(
            "Follow the cold draught to the open door, and walk through it.",
            move |h: &Haunt, _| (h.run.room >= to).then(|| done.to_string()),
        )
        .start(calm(house))
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
            "Every room is a grid of floorboards, and you always face north. {} walks north, {d} south, {l} west and {r} east. Take a few steps.",
            sentence_start(&u)
        ),
        move |h: &Haunt, _| {
            let moved = (h.run.x - start.0).abs() + (h.run.y - start.1).abs();
            (moved >= 2).then(|| "You are walking the house.".to_string())
        },
    );

    let status = house.step(
        format!(
            "{} gives any danger, the haunt clock, your courage and the way to the exit door. Press {s}, then press it again straight away.",
            sentence_start(&s)
        ),
        |h: &Haunt, _| {
            h.heard_more.then(|| {
                "The second press gives the room, its puzzle and where you stand.".to_string()
            })
        },
    );

    let find = house.step(
        format!(
            "Things in a room call out from where they lie, louder as you near them. This room asks for the {}: walk onto it and press {p}. {} repeats the clue.",
            first_find(),
            sentence_start(&rep)
        ),
        |h: &Haunt, _| {
            h.run.rooms[0]
                .solved
                .then(|| "A solved room unlocks its exit, and a cold draught blows from it.".to_string())
        },
    );

    let door1 = through(
        &house,
        1,
        "The door slams behind you. There is no going back.",
    );

    let ghost = house
        .step(
            "Ghosts walk the halls. A ghost comes from far off, from its direction. When it is right beside you, a sharp stab sounds: freeze, and take no step until it breathes out. One is coming now.",
            |h: &Haunt, _| {
                (h.run.ghosts_evaded >= 1).then(|| {
                    "It drifts away without finding you. Every ghost you let pass scores points."
                        .to_string()
                })
            },
        )
        .start(send_ghosts(&house, &[GhostKind::Widow]));

    let charm = house
        .step(
            "A soft glass chime is a charm on the floor. Walk onto it to take it.",
            |h: &Haunt, _| {
                (h.run.stats.charms >= 1).then(|| {
                    "Salt: it wards off one catch. Charms lie in some rooms from floor 3."
                        .to_string()
                })
            },
        )
        .start(calm(&house));

    let matching = house.step(
        format!(
            "This room's clue is on a phonograph. It plays a sound without naming it. Find the thing that makes the same sound and press {p} on it. {} plays it again. The other thing here is wrong, and touching it costs time.",
            sentence_start(&rep)
        ),
        |h: &Haunt, _| {
            h.run.rooms[1]
                .solved
                .then(|| "A riddle room works the same way, with a riddle for a clue.".to_string())
        },
    );

    let hint = house.step(
        format!(
            "{} gives a hint pointing the way to what you need, for a few seconds of the haunt clock. Press {h} now.",
            sentence_start(&h)
        ),
        |h: &Haunt, _| {
            (h.run.hints >= 1).then(|| "Hints cost more the deeper you go.".to_string())
        },
    );

    let door2 = through(&house, 2, "");

    let board = house.step(
        "That groan is a rotten board, right beside you. Step on it and you fall through. Go around it to the sealed exit door, at column 4 of the north wall, where a spirit whispers.",
        |h: &Haunt, _| {
            let room = &h.run.rooms[2];
            (h.run.room == 2 && room.inside(room.exit) == (h.run.x, h.run.y))
                .then(|| "You stand at the sealed door.".to_string())
        },
    );

    let offering = house.step(
        format!(
            "The spirit plays the sound of something you carry. {} chooses what you hold, and plays its sound. When it matches, press {p} here to offer it. {} plays the spirit's sound again.",
            sentence_start(&t),
            sentence_start(&rep)
        ),
        |h: &Haunt, _| {
            h.run.rooms[2]
                .solved
                .then(|| "The spirit takes it, and the door unlocks.".to_string())
        },
    );

    let door3 = through(&house, 3, "");

    let shadow = house.step(
        format!(
            "A low breathing drone is the shadow. It walks the room one cell at a time, toward your footsteps. Stand still and it wanders. Find the candle by its crackle, and keep the shadow off you. {} says where it is.",
            sentence_start(&s)
        ),
        |h: &Haunt, _| {
            h.run.rooms[3]
                .solved
                .then(|| "The shadow never leaves its room.".to_string())
        },
    );

    Lesson::new(
        ID,
        "Nevermore Manor",
        "The doors have locked behind you, and each floor is a chain of rooms, each with one puzzle that unlocks its exit. Learn in the practice house",
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
        "Deeper floors bring new dangers. Each is explained the first time you meet it. F1 says what to do at any moment. Play starts the climb.",
    )
}
