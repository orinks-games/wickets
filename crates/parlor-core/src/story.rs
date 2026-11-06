//! Story mode's planner: which games a sitting holds, from the date, the
//! part of the day, what each game holds today and the player's progress.
//!
//! Everything here is a plain function of a [`Today`], so tests walk it
//! over every day and every part of the day. The launcher gathers a
//! `Today` from the context once, when a sitting is planned, and saves the
//! [`Sitting`] it gets back, so a programme never changes under the
//! player. Ties are broken by a seed from the date and the part of the
//! day, so players with the same progress hear the same programme.

use hearken::{Ctx, LocalTime};

use crate::{Category, Date, Parlor, Period, Season};

/// The player's local time for story mode: the time a test pinned in
/// `ctx.game.now`; else, in debug builds only, `ECHO_PARLOR_NOW`
/// ("2026-10-31T21:30"), for trying a part of the day; else the trusted
/// clock in the computer's time zone.
pub fn now(ctx: &Ctx<Parlor>) -> LocalTime {
    if let Some(t) = ctx.game.now {
        return t;
    }
    let preview = cfg!(debug_assertions)
        .then(|| std::env::var("ECHO_PARLOR_NOW").ok())
        .flatten()
        .and_then(|v| LocalTime::parse(&v));
    preview.unwrap_or_else(|| hearken::clock::trusted_local_now().0)
}

/// The date a sitting belongs to: the local date, except that the hour
/// after midnight still belongs to the night before.
pub fn sitting_date(t: LocalTime) -> Date {
    let date = Date::new(t.year, t.month, t.day);
    if t.hour == 0 {
        Date::from_days(date.days() - 1)
    } else {
        date
    }
}

/// A part of the player's day, by their local clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PartOfDay {
    /// 5 to 8.
    Dawn,
    /// 8 to 12.
    Morning,
    /// 12 to 17.
    Afternoon,
    /// 17 to 21.
    Evening,
    /// 21 to 1.
    Night,
    /// 1 to 5.
    SmallHours,
}

impl PartOfDay {
    pub const ALL: [PartOfDay; 6] = [
        PartOfDay::Dawn,
        PartOfDay::Morning,
        PartOfDay::Afternoon,
        PartOfDay::Evening,
        PartOfDay::Night,
        PartOfDay::SmallHours,
    ];

    /// The part of the day an hour (0 to 23) falls in.
    pub fn of_hour(hour: u32) -> PartOfDay {
        match hour {
            5..=7 => PartOfDay::Dawn,
            8..=11 => PartOfDay::Morning,
            12..=16 => PartOfDay::Afternoon,
            17..=20 => PartOfDay::Evening,
            1..=4 => PartOfDay::SmallHours,
            _ => PartOfDay::Night,
        }
    }

    /// "this morning", "tonight", for "Story: tonight, 3 games".
    pub fn phrase(self) -> &'static str {
        match self {
            PartOfDay::Dawn => "this dawn",
            PartOfDay::Morning => "this morning",
            PartOfDay::Afternoon => "this afternoon",
            PartOfDay::Evening => "this evening",
            PartOfDay::Night => "tonight",
            PartOfDay::SmallHours => "the small hours",
        }
    }

    /// The saved id: "small_hours".
    pub fn id(self) -> &'static str {
        match self {
            PartOfDay::Dawn => "dawn",
            PartOfDay::Morning => "morning",
            PartOfDay::Afternoon => "afternoon",
            PartOfDay::Evening => "evening",
            PartOfDay::Night => "night",
            PartOfDay::SmallHours => "small_hours",
        }
    }

    pub fn from_id(id: &str) -> Option<PartOfDay> {
        PartOfDay::ALL.into_iter().find(|p| p.id() == id)
    }

    /// How many games a usual sitting holds now.
    pub fn usual_size(self) -> usize {
        match self {
            PartOfDay::Dawn | PartOfDay::SmallHours => 2,
            _ => 3,
        }
    }

    /// Whether only calm games are played: the small hours are quiet.
    pub fn calm_only(self) -> bool {
        self == PartOfDay::SmallHours
    }
}

