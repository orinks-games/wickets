//! Every game in the collection, in the order the launcher lists them,
//! and the story games, which story mode opens.

use hearken::Ctx;
use parlor_core::calendar::{self, Holiday};
use parlor_core::{challenges, preferences, Category, Date, GameInfo, Parlor};

/// Every game this copy lists: a game still on alpha or beta (see
/// [`parlor_core::channel`]) is left out of copies on a steadier channel.
pub fn all() -> Vec<GameInfo> {
    let mut games = every_game();
    games.retain(|g| parlor_core::channel::shows_game(g.id));
    games
}

/// Every game in the collection, whatever its stage.
fn every_game() -> Vec<GameInfo> {
    vec![
        liberty_lights::INFO,
        haunted_hallways::INFO,
        harvest_feast::INFO,
        winter_wonderland::INFO,
        basket_run::INFO,
        cupids_quiver::INFO,
        clover_jig::INFO,
        dragon_dance::INFO,
        firefly_meadow::INFO,
        neon_rail::INFO,
        safecracker::INFO,
        belfry::INFO,
        sonar_dive::INFO,
        tightrope::INFO,
        signal_box::INFO,
        midnight_lanes::INFO,
        soft_landing::INFO,
        pantry_mouse::INFO,
        freewheel::INFO,
        lantern_lake::INFO,
        tide_pools::INFO,
        rooftop_garden::INFO,
        larkfield::INFO,
        little_rapids::INFO,
        stargazer::INFO,
        slow_kettle::INFO,
        birdsong_trail::INFO,
        riverboat_dice::INFO,
        fireside_patience::INFO,
    ]
}

/// The story games, each with the story beat that opens it. They have no
/// lamp in the Long Hall, so they stay out of [`all`] and the lamp count.
pub fn story() -> Vec<(GameInfo, &'static str)> {
    Vec::new()
}

/// The holiday specials: games the launcher lists in their season, in
/// Seasonal games: most from their holiday's lead days before to the day
/// after, Snowfall Skate all winter.
/// They have no lamp in the Long Hall, so they stay out of [`all`]. Their
/// saves are kept the rest of the year.
pub fn specials() -> Vec<(GameInfo, Holiday)> {
    keepers().into_iter().map(|k| (k.game, k.holiday)).collect()
}

/// A holiday special, with how a player keeps it all year.
#[derive(Clone, Copy)]
pub struct Keeper {
    /// The game as its season lists it, in Seasonal games.
    pub game: GameInfo,
    pub holiday: Holiday,
    /// Whether `date` is in its season, when it is listed in Seasonal
    /// games: most from the holiday's lead days to the day after.
    pub in_season: fn(Date) -> bool,
    /// Where it is listed out of season, once kept.
    pub home: Category,
    /// Whether the player has started playing it, or has older saved progress.
    pub kept: fn(&mut Ctx<Parlor>) -> bool,
    /// The blurb in season until it is kept, saying how to keep it.
    pub keep_blurb: &'static str,
    /// The blurb out of season, saying what still waits for the holiday.
    pub year_round_blurb: &'static str,
}

/// Days after a special's season when a new player can still discover its base game.
/// This affects launcher visibility only, never a holiday event or finale's dates.
pub const DISCOVERY_GRACE_DAYS: i64 = 14;

impl Keeper {
    /// Is the base game available for a first visit today? Looking back from
    /// today handles winter ending in February, leap years and the turn of the year
    /// without changing the game's own seasonal rules.
    pub fn available(self, today: Date) -> bool {
        (0..=DISCOVERY_GRACE_DAYS)
            .any(|days| (self.in_season)(Date::from_days(today.days() - days)))
    }
}

/// Every holiday special, with how it is kept.
pub fn keepers() -> Vec<Keeper> {
    let mut keepers = every_keeper();
    keepers.retain(|k| parlor_core::channel::shows_game(k.game.id));
    keepers
}

fn every_keeper() -> Vec<Keeper> {
    vec![
        Keeper {
            game: graveyard_shuffle::INFO,
            holiday: graveyard_shuffle::HOLIDAY,
            in_season: |d| calendar::nearness(graveyard_shuffle::HOLIDAY, d).is_some(),
            home: graveyard_shuffle::HOME,
            kept: graveyard_shuffle::kept,
            keep_blurb: graveyard_shuffle::KEEP_BLURB,
            year_round_blurb: graveyard_shuffle::YEAR_ROUND_BLURB,
        },
        Keeper {
            game: snowfall_skate::INFO,
            holiday: snowfall_skate::HOLIDAY,
            in_season: snowfall_skate::in_season,
            home: snowfall_skate::HOME,
            kept: snowfall_skate::kept,
            keep_blurb: snowfall_skate::KEEP_BLURB,
            year_round_blurb: snowfall_skate::YEAR_ROUND_BLURB,
        },
    ]
}

