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

/// Merges `cloud` into `local`. True when `local` changed.
pub fn merge(local: &mut Copy, cloud: &Copy) -> bool {
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
        let merged = match rule {
            Rule::Best => best(ours, theirs, newer_is_theirs),
            Rule::Set => set(ours, theirs, newer_is_theirs),
            _ if newer_is_theirs => keep_earned(theirs, ours),
            _ => keep_earned(ours, theirs),
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
        assert!(merge(&mut one, &two));
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
        assert!(merge(&mut one, &two));
        assert_eq!(one.games["rooftop-garden/garden"], garden_b, "never a mix of the two");
        assert_eq!(one.stamps["rooftop-garden/garden"], 200);

        // Saved earlier on the account: this computer's stays, and on a tie too.
        let mut three = copy(json!({"rooftop-garden/garden": garden_a}), &[("rooftop-garden/garden", 300)]);
        assert!(!merge(&mut three, &two));
        let mut four = copy(json!({"rooftop-garden/garden": garden_a}), &[]);
        assert!(!merge(&mut four, &copy(json!({"rooftop-garden/garden": garden_b}), &[])));
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
        assert!(merge(&mut one, &two));
        assert_eq!(
            one.games["x/progress"],
            json!({"played": true, "stars": {"1": 3, "2": 2, "4": 1}, "nights": 2})
        );
    }

    #[test]
    fn local_keys_never_come_from_the_account() {
        let mut one = copy(json!({}), &[]);
        let two = copy(json!({"parlor/tester": {"date": "2026-12-01"}, "parlor.last_game": "belfry"}), &[]);
        assert!(!merge(&mut one, &two));
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
        merge(&mut ab, &b);
        let mut ba = b.clone();
        merge(&mut ba, &a);
        assert_eq!(ab.games["x/scores"], ba.games["x/scores"]);
        assert_eq!(ab.games["story"], ba.games["story"]);
        let mut sorted = |v: &Value| {
            let mut items: Vec<String> = v["earned"].as_array().unwrap().iter().map(|s| s.to_string()).collect();
            items.sort();
            items
        };
        assert_eq!(sorted(&ab.games["x/unlocks"]), sorted(&ba.games["x/unlocks"]));
        let again = ab.clone();
        assert!(!merge(&mut ab, &again));
        assert!(!merge(&mut ab, &b));
    }

    #[test]
    fn fields_a_newer_version_added_survive() {
        let mut one = copy(json!({"x/scores": {"best": 4}}), &[]);
        let two = copy(json!({"x/scores": {"best": 2, "new-board": 8}, "x/new-thing": {"a": 1}}), &[]);
        merge(&mut one, &two);
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
