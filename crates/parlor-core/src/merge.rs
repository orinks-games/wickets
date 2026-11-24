//! Putting two copies of the records together: this computer's and the one
//! kept with the player's account, after both were played.
//!
//! Each saved key follows one rule ([`Rule`], from [`rule`]):
//!
//! - **Best**: `<game>/scores` keeps the higher score on every board.
//! - **Set**: `<game>/unlocks` keeps everything earned on either side, and
//!   flags that only ever turn on (`/played`, `/tutorial_read`) stay on.
//! - **Local**: never leaves this computer (the tester's own date, the last
//!   game opened here).
//! - **Newer**: anything else, such as a garden, a story's place or a
//!   preference, is one whole value; the copy saved later wins, and a tie
//!   keeps this computer's. Two runs' fields are never mixed, since a mix
//!   of two gardens is neither, with one exception: what was earned stays
//!   earned. A `stars` map inside keeps the most stars for each entry, and a
//!   `played` flag stays on, so a holiday special's stars from either
//!   computer survive.
//!
//! When both sides played on from a copy they last agreed on (the base,
//! which Hearken keeps after each sync), a Newer value only one side
//! changed takes that side's. One both changed, or one two copies hold with
//! no base between them, still goes whole to the copy saved later, but with
//! its counts of plays added up from both sides and the best of each best
//! kept ([`put_together`]).
//!
//! A key only one side has is simply kept. Values are merged as JSON, so a
//! field a newer version added survives a pass through an older one.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

/// How one key is merged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    Best,
    Set,
    Local,
    Newer,
}

/// Why a merge could not go ahead, for the log.
pub const NOT_WRITABLE: &str = "the records here cannot be saved";

/// Keys that never leave this computer.
const LOCAL: &[&str] = &["parlor/tester", "parlor.last_game"];

/// The rule for `key`.
pub fn rule(key: &str) -> Rule {
    if LOCAL.contains(&key) || key.starts_with("dummy/") {
        Rule::Local
    } else if key.ends_with("/scores") || key == "online/posted" {
        Rule::Best
    } else if key.ends_with("/unlocks") || key.ends_with("/played") || key.ends_with("/tutorial_read") {
        Rule::Set
    } else {
        Rule::Newer
    }
}

/// Whether `key` counts as progress worth asking about before two copies
/// are put together (a score, an unlock, a game played, the story begun).
pub fn is_progress(key: &str, value: &Value) -> bool {
    match rule(key) {
        Rule::Best => value.as_object().is_some_and(|m| m.values().any(|v| v.as_u64().is_some_and(|n| n > 0))),
        Rule::Set => match value {
            Value::Bool(b) => *b,
            Value::Object(m) => m.values().any(|v| v.as_array().is_some_and(|a| !a.is_empty())),
            _ => false,
        },
        Rule::Newer => key == "parlor/story",
        Rule::Local => false,
    }
}

/// One copy: each key's value, and when each was last saved (unix seconds,
/// 0 when unknown).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Copy {
    pub games: BTreeMap<String, Value>,
    pub stamps: BTreeMap<String, i64>,
}

/// Merges `cloud` into `local`, from `base` when there is one: the last
/// copy both sides agreed on. True when `local` changed.
pub fn merge(local: &mut Copy, cloud: &Copy, base: Option<&Copy>) -> bool {
    let mut changed = false;
    for (key, theirs) in &cloud.games {
        let rule = rule(key);
        if rule == Rule::Local {
            continue;
        }
        let their_stamp = cloud.stamps.get(key).copied().unwrap_or(0);
        let Some(ours) = local.games.get(key) else {
            local.games.insert(key.clone(), theirs.clone());
            local.stamps.insert(key.clone(), their_stamp);
            changed = true;
            continue;
        };
        let our_stamp = local.stamps.get(key).copied().unwrap_or(0);
        let newer_is_theirs = their_stamp > our_stamp;
        let was = base.and_then(|b| b.games.get(key));
        let merged = match rule {
            Rule::Best => best(ours, theirs, newer_is_theirs),
            Rule::Set => set(ours, theirs, newer_is_theirs),
            _ => match was {
                Some(was) if theirs == was => ours.clone(),
                Some(was) if ours == was => theirs.clone(),
                was => put_together(ours, theirs, was, newer_is_theirs),
            },
        };
        if &merged != ours {
            local.games.insert(key.clone(), merged);
            changed = true;
        }
        if newer_is_theirs {
            local.stamps.insert(key.clone(), their_stamp);
        }
    }
    changed
}

