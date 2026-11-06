//! Update channels: stable, beta and alpha.
//!
//! Every released copy follows one channel (Settings, Update channel).
//! New work goes out on alpha first, moves to beta to be refined, then to
//! stable. Everything merges to main, so a game or feature that is not
//! ready for every player names its [`Stage`], and a copy released on a
//! steadier channel leaves it out. A stable copy shows stable work only; a
//! beta copy shows beta and stable work; an alpha copy shows everything.
//! A developer build (no update key) shows everything too.
//!
//! Promoting work is one edit: change its stage in [`GAMES`], or the stage
//! a game passes to [`shows`] for a feature, and release on that channel.

use std::cell::Cell;

/// The channels a player can follow, the default first, as
/// [`hearken::UpdateConfig::channels`] takes them.
pub const CHANNELS: [&str; 3] = ["stable", "beta", "alpha"];

/// The channels whose players install only packages built for them
/// ([`hearken::UpdateConfig::own_build`]). An alpha copy lists alpha-only
/// games, and a package built for beta or stable would hide them, so an
/// alpha player's copy ignores any update that is not an alpha package.
/// Releases publish only their own channel's manifest anyway
/// (`tools/check_channel_manifests.py`); this covers manifests already
/// published another way.
pub const OWN_BUILD: [&str; 1] = ["alpha"];

/// How ready a game or feature is, and so which copies show it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stage {
    /// On the edge: alpha copies only.
    Alpha,
    /// Being refined: beta and alpha copies.
    Beta,
    /// For every player.
    Stable,
}

impl Stage {
    /// The stage a channel's copies are built for. A name the game does not
    /// know is treated as stable, so nothing unready leaks through it.
    pub fn of_channel(channel: &str) -> Stage {
        match channel {
            "alpha" => Stage::Alpha,
            "beta" => Stage::Beta,
            _ => Stage::Stable,
        }
    }

    /// The channel's name, as Settings says it.
    pub fn channel(self) -> &'static str {
        match self {
            Stage::Alpha => "alpha",
            Stage::Beta => "beta",
            Stage::Stable => "stable",
        }
    }
}

/// Games not yet on stable, by id, with their stage. A game not listed is
/// stable.
pub const GAMES: &[(&str, Stage)] = &[("larkfield", Stage::Alpha), ("little-rapids", Stage::Alpha)];

/// The stage of the game `id`.
pub fn game_stage(id: &str) -> Stage {
    GAMES
        .iter()
        .find(|(g, _)| *g == id)
        .map_or(Stage::Stable, |(_, s)| *s)
}

thread_local! {
    static PINNED: Cell<Option<Stage>> = const { Cell::new(None) };
}

/// Makes this thread act as a copy built for `stage`, for tests; `None`
/// goes back to this build's own.
pub fn pin(stage: Option<Stage>) {
    PINNED.with(|p| p.set(stage));
}

/// The stage this copy was built for: its release channel's, or alpha for
/// a developer build, which shows everything.
pub fn copy_stage() -> Stage {
    if let Some(stage) = PINNED.with(Cell::get) {
        return stage;
    }
    hearken::update::Running::baked().map_or(Stage::Alpha, |r| Stage::of_channel(&r.channel))
}

/// Whether this copy shows work at `stage`.
pub fn shows(stage: Stage) -> bool {
    stage >= copy_stage()
}

/// Whether this copy lists the game `id`.
pub fn shows_game(id: &str) -> bool {
    shows(game_stage(id))
}

/// Keeps a player on the channel their copy came from. A player who
/// downloads a beta or alpha copy by hand has made no choice in Settings,
/// so their copy follows its own channel. Saving the choice preserves it
/// when a player manually installs a build from another channel.
/// Returns whether it saved.
pub fn keep_copy_channel(ctx: &mut hearken::Ctx<crate::Parlor>) -> bool {
    if ctx.settings.updates.channel.is_some() {
        return false;
    }
    let Some(channel) = ctx.updates.running().map(|r| r.channel.clone()) else {
        return false;
    };
    if channel == CHANNELS[0] || !CHANNELS.contains(&channel.as_str()) {
        return false;
    }
    ctx.settings.updates.channel = Some(channel);
    ctx.save_settings();
    true
}

#[cfg(test)]
mod own_build_tests {
    use hearken::Game;

    #[test]
    fn alpha_players_take_only_alpha_packages() {
        let config = crate::Parlor::updates().expect("released copies update");
        assert_eq!(config.own_build, ["alpha"]);
    }
}