/// The holiday a Seasonal game belongs to: the collection's own by id,
/// and each holiday special's from its keeper.
pub fn holiday(g: &GameInfo) -> Option<Holiday> {
    let own = match g.id {
        "liberty-lights" => Some(Holiday::IndependenceDay),
        "haunted-hallways" => Some(Holiday::Halloween),
        "harvest-feast" => Some(Holiday::Thanksgiving),
        "winter-wonderland" => Some(Holiday::NewYearsEve),
        "basket-run" => Some(Holiday::Easter),
        "cupids-quiver" => Some(Holiday::ValentinesDay),
        "clover-jig" => Some(Holiday::StPatricksDay),
        "dragon-dance" => Some(Holiday::LunarNewYear),
        "firefly-meadow" => Some(Holiday::SummerSolstice),
        _ => None,
    };
    own.or_else(|| {
        every_keeper()
            .into_iter()
            .find(|k| k.game.id == g.id)
            .map(|k| k.holiday)
    })
}

/// Whole days from `today` to `g`'s holiday, round the year: a holiday
/// from yesterday or today is 0 or less, so it leads. A game with no
/// holiday goes last.
pub fn days_to_holiday(g: &GameInfo, today: Date) -> i64 {
    holiday(g)
        .and_then(|h| h.next(today))
        .map_or(i64::MAX, |(_, days)| days)
}

/// The holiday specials in season on `date`.
pub fn in_season(date: Date) -> Vec<GameInfo> {
    keepers()
        .into_iter()
        .filter(|k| (k.in_season)(date))
        .map(|k| k.game)
        .collect()
}

/// The holiday specials the lists show today. During discovery, each is in
/// Seasonal games, including a two-week grace period after the season. Out of
/// season, a kept one is in its own category, with what still waits for
/// its holiday.
pub fn specials_listed(ctx: &mut Ctx<Parlor>) -> Vec<GameInfo> {
    specials_listed_at(ctx, discovery_date(ctx))
}

/// The date that decides whether a special not yet played can be found:
/// a pinned date (a test's, a tester's Test date, or a debug build's
/// preview), else the internet date from [`crate::discovery_clock`]. `None`
/// while that date is unknown: the computer's clock never stands in.
pub fn discovery_date(ctx: &Ctx<Parlor>) -> Option<Date> {
    challenges::pinned_today(ctx).or(match crate::discovery_clock::status() {
        crate::discovery_clock::Status::Ready(date) => Some(date),
        _ => None,
    })
}

/// Whether a game can be opened from a launcher list now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Open,
    /// A special not yet played, while the internet date is unknown.
    NeedsDate,
    /// A special not yet played, outside its season and catch-up weeks.
    Closed,
}

/// Checks a game again as it opens, since a list can be older than the
/// date: a special the player has not played needs the internet date and
/// its window. Every other game, and a kept special, opens.
pub fn access(ctx: &mut Ctx<Parlor>, id: &str) -> Access {
    let date = discovery_date(ctx);
    access_at(ctx, id, date)
}

/// [`access`] on a given internet date, for tests.
#[doc(hidden)]
pub fn access_at(ctx: &mut Ctx<Parlor>, id: &str, network_date: Option<Date>) -> Access {
    let Some(k) = keepers().into_iter().find(|k| k.game.id == id) else {
        return Access::Open;
    };
    if (k.kept)(ctx) {
        return Access::Open;
    }
    match network_date {
        None => Access::NeedsDate,
        Some(date) if k.available(date) => Access::Open,
        Some(_) => Access::Closed,
    }
}

/// Explicit date seam for deterministic offline/boundary tests. None means no
/// current network date, not permission to fall back to the computer clock.
#[doc(hidden)]
pub fn specials_listed_at(ctx: &mut Ctx<Parlor>, network_date: Option<Date>) -> Vec<GameInfo> {
    let today = challenges::today(ctx);
    keepers()
        .into_iter()
        .filter_map(|k| {
            let kept = (k.kept)(ctx);
            let mut g = k.game;
            if !kept && network_date.is_some_and(|date| k.available(date)) {
                g.blurb = k.keep_blurb;
                Some(g)
            } else if kept && (k.in_season)(today) {
                Some(g)
            } else if kept {
                g.category = k.home;
                g.blurb = k.year_round_blurb;
                Some(g)
            } else {
                None
            }
        })
        .collect()
}

/// Every game the lists show today: the collection, then the holiday
/// specials during their discovery window or kept.
pub fn listed(ctx: &mut Ctx<Parlor>) -> Vec<GameInfo> {
    let mut out = all();
    out.extend(specials_listed(ctx));
    out
}

/// The lamp games story mode offers: the collection, less any category
/// or game the player left out in Game preferences. Those stay in Play.
pub fn story_hall(ctx: &mut Ctx<Parlor>) -> Vec<GameInfo> {
    let prefs = preferences::load(ctx);
    all().into_iter().filter(|g| prefs.shows(g)).collect()
}

/// Every game Game preferences lists: the collection. The holiday
/// specials are left out, so none is named before its season; they follow
/// the category they are listed in.
pub fn preference_games() -> Vec<GameInfo> {
    all()
}