/// What a field of a Newer value holds, by its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    /// Plays counted up: both sides' plays add up.
    Count,
    /// A best: the higher stays (a number, or a map of them).
    Best,
    /// Lower is better (a fastest time, the fewest moves).
    Lowest,
    /// Anything else (a level, a place, a choice) goes with the whole value.
    Whole,
}

/// Fields that count plays, across the games.
const COUNTS: &[&str] = &[
    "nights", "wins", "runs", "games", "rounds", "strikes", "spares", "opened", "cracked",
    "sets_danced", "tunes_danced", "steps", "hearts", "pairs", "roses", "pipes", "perfect",
    "lamps", "practice_halls", "crumbs",
];

fn field(name: &str) -> Field {
    if COUNTS.contains(&name) {
        Field::Count
    } else if name.starts_with("fastest") || name.contains("fewest") || name.starts_with("shortest") {
        Field::Lowest
    } else if name.starts_with("best") || name.ends_with("_best") || name.starts_with("longest") || name.starts_with("deepest") {
        Field::Best
    } else {
        Field::Whole
    }
}

/// A value both sides changed (or that two copies with no base both hold):
/// the newer side whole, but with both sides' plays added up and the best
/// of each best kept. `was` is the base's value, `None` when there is none,
/// so each side's plays all count.
fn put_together(ours: &Value, theirs: &Value, was: Option<&Value>, newer_is_theirs: bool) -> Value {
    let (newer, older) = if newer_is_theirs { (theirs, ours) } else { (ours, theirs) };
    let (Value::Object(o), Value::Object(t)) = (ours, theirs) else {
        return keep_earned(newer, older);
    };
    let w = was.and_then(Value::as_object);
    let mut out = newer.as_object().cloned().unwrap_or_default();
    for (k, mine) in o {
        let Some(other) = t.get(k) else { continue };
        let next = match field(k) {
            Field::Count => match (mine.as_i64(), other.as_i64()) {
                (Some(a), Some(b)) => {
                    let base = w.and_then(|w| w.get(k)).and_then(Value::as_i64).unwrap_or(0);
                    Some(Value::from((a + b - base).max(a.min(b)).max(0)))
                }
                _ => None,
            },
            Field::Best => Some(best(mine, other, newer_is_theirs)),
            Field::Lowest => match (mine.as_f64(), other.as_f64()) {
                (Some(a), Some(b)) => Some(if a <= b { mine.clone() } else { other.clone() }),
                _ => None,
            },
            Field::Whole => None,
        };
        if let Some(next) = next {
            out.insert(k.clone(), next);
        }
    }
    keep_earned(&Value::Object(out), older)
}

/// `newer` whole, with the stars and played flag `older` earned kept.
fn keep_earned(newer: &Value, older: &Value) -> Value {
    let (Value::Object(n), Value::Object(o)) = (newer, older) else {
        return newer.clone();
    };
    let mut out = n.clone();
    if let (Some(a @ Value::Object(_)), Some(b @ Value::Object(_))) = (n.get("stars"), o.get("stars")) {
        out.insert("stars".into(), best(a, b, false));
    }
    if o.get("played") == Some(&Value::Bool(true)) && n.get("played").is_some_and(Value::is_boolean) {
        out.insert("played".into(), Value::Bool(true));
    }
    Value::Object(out)
}