/// How a game sits in the story: when it suits and in which seasons.
/// Plain data on [`crate::GameInfo`], so the planner runs no game code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoryTags {
    /// The parts of the day the game suits best.
    pub times: &'static [PartOfDay],
    /// The seasons it suits best; empty for any season.
    pub seasons: &'static [Season],
}

/// Whether a category is calm: no clock to beat and nothing to dodge.
pub fn is_calm(category: Category) -> bool {
    matches!(category, Category::Cozy | Category::Table)
}

/// How close a game's event is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventToday {
    /// The holiday or peak itself.
    Day,
    /// The run-up, with whole days to the holiday when known.
    RunUp { days: Option<i64> },
}

/// Days before a holiday when its run-up takes any part of the day, and
/// counts for a game still to be learned.
pub const RUN_UP_FINAL_DAYS: i64 = 3;

impl EventToday {
    /// The last few days of a run-up.
    fn final_days(self) -> bool {
        matches!(self, EventToday::RunUp { days: Some(d) } if d <= RUN_UP_FINAL_DAYS)
    }
}

/// One way into a game.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SceneKind {
    /// The game's interactive tutorial, for a game never played.
    Lesson,
    /// The event the game holds today.
    Event,
    /// Today's daily, weekly or monthly challenge.
    Challenge(Period),
    /// The game's title, to play any mode.
    Play,
}

impl SceneKind {
    /// The saved id: "lesson", "event", "daily", "play".
    pub fn id(self) -> &'static str {
        match self {
            SceneKind::Lesson => "lesson",
            SceneKind::Event => "event",
            SceneKind::Challenge(Period::Daily) => "daily",
            SceneKind::Challenge(Period::Weekly) => "weekly",
            SceneKind::Challenge(Period::Monthly) => "monthly",
            SceneKind::Play => "play",
        }
    }

    pub fn from_id(id: &str) -> Option<SceneKind> {
        Some(match id {
            "lesson" => SceneKind::Lesson,
            "event" => SceneKind::Event,
            "daily" => SceneKind::Challenge(Period::Daily),
            "weekly" => SceneKind::Challenge(Period::Weekly),
            "monthly" => SceneKind::Challenge(Period::Monthly),
            "play" => SceneKind::Play,
            _ => return None,
        })
    }
}

/// One game of a sitting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Scene {
    pub game: &'static str,
    pub kind: SceneKind,
}

/// What the planner knows about one game today.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameToday {
    pub id: &'static str,
    pub category: Category,
    pub tags: StoryTags,
    /// Never played and its tutorial never finished.
    pub needs_lesson: bool,
    pub event: Option<EventToday>,
    /// A villain from the Almanac is hiding in the game today.
    pub visitor: bool,
    /// The game has a seasonal touch on today.
    pub touch: bool,
    /// Challenges not played yet, with whole days until each resets.
    pub open_challenges: Vec<(Period, i64)>,
    /// The game's lamp is lit: it has been finished in a sitting.
    pub lamp_lit: bool,
    /// The game was in one of the player's last two sittings.
    pub recent: bool,
}

/// Everything the planner reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Today {
    pub date: Date,
    pub part: PartOfDay,
    /// Games in the sitting: 2 to 4. [`PartOfDay::usual_size`] by default.
    pub size: usize,
    /// How many lamps are lit, which paces the lessons.
    pub lamps: usize,
    pub games: Vec<GameToday>,
}

/// A planned sitting: its games in play order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sitting {
    pub date: Date,
    pub part: PartOfDay,
    pub scenes: Vec<Scene>,
}

/// Lamps lit before lessons stop being pushed to the front.
pub const FIRST_ACT_LAMPS: usize = 6;
/// The most event scenes a sitting holds, so ordinary games get a turn.
pub const MAX_EVENTS: usize = 2;
/// The most lessons a sitting holds.
pub const MAX_LESSONS: usize = 1;

