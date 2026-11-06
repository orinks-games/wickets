//! The hamlet's rules, without a screen.

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