/// The higher number at every key of two maps; anything that is not a
/// number goes to the newer side.
fn best(ours: &Value, theirs: &Value, newer_is_theirs: bool) -> Value {
    let (Value::Object(a), Value::Object(b)) = (ours, theirs) else {
        return if newer_is_theirs { theirs.clone() } else { ours.clone() };
    };
    let mut out = a.clone();
    for (board, score) in b {
        let keep = match (out.get(board), score) {
            (None, _) => score.clone(),
            (Some(Value::Number(x)), Value::Number(y)) => match (x.as_u64(), y.as_u64()) {
                (Some(x), Some(y)) => Value::from(x.max(y)),
                _ => best_f64(x.as_f64(), y.as_f64(), score, &out[board]),
            },
            (Some(Value::Object(_)), Value::Object(_)) => best(&out[board], score, newer_is_theirs),
            (Some(mine), _) if !newer_is_theirs => mine.clone(),
            _ => score.clone(),
        };
        out.insert(board.clone(), keep);
    }
    Value::Object(out)
}

fn best_f64(x: Option<f64>, y: Option<f64>, theirs: &Value, ours: &Value) -> Value {
    match (x, y) {
        (Some(x), Some(y)) if y > x => theirs.clone(),
        _ => ours.clone(),
    }
}

