//! The rules on their own: courses, the snow, lines, spins and the bot.

use snowfall_skate::play::auto_target;
use snowfall_skate::ponds::{self, pond, PONDS};
use snowfall_skate::rules::{Course, End, Ev, Level, Skate, Spin};

const DT: f64 = 0.02;

/// Skates `course` to its end with the autoplay lean, landing a spin in
/// every long gap when `spins` is set. Gives the skate and every event.
pub fn bot(course: Course, spins: bool) -> (Skate, Vec<Ev>) {
    let mut s = Skate::new(course);
    s.start();
    let mut evs = Vec::new();
    let mut winding = false;
    for _ in 0..(60 * 60 * 50) {
        if s.ended().is_some() {
            break;
        }
        if spins {
            let far = s
                .next_lantern()
                .map(|i| s.course.lanterns[i].at - s.now() > 2700)
                .unwrap_or(true);
            if !winding && far && s.patch_ahead().is_none() {
                s.spin_down();
                winding = s.spinning();
            } else if winding && s.spin_ready() {
                evs.extend(s.spin_up());
                winding = false;
            }
        }
        let lean = if s.spinning() { 0 } else { auto_target(&s) };
        evs.extend(s.update(DT, lean));
    }
    (s, evs)
}

fn level(n: u32) -> Level {
    pond(n).level
}

#[test]
fn a_course_is_the_same_from_the_same_seed() {
    let mut a = Course::new(level(2), 9, Vec::new());
    let mut b = Course::new(level(2), 9, Vec::new());
    a.extend(120_000);
    b.extend(120_000);
    assert_eq!(a.lanterns, b.lanterns);
    assert_eq!(a.patches, b.patches);
    let mut c = Course::new(level(2), 10, Vec::new());
    c.extend(120_000);
    assert_ne!(a.lanterns, c.lanterns);
}

#[test]
fn the_village_pond_has_no_thin_ice() {
    let mut c = Course::new(level(1), 3, Vec::new());
    c.extend(300_000);
    assert!(!c.has_ice());
    assert!(c.has_drifts());
}

#[test]
fn every_pond_snows_over_in_the_end() {
    for p in &PONDS {
        for seed in 1..4 {
            let (s, _) = bot(Course::new(p.level, seed, Vec::new()), true);
            assert_eq!(s.ended(), Some(End::SnowedOver), "pond {}", p.number);
        }
    }
}

#[test]
fn the_bot_earns_every_star_and_a_lazy_skater_none() {
    for p in &PONDS {
        let mut scores = Vec::new();
        for seed in 1..6 {
            let (s, _) = bot(Course::new(p.level, seed, Vec::new()), true);
            scores.push(s.score);
        }
        let worst = *scores.iter().min().unwrap();
        println!("pond {}: {scores:?}", p.number);
        assert!(worst >= p.marks[2], "pond {}: {scores:?}", p.number);
        // Standing still in the middle.
        let mut s = Skate::new(Course::new(p.level, 1, Vec::new()));
        s.start();
        for _ in 0..(60 * 60 * 50) {
            if s.ended().is_some() {
                break;
            }
            s.update(DT, 0);
        }
        assert!(s.score < p.marks[0], "pond {} idle {}", p.number, s.score);
    }
}

#[test]
fn courses_with_an_end_end() {
    for level in [ponds::PRACTICE, ponds::FAIR, ponds::LANTERN_SKATE] {
        let (s, _) = bot(Course::new(level, 5, Vec::new()), false);
        assert_eq!(s.ended(), Some(End::Done));
    }
}

#[test]
fn the_bot_takes_every_lantern_on_practice() {
    let (s, _) = bot(Course::new(ponds::PRACTICE, 7, Vec::new()), false);
    assert_eq!(s.tally.taken, 10, "{:?}", s.tally);
    assert_eq!(s.tally.drifts + s.tally.cracks, 0, "{:?}", s.tally);
}

#[test]
fn a_line_is_worth_more_each_lantern_up_to_five() {
    let (_, evs) = bot(Course::new(ponds::PRACTICE, 7, Vec::new()), false);
    let points: Vec<u32> = evs
        .iter()
        .filter_map(|e| match e {
            Ev::Taken { points, .. } => Some(*points),
            _ => None,
        })
        .collect();
    assert_eq!(&points[..7], &[1, 2, 3, 4, 5, 5, 5]);
}

#[test]
fn one_spin_lands_for_each_lantern_passed() {
    let mut s = Skate::new(Course::new(ponds::PRACTICE, 7, Vec::new()));
    s.start();
    let mut spun = Vec::new();
    for _ in 0..2 {
        s.spin_down();
        for _ in 0..60 {
            s.update(DT, 0);
        }
        assert!(s.spin_ready());
        spun.extend(s.spin_up());
    }
    assert!(matches!(spun[0], Ev::Spun(Spin::Landed(3))), "{spun:?}");
    assert!(matches!(spun[1], Ev::Spun(Spin::TooSoon)), "{spun:?}");
}

#[test]
fn a_spin_held_too_long_wobbles() {
    let mut s = Skate::new(Course::new(ponds::PRACTICE, 7, Vec::new()));
    s.start();
    s.spin_down();
    let mut evs = Vec::new();
    for _ in 0..200 {
        evs.extend(s.update(DT, 0));
    }
    assert!(
        evs.iter().any(|e| matches!(e, Ev::Spun(Spin::Wobble))),
        "{evs:?}"
    );
}

#[test]
fn a_short_press_is_only_a_stride() {
    let mut s = Skate::new(Course::new(ponds::PRACTICE, 7, Vec::new()));
    s.start();
    s.spin_down();
    s.update(0.2, 0);
    assert_eq!(s.spin_up(), vec![Ev::Spun(Spin::Stride)]);
}