/// The scene a game would play today, and its score.
fn best_scene(t: &Today, g: &GameToday, lessons_left: bool) -> (SceneKind, i32) {
    let season = Season::of(t.date);
    let mut score = 0;
    if g.tags.times.contains(&t.part) {
        score += 20;
    }
    if g.tags.seasons.is_empty() || g.tags.seasons.contains(&season) {
        score += if g.tags.seasons.is_empty() { 5 } else { 15 };
    }
    if !g.lamp_lit {
        score += 15;
    }
    if g.recent {
        score -= 30;
    }
    if g.visitor {
        score += 50;
    }
    if g.touch {
        score += 25;
    }
    // The holiday is days away: its game comes first, to learn if need be.
    let final_days = g.event.is_some_and(EventToday::final_days);
    if g.needs_lesson && lessons_left {
        score += if t.lamps < FIRST_ACT_LAMPS { 25 } else { 10 };
        if final_days {
            score += 40;
        }
        return (SceneKind::Lesson, score);
    }
    if let Some(event) = g.event {
        score += match event {
            EventToday::Day => 100,
            _ if final_days => 40,
            // A run-up lasts days, so it waits for a part of the day the
            // game suits rather than taking every sitting.
            EventToday::RunUp { .. } if g.tags.times.contains(&t.part) => 40,
            EventToday::RunUp { .. } => 10,
        };
        return (SceneKind::Event, score);
    }
    // The challenge that resets soonest; a daily before a longer one that
    // has days to go.
    let challenge = g
        .open_challenges
        .iter()
        .min_by_key(|(period, days)| (*days > 2 && *period != Period::Daily, *days));
    match challenge {
        Some(&(period, days)) => {
            score += if days <= 2 && period != Period::Daily {
                20
            } else {
                10
            };
            (SceneKind::Challenge(period), score)
        }
        None => (SceneKind::Play, score),
    }
}

/// A small tie-break, the same for every player on the same day and part
/// of the day: 0 to 9.
fn jitter(t: &Today, id: &str) -> i32 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let key = format!(
        "{}-{:02}-{:02}/{}/{id}",
        t.date.year,
        t.date.month,
        t.date.day,
        t.part.id()
    );
    for b in key.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    (crate::Rng::seeded(h).below(10)) as i32
}

#[derive(Clone, Copy, Debug)]
struct Pick {
    scene: Scene,
    category: Category,
    score: i32,
    day_event: bool,
    /// A holiday visitor's game, which the sitting holds while they are in.
    visitor: bool,
}

/// The candidates, best first.
fn ranked(t: &Today, lessons_left: bool, skip: &[&str]) -> Vec<Pick> {
    // A player who left every calm kind out of the story still gets a
    // small-hours sitting, from the kinds they kept.
    let calm_only = t.part.calm_only() && t.games.iter().any(|g| is_calm(g.category));
    let mut picks: Vec<Pick> = t
        .games
        .iter()
        .filter(|g| !skip.contains(&g.id))
        .filter(|g| !calm_only || is_calm(g.category))
        .map(|g| {
            let (kind, score) = best_scene(t, g, lessons_left);
            Pick {
                scene: Scene { game: g.id, kind },
                category: g.category,
                score: score * 10 + jitter(t, g.id),
                day_event: kind == SceneKind::Event && g.event == Some(EventToday::Day),
                visitor: g.visitor,
            }
        })
        .collect();
    picks.sort_by(|a, b| b.score.cmp(&a.score).then(a.scene.game.cmp(b.scene.game)));
    picks
}

fn fits(chosen: &[Pick], p: &Pick, size: usize, max_lessons: usize) -> bool {
    if !fits_loose(chosen, p, max_lessons) {
        return false;
    }
    let same = chosen.iter().filter(|c| c.category == p.category).count();
    // Two holidays of one kind on the same day may share a sitting, kept
    // apart by a game between them; anything else is one of each kind.
    let shared_day = p.day_event
        && size >= 3
        && same == 1
        && chosen
            .iter()
            .all(|c| c.category != p.category || c.day_event);
    if same > 0 && !shared_day {
        return false;
    }
    // Keep the last free slot for a calm game when none is in yet.
    let calm_in = chosen.iter().any(|c| is_calm(c.category));
    !(chosen.len() + 1 == size && !calm_in && !is_calm(p.category))
}

