//! The hamlet's rules, without a screen.

use larkfield::almanac::{morning, Pace, Trouble};
use larkfield::buildings::Building;
use larkfield::rules::{At, Caller, Event, Hamlet, Phase, Place, Plot, MAX_SETTLERS};

/// Plays one day the way a steady player would: sows dug plots, starts
/// any building it can afford, takes everyone in, and eats supper.
pub fn play_day(h: &mut Hamlet) -> Vec<String> {
    h.start_work();
    if matches!(h.caller, Some(Caller::Stranger(_))) {
        h.answer_arrival(true);
    }
    if let Some(Caller::Visitor(_)) = h.caller {
        h.at = At::In(Place::Gate, 0);
        h.talk();
    }
    for b in Building::ALL {
        if h.can_build(b).is_ok() {
            h.start_building(b);
        }
    }
    // Dig and sow.
    for i in 0..h.plots.len() {
        h.at = At::Plot(i / 4, i % 4);
        if h.plots[i] == Plot::Scrub && h.hour < 1.0 {
            h.act();
        }
        if h.plots[i] == Plot::Tilled {
            if let Some((c, _)) = h.seed_list().first() {
                let id = c.id;
                h.sow(id);
            }
        }
        if h.plots[i].needs_water() || h.plots[i].ripe() {
            h.act();
        }
    }
    if h.barge {
        for id in ["turnip", "radish", "potato"] {
            while h.has(id) > 8 {
                h.sell(id);
            }
        }
        let stock = h.barge_stock();
        for (id, _, price) in stock {
            if id.starts_with("seed:") && h.coins > price + 30 && h.seeds.values().sum::<u32>() < 6
            {
                let _ = h.buy(&id);
            }
        }
    }
    while h.phase == Phase::Day {
        h.update(0.5);
    }
    let pages = h.supper(None);
    h.take_events();
    pages
}

#[test]
fn a_steady_year_grows_the_hamlet() {
    let mut h = Hamlet::home(11);
    for day in 0..40 {
        let pages = play_day(&mut h);
        if day % 10 == 9 {
            println!(
                "day {} {:?}: settlers {} built {:?} coins {} food {} wood {} stone {} moods {:?} keepsakes {}",
                h.day,
                h.season(),
                h.settlers.len(),
                h.built,
                h.coins,
                h.food_days(),
                h.has("wood"),
                h.has("stone"),
                h.settlers.iter().map(|s| (s.name.clone(), s.mood().name(), s.wobble)).collect::<Vec<_>>(),
                h.keepsakes.len()
            );
            println!("  {}", pages.join(" | "));
        }
    }
    assert!(
        h.settlers.len() >= 3,
        "only {} settlers after a year",
        h.settlers.len()
    );
    assert!(h.settlers.len() <= MAX_SETTLERS);
    assert!(h.built.len() >= 3, "only {:?} built after a year", h.built);
    assert_eq!(h.festivals, 4, "a year holds four festivals");
    assert!(!h.keepsakes.is_empty(), "no dream came true in a year");
}

#[test]
fn the_fishing_line_bites_and_reels() {
    let mut h = Hamlet::home(3);
    h.start_work();
    h.at = At::In(Place::River, 0);
    h.act();
    let mut bit = false;
    for _ in 0..100 {
        h.update(0.1);
        if h.take_events().contains(&Event::Bite) {
            bit = true;
            break;
        }
    }
    assert!(bit);
    h.act();
    assert_eq!(h.has("fish"), 1);
}