/// Everything either side earned: arrays are joined (each item once),
/// flags stay on once on, maps are merged key by key; anything else goes to
/// the newer side.
fn set(ours: &Value, theirs: &Value, newer_is_theirs: bool) -> Value {
    match (ours, theirs) {
        (Value::Bool(a), Value::Bool(b)) => Value::Bool(*a || *b),
        (Value::Array(a), Value::Array(b)) => {
            let mut seen = BTreeSet::new();
            let mut out = Vec::new();
            for item in a.iter().chain(b) {
                if seen.insert(item.to_string()) {
                    out.push(item.clone());
                }
            }
            Value::Array(out)
        }
        (Value::Object(a), Value::Object(b)) => {
            let mut out = a.clone();
            for (k, v) in b {
                let next = match out.get(k) {
                    None => v.clone(),
                    Some(mine) => set(mine, v, newer_is_theirs),
                };
                out.insert(k.clone(), next);
            }
            Value::Object(out)
        }
        _ if newer_is_theirs => theirs.clone(),
        _ => ours.clone(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn copy(games: Value, stamps: &[(&str, i64)]) -> Copy {
        Copy {
            games: serde_json::from_value(games).unwrap(),
            stamps: stamps.iter().map(|(k, s)| (k.to_string(), *s)).collect(),
        }
    }

    #[test]
    fn scores_keep_the_higher_and_unlocks_keep_everything() {
        let mut one = copy(
            json!({
                "belfry/scores": {"best": 40, "daily": 7},
                "belfry/unlocks": {"earned": ["a", "b"]},
                "belfry/played": true,
            }),
            &[],
        );
        let two = copy(
            json!({
                "belfry/scores": {"best": 30, "weekly": 9},
                "belfry/unlocks": {"earned": ["b", "c"]},
                "belfry/played": false,
                "safecracker/scores": {"best": 3},
            }),
            &[],
        );
        assert!(merge(&mut one, &two, None));
        assert_eq!(one.games["belfry/scores"], json!({"best": 40, "daily": 7, "weekly": 9}));
        assert_eq!(one.games["belfry/unlocks"], json!({"earned": ["a", "b", "c"]}));
        assert_eq!(one.games["belfry/played"], json!(true));
        assert_eq!(one.games["safecracker/scores"], json!({"best": 3}));
    }

    #[test]
    fn whole_values_go_to_the_copy_saved_later() {
        let garden_a = json!({"plants": ["bean"], "day": 4});
        let garden_b = json!({"plants": ["tomato", "lavender"], "day": 2});
        let mut one = copy(json!({"rooftop-garden/garden": garden_a}), &[("rooftop-garden/garden", 100)]);
        let two = copy(json!({"rooftop-garden/garden": garden_b}), &[("rooftop-garden/garden", 200)]);
        assert!(merge(&mut one, &two, None));
        assert_eq!(one.games["rooftop-garden/garden"], garden_b, "never a mix of the two");
        assert_eq!(one.stamps["rooftop-garden/garden"], 200);

        // Saved earlier on the account: this computer's stays, and on a tie too.
        let mut three = copy(json!({"rooftop-garden/garden": garden_a}), &[("rooftop-garden/garden", 300)]);
        assert!(!merge(&mut three, &two, None));
        let mut four = copy(json!({"rooftop-garden/garden": garden_a}), &[]);
        assert!(!merge(&mut four, &copy(json!({"rooftop-garden/garden": garden_b}), &[]), None));
        assert_eq!(four.games["rooftop-garden/garden"], garden_a);
    }

    #[test]
    fn stars_earned_on_either_computer_stay() {
        let mut one = copy(
            json!({"x/progress": {"played": true, "stars": {"1": 3, "4": 1}, "nights": 5}}),
            &[("x/progress", 100)],
        );
        let two = copy(
            json!({"x/progress": {"played": false, "stars": {"1": 1, "2": 2}, "nights": 2}}),
            &[("x/progress", 200)],
        );
        assert!(merge(&mut one, &two, None));
        assert_eq!(
            one.games["x/progress"],
            json!({"played": true, "stars": {"1": 3, "2": 2, "4": 1}, "nights": 7}),
            "with no base, both computers' nights count"
        );
    }

    #[test]
    fn counts_played_on_both_computers_add_up() {
        let was = copy(json!({"riverboat-dice/progress": {"nights": 3, "wins": 1, "longest_stay": 10}}), &[]);
        let mut one = copy(
            json!({"riverboat-dice/progress": {"nights": 4, "wins": 2, "longest_stay": 10}}),
            &[("riverboat-dice/progress", 100)],
        );
        let two = copy(
            json!({"riverboat-dice/progress": {"nights": 5, "wins": 1, "longest_stay": 12}}),
            &[("riverboat-dice/progress", 200)],
        );
        assert!(merge(&mut one, &two, Some(&was)));
        assert_eq!(
            one.games["riverboat-dice/progress"],
            json!({"nights": 6, "wins": 2, "longest_stay": 12})
        );
    }

    #[test]
    fn from_a_base_only_the_side_that_changed_counts() {
        let was = copy(json!({"x/progress": {"level": 2}, "x/garden": {"day": 1}}), &[]);
        // This computer saved later but never touched the garden.
        let mut one = copy(
            json!({"x/progress": {"level": 2}, "x/garden": {"day": 1}}),
            &[("x/garden", 900)],
        );
        let two = copy(json!({"x/progress": {"level": 3}, "x/garden": {"day": 4}}), &[("x/garden", 100)]);
        assert!(merge(&mut one, &two, Some(&was)));
        assert_eq!(one.games["x/garden"], json!({"day": 4}));
        assert_eq!(one.games["x/progress"], json!({"level": 3}));
    }

    #[test]
    fn the_same_count_on_both_sides_still_adds_up() {
        let was = copy(json!({"riverboat-dice/progress": {"nights": 1, "wins": 0}}), &[]);
        let mut one = copy(json!({"riverboat-dice/progress": {"nights": 2, "wins": 0}}), &[("riverboat-dice/progress", 100)]);
        let two = copy(json!({"riverboat-dice/progress": {"nights": 2, "wins": 1}}), &[("riverboat-dice/progress", 200)]);
        assert!(merge(&mut one, &two, Some(&was)));
        assert_eq!(one.games["riverboat-dice/progress"], json!({"nights": 3, "wins": 1}));
    }

    #[test]
    fn combining_with_no_base_keeps_both_computers_plays() {
        let mut here = copy(json!({"riverboat-dice/progress": {"nights": 1, "wins": 0}}), &[("riverboat-dice/progress", 900)]);
        let account = copy(json!({"riverboat-dice/progress": {"nights": 2, "wins": 1}}), &[("riverboat-dice/progress", 100)]);
        assert!(merge(&mut here, &account, None));
        assert_eq!(here.games["riverboat-dice/progress"], json!({"nights": 3, "wins": 1}));
    }

    #[test]
    fn levels_and_bests_keep_the_higher_and_the_rest_goes_whole() {
        let was = copy(
            json!({"x/progress": {"night": 3, "fastest": 90, "best": {"1": 10}, "songs": ["a"], "boat": "red"}}),
            &[],
        );
        let mut one = copy(
            json!({"x/progress": {"night": 4, "fastest": 80, "best": {"1": 12}, "songs": ["a", "b"], "boat": "blue"}}),
            &[("x/progress", 100)],
        );
        let two = copy(
            json!({"x/progress": {"night": 4, "fastest": 85, "best": {"1": 11, "2": 5}, "songs": ["a", "c"], "boat": "green"}}),
            &[("x/progress", 200)],
        );
        assert!(merge(&mut one, &two, Some(&was)));
        assert_eq!(
            one.games["x/progress"],
            json!({"night": 4, "fastest": 80, "best": {"1": 12, "2": 5}, "songs": ["a", "c"], "boat": "green"})
        );
    }

    #[test]
    fn local_keys_never_come_from_the_account() {
        let mut one = copy(json!({}), &[]);
        let two = copy(json!({"parlor/tester": {"date": "2026-12-01"}, "parlor.last_game": "belfry"}), &[]);
        assert!(!merge(&mut one, &two, None));
        assert!(one.games.is_empty());
    }

    #[test]
    fn merging_is_the_same_either_way_and_twice_changes_nothing() {
        let a = copy(
            json!({"x/scores": {"best": 4}, "x/unlocks": {"earned": ["p"]}, "story": 1}),
            &[("story", 5)],
        );
        let b = copy(
            json!({"x/scores": {"best": 9, "daily": 1}, "x/unlocks": {"earned": ["q"]}, "story": 2}),
            &[("story", 9)],
        );
        let mut ab = a.clone();
        merge(&mut ab, &b, None);
        let mut ba = b.clone();
        merge(&mut ba, &a, None);
        assert_eq!(ab.games["x/scores"], ba.games["x/scores"]);
        assert_eq!(ab.games["story"], ba.games["story"]);
        let sorted = |v: &Value| {
            let mut items: Vec<String> = v["earned"].as_array().unwrap().iter().map(|s| s.to_string()).collect();
            items.sort();
            items
        };
        assert_eq!(sorted(&ab.games["x/unlocks"]), sorted(&ba.games["x/unlocks"]));
        let again = ab.clone();
        assert!(!merge(&mut ab, &again, None));
        assert!(!merge(&mut ab, &b, None));
    }

    #[test]
    fn fields_a_newer_version_added_survive() {
        let mut one = copy(json!({"x/scores": {"best": 4}}), &[]);
        let two = copy(json!({"x/scores": {"best": 2, "new-board": 8}, "x/new-thing": {"a": 1}}), &[]);
        merge(&mut one, &two, None);
        assert_eq!(one.games["x/scores"]["new-board"], json!(8));
        assert_eq!(one.games["x/new-thing"], json!({"a": 1}));
    }

    #[test]
    fn progress_is_a_score_an_unlock_or_a_game_played() {
        assert!(!is_progress("x/scores", &json!({"best": 0})));
        assert!(is_progress("x/scores", &json!({"best": 3})));
        assert!(is_progress("x/unlocks", &json!({"earned": ["a"]})));
        assert!(!is_progress("x/unlocks", &json!({"earned": []})));
        assert!(is_progress("x/played", &json!(true)));
        assert!(!is_progress("parlor/preferences", &json!({"a": 1})));
        assert!(!is_progress("parlor/tester", &json!({"a": 1})));
    }
}