/// Puts the picks in play order: a light opener, the feature, then a calm
/// nightcap, never two of a category side by side.
fn order(chosen: Vec<Pick>) -> Vec<Scene> {
    // The feature: a holiday on its day, else the best score.
    let feature = chosen
        .iter()
        .max_by(|a, b| {
            (a.day_event, a.score)
                .cmp(&(b.day_event, b.score))
                .then(b.scene.game.cmp(a.scene.game))
        })
        .map(|p| p.scene.game);
    let n = chosen.len();
    let mut best: Option<(i32, Vec<usize>)> = None;
    let mut idx: Vec<usize> = (0..n).collect();
    permutations(&mut idx, 0, &mut |perm| {
        let apart = perm
            .windows(2)
            .all(|w| chosen[w[0]].category != chosen[w[1]].category);
        if !apart {
            return;
        }
        let mut score = 0;
        if is_calm(chosen[perm[n - 1]].category) {
            score += 4;
        }
        let at = perm
            .iter()
            .position(|&i| Some(chosen[i].scene.game) == feature);
        if n >= 3 && at == Some(n - 2) || n < 3 && at == Some(0) {
            score += 2;
        }
        // Otherwise keep the order the scores gave.
        if best.as_ref().is_none_or(|(s, _)| score > *s) {
            best = Some((score, perm.to_vec()));
        }
    });
    let perm = best.map(|(_, p)| p).unwrap_or_else(|| (0..n).collect());
    perm.into_iter().map(|i| chosen[i].scene).collect()
}

/// Calls `f` with every ordering of `items[k..]`, in a fixed order.
fn permutations(items: &mut Vec<usize>, k: usize, f: &mut dyn FnMut(&[usize])) {
    if k == items.len() {
        f(items);
        return;
    }
    for i in k..items.len() {
        items.swap(k, i);
        permutations(items, k + 1, f);
        items.swap(k, i);
    }
}

/// Plans a sitting. Holds `t.size` games (2 to 4), fewer only when fewer
/// games are left to play.
pub fn plan(t: &Today) -> Sitting {
    pick(t, &[])
}

fn pick(t: &Today, skip: &[&str]) -> Sitting {
    let size = t.size.clamp(2, 4);
    let mut chosen: Vec<Pick> = Vec::new();
    let all = ranked(t, true, skip);
    let take = |chosen: &mut Vec<Pick>, only_days: bool, max_lessons: usize| {
        for p in all
            .iter()
            .filter(|p| !only_days || p.day_event || p.visitor)
        {
            if chosen.len() >= size {
                break;
            }
            let new = chosen.iter().all(|c| c.scene.game != p.scene.game);
            if new && fits(chosen, p, size, max_lessons) {
                chosen.push(*p);
            }
        }
    };
    // A holiday on its day and a visitor's game come first (the visitor
    // asks for it), then the best of the rest with
    // one lesson at most. A player with too few games played to fill the
    // sitting gets more lessons rather than a short evening.
    take(&mut chosen, true, MAX_LESSONS);
    take(&mut chosen, false, MAX_LESSONS);
    take(&mut chosen, false, size);
    // Too few kinds of game left (a narrow skip list): fill with anything.
    for p in &all {
        if chosen.len() >= size {
            break;
        }
        if chosen.iter().all(|c| c.scene.game != p.scene.game) {
            chosen.push(*p);
        }
    }
    let scenes = if chosen.is_empty() {
        Vec::new()
    } else {
        order(chosen)
    };
    Sitting {
        date: t.date,
        part: t.part,
        scenes,
    }
}

