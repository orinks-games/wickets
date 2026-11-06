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

/// The holiday specials: games the launcher lists in their holiday's
/// season, from its lead days before to the day after, in Seasonal games.
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
    /// Where it is listed out of season, once kept.
    pub home: Category,
    /// Whether the player has kept it: something only its season allows,
    /// such as beating its first night.
    pub kept: fn(&mut Ctx<Parlor>) -> bool,
    /// The blurb in season until it is kept, saying how to keep it.
    pub keep_blurb: &'static str,
    /// The blurb out of season, saying what still waits for the holiday.
    pub year_round_blurb: &'static str,
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
            home: graveyard_shuffle::HOME,
            kept: graveyard_shuffle::kept,
            keep_blurb: graveyard_shuffle::KEEP_BLURB,
            year_round_blurb: graveyard_shuffle::YEAR_ROUND_BLURB,
        },
    ]
}

/// The holiday specials in season on `date`.
pub fn in_season(date: Date) -> Vec<GameInfo> {
    specials()
        .into_iter()
        .filter(|(_, holiday)| calendar::nearness(*holiday, date).is_some())
        .map(|(g, _)| g)
        .collect()
}

/// The holiday specials the lists show today. In season, each is in
/// Seasonal games, with a word on how to keep it until it is kept. Out of
/// season, a kept one is in its own category, with what still waits for
/// its holiday.
pub fn specials_listed(ctx: &mut Ctx<Parlor>) -> Vec<GameInfo> {
    let today = challenges::today(ctx);
    keepers()
        .into_iter()
        .filter_map(|k| {
            let kept = (k.kept)(ctx);
            let mut g = k.game;
            if calendar::nearness(k.holiday, today).is_some() {
                if !kept {
                    g.blurb = k.keep_blurb;
                }
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
/// specials in season or kept.
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
