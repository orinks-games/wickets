//! The hall's rules, with no screen.

use lamplighter::challenge::Plan;
use lamplighter::rules::*;
use parlor_core::Period;

fn started(layout: Layout, seed: u64) -> Hall {
    let mut h = Hall::new(layout, seed);
    h.start();
    h
}

/// Lights every lamp, nearest first, and returns every event.
fn light_all(h: &mut Hall) -> Vec<Ev> {
    let mut evs = Vec::new();
    while let Some((ahead, side)) = h.nearest_out() {
        for _ in 0..ahead.abs() {
            evs.extend(h.walk(ahead > 0));
        }
        evs.extend(h.reach(side));
    }
    evs
}

#[test]
fn a_hall_has_its_layouts_lamps_out() {
    for seed in 0..50 {
        let h = Hall::new(ROUND, seed);
        assert_eq!(h.length(), ROUND.length);
        assert_eq!(h.out(), ROUND.out, "seed {seed}");
        assert_eq!(h.pos, 0);
    }
}

#[test]
fn the_same_seed_gives_the_same_hall() {
    let a = Hall::new(ROUND, 9).out_lamps();
    assert_eq!(a, Hall::new(ROUND, 9).out_lamps());
    assert_ne!(a, Hall::new(ROUND, 10).out_lamps());
}

#[test]
fn nothing_moves_before_the_taper_is_lit() {
    let mut h = Hall::new(ROUND, 1);
    assert_eq!(h.walk(true), [Ev::NotStarted]);
    assert_eq!(h.reach(Side::Left), [Ev::NotStarted]);
    assert!(h.update(5.0).is_empty());
    assert_eq!(h.taper, ROUND.taper);
}

#[test]
fn the_ends_of_the_hall_stop_you() {
    let mut h = started(PRACTICE, 1);
    assert_eq!(h.walk(false), [Ev::LobbyDoor]);
    for _ in 1..PRACTICE.length {
        assert!(matches!(h.walk(true)[..], [Ev::Step { .. }]));
    }
    assert_eq!(h.pos, PRACTICE.length - 1);
    assert_eq!(h.walk(true), [Ev::FarWall]);
}

#[test]
fn reaching_lights_only_a_lamp_that_is_out() {
    let mut h = started(PRACTICE, 3);
    let (step, side) = h.out_lamps()[0];
    for _ in 0..step {
        h.walk(true);
    }
    assert_eq!(h.reach(side)[0], Ev::Lit { side });
    assert_eq!(h.reach(side), [Ev::AlreadyBurning { side }]);
    assert_eq!(h.score, LAMP_POINTS);
    assert_eq!(h.out(), PRACTICE.out - 1);
}

#[test]
fn a_bare_wall_says_so() {
    for seed in 0..20 {
        let mut h = started(ROUND, seed);
        for step in 0..h.length() {
            for side in Side::BOTH {
                if h.alcove(step, side) == Alcove::Bare {
                    while h.pos < step {
                        h.walk(true);
                    }
                    assert_eq!(h.reach(side), [Ev::Bare { side }]);
                    return;
                }
            }
        }
    }
    panic!("no bare wall in 20 halls");
}

#[test]
fn lighting_every_lamp_ends_the_round_with_the_taper_bonus() {
    let mut h = started(ROUND, 5);
    h.update(30.0);
    let evs = light_all(&mut h);
    assert!(h.finished());
    let bonus = 90;
    assert_eq!(evs.last(), Some(&Ev::HallLit { bonus }));
    assert_eq!(h.bonus, bonus);
    let lamps = ROUND.out as u32 + ROUND.draughts;
    assert_eq!(h.lit, lamps);
    assert_eq!(h.score, lamps * LAMP_POINTS + bonus);
}

#[test]
fn a_draught_blows_out_one_lamp_after_the_fourth() {
    let mut h = started(ROUND, 5);
    let evs = light_all(&mut h);
    let draughts: Vec<&Ev> = evs
        .iter()
        .filter(|e| matches!(e, Ev::Draught { .. }))
        .collect();
    assert_eq!(draughts.len(), ROUND.draughts as usize);
}

#[test]
fn the_taper_warns_then_is_replaced_and_the_bonus_is_gone() {
    let mut h = started(ROUND, 2);
    let t = ROUND.taper.unwrap();
    assert!(h.update(t - TAPER_LOW - 0.5).is_empty());
    assert_eq!(h.update(1.0), [Ev::TaperLow]);
    assert!(h.update(1.0).is_empty(), "warned once");
    assert_eq!(h.update(TAPER_LOW), [Ev::TaperOut]);
    assert!(!h.first_taper);
    assert_eq!(h.taper_secs(), Some(t as u32));
    assert_eq!(h.update(t - 10.0), [], "no warning on a fresh taper");
    light_all(&mut h);
    assert_eq!(h.bonus, 0);
    assert_eq!(h.tapers, 1);
}

#[test]
fn the_practice_hall_has_no_taper() {
    let mut h = started(PRACTICE, 2);
    assert!(h.update(1000.0).is_empty());
    assert_eq!(h.taper_secs(), None);
    light_all(&mut h);
    assert_eq!(h.bonus, 0);
}

#[test]
fn the_nearest_lamp_prefers_ahead_then_left() {
    let mut h = started(ROUND, 7);
    let (ahead, side) = h.nearest_out().unwrap();
    for _ in 0..ahead {
        h.walk(true);
    }
    assert_eq!(h.nearest_out().unwrap().0, 0);
    assert_eq!(h.alcove(h.pos, side), Alcove::Out);
}

#[test]
fn words_read_cleanly() {
    assert_eq!(time_words(45), "45 seconds");
    assert_eq!(time_words(60), "1 minute");
    assert_eq!(time_words(101), "1 minute 41 seconds");
    assert_eq!(where_words(0, Side::Left), "here on your left");
    assert_eq!(where_words(1, Side::Right), "1 step ahead on your right");
    assert_eq!(where_words(-3, Side::Left), "3 steps back on your left");
    assert_eq!(lamps_words(1), "1 lamp");
}

#[test]
fn challenge_halls_grow_through_the_periods() {
    let d = Plan::new(Period::Daily, 1).layout;
    let w = Plan::new(Period::Weekly, 1).layout;
    let m = Plan::new(Period::Monthly, 1).layout;
    assert!(d.length < w.length && w.length < m.length);
    assert!(d.draughts < w.draughts && w.draughts < m.draughts);
    assert_eq!(
        Plan::new(Period::Weekly, 4).hall().out_lamps(),
        Plan::new(Period::Weekly, 4).hall().out_lamps()
    );
}