/// Swaps the game at `index` for the best one that keeps the sitting's
/// shape and is not in `tried`. `None` when nothing else fits.
pub fn swap(t: &Today, sitting: &Sitting, index: usize, tried: &[&str]) -> Option<Scene> {
    let current = sitting.scenes.get(index)?;
    let category = |id: &str| t.games.iter().find(|g| g.id == id).map(|g| g.category);
    let others: Vec<Pick> = sitting
        .scenes
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != index)
        .filter_map(|(_, s)| {
            Some(Pick {
                scene: *s,
                category: category(s.game)?,
                score: 0,
                day_event: false,
                visitor: false,
            })
        })
        .collect();
    let size = sitting.scenes.len();
    let mut skip: Vec<&str> = sitting.scenes.iter().map(|s| s.game).collect();
    skip.extend_from_slice(tried);
    let apart_from_neighbours = |c: Category| {
        [index.checked_sub(1), Some(index + 1)]
            .into_iter()
            .flatten()
            .filter_map(|i| sitting.scenes.get(i))
            .all(|s| category(s.game) != Some(c))
    };
    let neighbours = |p: &Pick| apart_from_neighbours(p.category);
    let was_calm = category(current.game).is_some_and(is_calm);
    let calm_elsewhere = others.iter().any(|o| is_calm(o.category));
    // A new player's sitting can be all lessons; a swap may keep as many
    // as the sitting already has, so it never adds one.
    let lessons_now = sitting
        .scenes
        .iter()
        .filter(|s| s.kind == SceneKind::Lesson)
        .count();
    let max_lessons = MAX_LESSONS.max(lessons_now);
    let calm_kept = |p: &Pick| !was_calm || calm_elsewhere || is_calm(p.category);
    let all = ranked(t, true, &skip);
    // A hall of one kind (or two, both beside this game) has nothing of
    // another kind to put here: then the limits and the calm game alone.
    if !t.games.iter().any(|g| apart_from_neighbours(g.category)) {
        return all
            .into_iter()
            .find(|p| fits_loose(&others, p, max_lessons) && calm_kept(p))
            .map(|p| p.scene);
    }
    all.into_iter()
        .filter(|p| {
            fits(&others, p, size, max_lessons)
                || (neighbours(p) && fits_loose(&others, p, max_lessons))
        })
        .find(|p| neighbours(p) && calm_kept(p))
        .map(|p| p.scene)
}