#[test]
fn a_filled_market_order_is_said_at_once() {
    let mut h = larkfield::challenge::hamlet(parlor_core::Period::Daily, 3);
    h.start_work();
    let goods = h.order.as_ref().expect("an order").goods.clone();
    for (id, n) in goods {
        h.add(&id, n);
    }
    h.events.clear();
    h.update(0.1);
    let said: Vec<String> = h
        .events
        .iter()
        .filter_map(|e| match e {
            Event::Say(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    assert!(
        said.iter().any(|t| t.contains("order is filled")),
        "{said:?}"
    );
    assert!(h.goal_met);
    // The score line adds up to the score.
    let line = h.score_line();
    let words: Vec<&str> = line.split_whitespace().collect();
    let sum: u64 = words
        .windows(2)
        .filter(|w| w[1] == "for")
        .filter_map(|w| w[0].parse::<u64>().ok())
        .sum();
    assert_eq!(sum, h.score(), "{line}");
    assert!(line.contains("100 for the goal"), "{line}");
}

fn said(h: &mut Hamlet) -> Vec<String> {
    h.take_events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Say(t) => Some(t),
            _ => None,
        })
        .collect()
}

/// Runs the day on to the supper bell.
fn to_supper(h: &mut Hamlet) {
    while h.phase == Phase::Day {
        h.update(0.5);
    }
}

#[test]
fn a_fire_burns_through_unless_put_out() {
    let mut h = Hamlet::home(3);
    h.pace = Pace::Steady;
    h.start_work();
    h.settlers.clear();
    h.built.push(Building::Mill);
    h.troubles.push(Trouble::Fire {
        b: Building::Mill,
        buckets: 6,
        left: 2.0,
    });
    h.at = At::In(Place::Yard, 0);
    for _ in 0..6 {
        h.act();
    }
    assert!(h.troubles.is_empty(), "{:?}", h.troubles);
    assert!(said(&mut h).iter().any(|t| t.contains("is out")));

    // Left alone, it burns through, and wood repairs it.
    h.troubles.push(Trouble::Fire {
        b: Building::Mill,
        buckets: 6,
        left: 2.0,
    });
    while h.phase == Phase::Day && h.troubles.iter().any(|t| matches!(t, Trouble::Fire { .. })) {
        h.update(0.5);
    }
    assert_eq!(h.phase, Phase::Day);
    assert_eq!(h.troubles, vec![Trouble::Burnt(Building::Mill)]);
    let wood = h.has("wood");
    h.take("wood", wood);
    h.act();
    assert!(said(&mut h).iter().any(|t| t.contains("3 wood")));
    h.add("wood", 3);
    h.act();
    assert!(h.troubles.is_empty());
    assert_eq!(h.has("wood"), 0);
}

#[test]
fn a_fox_takes_a_hen_unless_chased() {
    let mut h = Hamlet::home(3);
    h.start_work();
    h.settlers.clear();
    h.hens = 2;
    h.troubles.push(Trouble::Fox { left: 1.5 });
    h.at = At::In(Place::Cottages, 1);
    h.tend();
    assert!(h.troubles.is_empty());
    assert_eq!(h.hens, 2);
    h.troubles.push(Trouble::Fox { left: 1.5 });
    to_supper(&mut h);
    assert!(h.troubles.is_empty());
    assert_eq!(h.hens, 1);
}

#[test]
fn a_flood_washes_out_the_far_row_unless_held() {
    let mut h = Hamlet::home(3);
    h.start_work();
    h.settlers.clear();
    let sown = Plot::Sown {
        crop: "turnip".into(),
        growth: 2,
        watered: true,
        weeds: false,
    };
    h.plots[0] = sown.clone();
    h.plots[11] = sown;
    h.troubles.push(Trouble::Flood);
    to_supper(&mut h);
    let pages = h.supper(None);
    assert!(pages.iter().any(|p| p.contains("washed")), "{pages:?}");
    assert_eq!(h.plots[0], Plot::Tilled);
    assert!(matches!(h.plots[11], Plot::Sown { .. }));
    assert!(!h.troubles.contains(&Trouble::Flood));

    // Stone at the dock holds it back.
    h.start_work();
    h.troubles.push(Trouble::Flood);
    h.add("stone", 2);
    let stone = h.has("stone");
    h.at = At::In(Place::River, 1);
    h.act();
    assert!(!h.troubles.contains(&Trouble::Flood));
    assert_eq!(h.has("stone"), stone - 2);
}

#[test]
fn someone_lost_in_the_woods_is_found_with_enter() {
    let mut h = Hamlet::home(3);
    h.start_work();
    let s = h.make_settler(None);
    let name = s.name.clone();
    h.welcome(s);
    h.troubles.push(Trouble::Lost(name.clone()));
    h.at = At::In(Place::Woods, 0);
    h.talk();
    assert!(h.troubles.is_empty());
    assert!(said(&mut h).iter().any(|t| t.contains(&name)));
}

#[test]
fn a_glum_settler_warns_twice_then_leaves() {
    let mut h = Hamlet::home(3);
    h.pace = Pace::Steady;
    let s = h.make_settler(None);
    let name = s.name.clone();
    h.welcome(s);
    for (days, line) in [
        (5, "thinking of leaving"),
        (6, "leave tomorrow"),
        (7, "left for the next village"),
    ] {
        h.settlers.iter_mut().for_each(|s| s.low_days = days);
        h.news.clear();
        morning(&mut h);
        assert!(
            h.news.iter().any(|n| n.contains(&name) && n.contains(line)),
            "{days}: {:?}",
            h.news
        );
    }
    assert!(h.settler(&name).is_none());
}

#[test]
fn the_gentle_pace_has_no_dangers_and_nobody_leaves() {
    let mut h = Hamlet::home(5);
    h.pace = Pace::Gentle;
    for _ in 0..40 {
        h.settlers.iter_mut().for_each(|s| s.low_days = 30);
        let n = h.settlers.len();
        play_day(&mut h);
        assert!(h.alarm_at.is_none());
        assert!(!h.troubles.iter().any(|t| t.danger()), "{:?}", h.troubles);
        assert!(h.settlers.len() >= n);
        assert_eq!(h.cold_snap, 0);
    }
}