/// The event and lesson limits alone.
fn fits_loose(chosen: &[Pick], p: &Pick, max_lessons: usize) -> bool {
    let events = chosen
        .iter()
        .filter(|c| c.scene.kind == SceneKind::Event)
        .count();
    let lessons = chosen
        .iter()
        .filter(|c| c.scene.kind == SceneKind::Lesson)
        .count();
    !(p.scene.kind == SceneKind::Event && events >= MAX_EVENTS)
        && !(p.scene.kind == SceneKind::Lesson && lessons >= max_lessons)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::{nearness, Holiday, Nearness};

    use PartOfDay::*;
    use Season::*;

    const ANY: &[Season] = &[];

    /// The parlor's games with their categories and story tags, as the
    /// launcher lists them.
    fn roster() -> Vec<(&'static str, Category, StoryTags, Option<Holiday>)> {
        use Category::*;
        let t = |times, seasons| StoryTags { times, seasons };
        vec![
            (
                "liberty-lights",
                Seasonal,
                t(&[Evening, Night], &[Summer]),
                Some(Holiday::IndependenceDay),
            ),
            (
                "haunted-hallways",
                Seasonal,
                t(&[Night, SmallHours], &[Autumn]),
                Some(Holiday::Halloween),
            ),
            (
                "harvest-feast",
                Seasonal,
                t(&[Afternoon, Evening], &[Autumn]),
                Some(Holiday::Thanksgiving),
            ),
            (
                "winter-wonderland",
                Seasonal,
                t(&[Morning, Evening], &[Winter]),
                None,
            ),
            (
                "basket-run",
                Seasonal,
                t(&[Morning], &[Spring]),
                Some(Holiday::Easter),
            ),
            (
                "cupids-quiver",
                Seasonal,
                t(&[Evening], &[Winter]),
                Some(Holiday::ValentinesDay),
            ),
            (
                "clover-jig",
                Seasonal,
                t(&[Evening], &[Spring]),
                Some(Holiday::StPatricksDay),
            ),
            (
                "dragon-dance",
                Seasonal,
                t(&[Afternoon, Evening], &[Winter]),
                Some(Holiday::LunarNewYear),
            ),
            (
                "firefly-meadow",
                Seasonal,
                t(&[Evening, Night], &[Summer]),
                Some(Holiday::SummerSolstice),
            ),
            ("neon-rail", Action, t(&[Night], ANY), None),
            ("safecracker", Puzzle, t(&[Night, SmallHours], ANY), None),
            ("belfry", Puzzle, t(&[Morning, Evening], ANY), None),
            ("sonar-dive", Action, t(&[Afternoon], &[Summer]), None),
            ("tightrope", Action, t(&[Afternoon], ANY), None),
            ("signal-box", Action, t(&[Morning, Evening], ANY), None),
            ("midnight-lanes", Action, t(&[Night], ANY), None),
            ("soft-landing", Action, t(&[Night], ANY), None),
            (
                "pantry-mouse",
                Action,
                t(&[Night, SmallHours], &[Autumn, Winter]),
                None,
            ),
            (
                "freewheel",
                Action,
                t(&[Morning], &[Spring, Summer, Autumn]),
                None,
            ),
            (
                "lantern-lake",
                Cozy,
                t(&[Evening, Night], &[Summer]),
                None,
            ),
            (
                "tide-pools",
                Cozy,
                t(&[Morning, Afternoon], &[Spring, Summer]),
                None,
            ),
            (
                "rooftop-garden",
                Cozy,
                t(&[Morning], &[Spring, Summer]),
                None,
            ),
            ("stargazer", Cozy, t(&[Night, SmallHours], ANY), None),
            (
                "slow-kettle",
                Cozy,
                t(&[Afternoon, Evening], &[Autumn, Winter]),
                None,
            ),
            (
                "birdsong-trail",
                Cozy,
                t(&[Dawn, Morning], &[Spring]),
                None,
            ),
            (
                "riverboat-dice",
                Table,
                t(&[Evening, Night], &[Summer]),
                None,
            ),
            (
                "fireside-patience",
                Table,
                t(&[Evening, Night], &[Autumn, Winter]),
                None,
            ),
        ]
    }

    fn today(date: Date, part: PartOfDay, lit: &[&str], recent: &[&str]) -> Today {
        let games = roster()
            .into_iter()
            .map(|(id, category, tags, holiday)| GameToday {
                id,
                category,
                tags,
                needs_lesson: !lit.contains(&id),
                event: holiday.and_then(|h| match nearness(h, date)? {
                    Nearness::Day => Some(EventToday::Day),
                    Nearness::Season { days } => Some(EventToday::RunUp { days: Some(days) }),
                }),
                visitor: false,
                touch: false,
                open_challenges: vec![
                    (Period::Daily, 1),
                    (Period::Weekly, 7 - date.weekday() as i64),
                ],
                lamp_lit: lit.contains(&id),
                recent: recent.contains(&id),
            })
            .collect();
        Today {
            date,
            part,
            size: part.usual_size(),
            lamps: lit.len(),
            games,
        }
    }

    fn category(id: &str) -> Category {
        roster().into_iter().find(|g| g.0 == id).unwrap().1
    }

    fn check_shape(s: &Sitting, t: &Today) {
        let n = s.scenes.len();
        assert_eq!(n, t.size, "{s:?}");
        for w in s.scenes.windows(2) {
            assert_ne!(category(w[0].game), category(w[1].game), "{s:?}");
        }
        assert!(
            s.scenes.iter().any(|sc| is_calm(category(sc.game))),
            "{s:?}"
        );
        let lessons = s
            .scenes
            .iter()
            .filter(|sc| sc.kind == SceneKind::Lesson)
            .count();
        let events = s
            .scenes
            .iter()
            .filter(|sc| sc.kind == SceneKind::Event)
            .count();
        assert!(events <= MAX_EVENTS, "{s:?}");
        // A player who knows enough games learns one at a time.
        if t.lamps >= 8 {
            assert!(lessons <= MAX_LESSONS, "{s:?}");
        }
        let mut ids: Vec<_> = s.scenes.iter().map(|sc| sc.game).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n, "{s:?}");
        if t.part.calm_only() {
            assert!(
                s.scenes.iter().all(|sc| is_calm(category(sc.game))),
                "{s:?}"
            );
        }
    }

    #[test]
    fn parts_of_the_day() {
        assert_eq!(PartOfDay::of_hour(0), Night);
        assert_eq!(PartOfDay::of_hour(1), SmallHours);
        assert_eq!(PartOfDay::of_hour(5), Dawn);
        assert_eq!(PartOfDay::of_hour(11), Morning);
        assert_eq!(PartOfDay::of_hour(12), Afternoon);
        assert_eq!(PartOfDay::of_hour(17), Evening);
        assert_eq!(PartOfDay::of_hour(21), Night);
        for p in PartOfDay::ALL {
            assert_eq!(PartOfDay::from_id(p.id()), Some(p));
        }
        for k in [
            SceneKind::Lesson,
            SceneKind::Event,
            SceneKind::Play,
            SceneKind::Challenge(Period::Daily),
            SceneKind::Challenge(Period::Weekly),
            SceneKind::Challenge(Period::Monthly),
        ] {
            assert_eq!(SceneKind::from_id(k.id()), Some(k));
        }
    }

    #[test]
    fn tonight_in_late_october() {
        let all: Vec<&str> = roster().iter().map(|g| g.0).collect();
        let t = today(Date::new(2026, 10, 22), Night, &all, &[]);
        let s = plan(&t);
        check_shape(&s, &t);
        // Halloween's run-up puts Nevermore Manor in, by its event.
        assert!(
            s.scenes.contains(&Scene {
                game: "haunted-hallways",
                kind: SceneKind::Event
            }),
            "{s:?}"
        );
    }

    #[test]
    fn a_holiday_on_its_day_is_the_feature() {
        let all: Vec<&str> = roster().iter().map(|g| g.0).collect();
        for (h, date) in [
            ("liberty-lights", Date::new(2027, 7, 4)),
            ("haunted-hallways", Date::new(2026, 10, 31)),
        ] {
            for part in [Morning, Afternoon, Evening, Night] {
                let t = today(date, part, &all, &[]);
                let s = plan(&t);
                check_shape(&s, &t);
                assert!(
                    s.scenes
                        .iter()
                        .any(|sc| sc.game == h && sc.kind == SceneKind::Event),
                    "{date:?} {part:?} {s:?}"
                );
            }
        }
    }

    /// While Lantern Jack is in, his game is on the sitting even when it
    /// was on the last one and another lesson scores as well.
    #[test]
    fn a_visitors_game_is_always_on_the_sitting() {
        for (date, part) in [
            (Date::new(2026, 10, 30), Morning),
            (Date::new(2026, 10, 30), Evening),
            (Date::new(2026, 10, 23), Afternoon),
            (Date::new(2026, 10, 23), Evening),
        ] {
            for lit in [&[][..], &["haunted-hallways"][..]] {
                let mut t = today(date, part, lit, &["haunted-hallways", "belfry"]);
                for g in &mut t.games {
                    g.visitor = g.id == "haunted-hallways";
                }
                let s = plan(&t);
                assert!(
                    s.scenes.iter().any(|sc| sc.game == "haunted-hallways"),
                    "{date:?} {part:?} {lit:?}: {s:?}"
                );
                check_shape(&s, &t);
            }
        }
    }

    #[test]
    fn one_lesson_a_sitting_once_a_few_games_are_known() {
        let known = [
            "neon-rail",
            "belfry",
            "stargazer",
            "riverboat-dice",
            "tightrope",
            "lantern-lake",
            "winter-wonderland",
            "safecracker",
        ];
        let t = today(Date::new(2026, 10, 22), Evening, &known, &[]);
        let s = plan(&t);
        check_shape(&s, &t);
        let lessons = s
            .scenes
            .iter()
            .filter(|sc| sc.kind == SceneKind::Lesson)
            .count();
        assert_eq!(lessons, 1, "{s:?}");
        // A new player learns a game in every slot.
        let t = today(Date::new(2026, 10, 22), Evening, &[], &[]);
        let s = plan(&t);
        check_shape(&s, &t);
        assert!(
            s.scenes.iter().all(|sc| sc.kind == SceneKind::Lesson),
            "{s:?}"
        );
    }

    #[test]
    fn recent_games_step_aside() {
        let all: Vec<&str> = roster().iter().map(|g| g.0).collect();
        let date = Date::new(2027, 3, 9);
        let first = plan(&today(date, Afternoon, &all, &[]));
        let recent: Vec<&str> = first.scenes.iter().map(|s| s.game).collect();
        let second = plan(&today(date, Afternoon, &all, &recent));
        assert!(
            second.scenes.iter().all(|s| !recent.contains(&s.game)),
            "{first:?} then {second:?}"
        );
    }

    #[test]
    fn swap_keeps_the_shape() {
        let all: Vec<&str> = roster().iter().map(|g| g.0).collect();
        let t = today(Date::new(2026, 10, 22), Evening, &all, &[]);
        let s = plan(&t);
        for i in 0..s.scenes.len() {
            let mut tried = Vec::new();
            while let Some(new) = swap(&t, &s, i, &tried) {
                let mut changed = s.clone();
                changed.scenes[i] = new;
                check_shape(&changed, &t);
                tried.push(new.game);
                assert!(tried.len() < 30);
            }
        }
    }

    #[test]
    fn a_new_player_can_swap_any_game_of_the_first_sitting() {
        let t = today(Date::new(2026, 10, 24), Evening, &[], &[]);
        let s = plan(&t);
        assert!(
            s.scenes.iter().all(|sc| sc.kind == SceneKind::Lesson),
            "{s:?}"
        );
        for i in 0..s.scenes.len() {
            let new = swap(&t, &s, i, &[]).unwrap_or_else(|| panic!("scene {i}: {s:?}"));
            let mut changed = s.clone();
            changed.scenes[i] = new;
            check_shape(&changed, &t);
        }
    }

    /// Every day from 2026 to 2040 at every part of the day, for a new
    /// player, one halfway and one with every lamp lit.
    #[test]
    fn the_date_walk() {
        let all: Vec<&str> = roster().iter().map(|g| g.0).collect();
        let half: Vec<&str> = all.iter().copied().step_by(2).collect();
        let mut seen: std::collections::HashMap<(i32, &str), usize> = Default::default();
        let start = Date::new(2026, 1, 1).days();
        let end = Date::new(2040, 12, 31).days();
        for day in start..=end {
            let date = Date::from_days(day);
            for part in PartOfDay::ALL {
                for lit in [&[][..], &half[..], &all[..]] {
                    let t = today(date, part, lit, &[]);
                    let s = plan(&t);
                    check_shape(&s, &t);
                    assert_eq!(plan(&t), s);
                    if lit.len() == all.len() {
                        for sc in &s.scenes {
                            *seen.entry((date.year, sc.game)).or_default() += 1;
                        }
                        // Every holiday on its day is in, as many as the
                        // sitting has room for: two can share a longer one.
                        let due: Vec<&str> = roster()
                            .into_iter()
                            .filter(|g| {
                                g.3.is_some_and(|h| nearness(h, date) == Some(Nearness::Day))
                            })
                            .map(|g| g.0)
                            .collect();
                        if part != SmallHours && !due.is_empty() {
                            let room = if t.size >= 3 { 2 } else { 1 };
                            let got = due
                                .iter()
                                .filter(|id| s.scenes.iter().any(|sc| sc.game == **id))
                                .count();
                            assert!(
                                got >= due.len().min(room),
                                "{due:?} on {date:?} {part:?}: {s:?}"
                            );
                        }
                    }
                }
            }
        }
        for year in 2026..=2040 {
            for id in &all {
                let n = seen.get(&(year, *id)).copied().unwrap_or(0);
                assert!(n >= 4, "{id} only {n} times in {year}");
            }
        }
    }

    #[test]
    fn a_hall_of_one_kind_still_fills_a_sitting() {
        let date = Date::new(2026, 10, 20);
        for part in PartOfDay::ALL {
            let mut t = today(date, part, &[], &[]);
            t.games.retain(|g| g.category == Category::Action);
            let s = plan(&t);
            assert_eq!(s.scenes.len(), t.size.clamp(2, 4), "{part:?}: {s:?}");
            assert!(
                s.scenes
                    .iter()
                    .all(|sc| category(sc.game) == Category::Action),
                "{s:?}"
            );
        }
    }

    #[test]
    fn a_hall_of_one_kind_can_swap() {
        let date = Date::new(2026, 10, 20);
        for part in PartOfDay::ALL {
            let mut t = today(date, part, &[], &[]);
            t.games.retain(|g| g.category == Category::Action);
            let s = plan(&t);
            for i in 0..s.scenes.len() {
                let new = swap(&t, &s, i, &[]).unwrap_or_else(|| panic!("{part:?} {i}: {s:?}"));
                assert!(s.scenes.iter().all(|sc| sc.game != new.game), "{new:?}");
                assert_eq!(category(new.game), Category::Action);
            }
        }
    }
}
