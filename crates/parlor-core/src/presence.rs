//! What Discord shows a player's friends: "Playing Wicket's", then where
//! they are in the collection. Only game names, never anything the player
//! typed or saved.

use hearken::{DiscordConfig, Presence};

use crate::ParlorData;

/// Wicket's application on the Discord developer portal. Discord shows the
/// application's name as the game being played. Empty keeps Discord off.
pub const DISCORD_APP_ID: &str = "1565374188229759016";

/// The engine's Discord settings for Wicket's, or `None` while there is no
/// application ID.
pub fn config() -> Option<DiscordConfig> {
    (!DISCORD_APP_ID.is_empty()).then(|| DiscordConfig::new(DISCORD_APP_ID))
}

/// Outside story mode: the open game, else the lobby.
pub fn outside_story(data: &ParlorData) -> Presence {
    match data.open_game_name {
        Some(name) => Presence::new(format!("Playing {name}")),
        None => Presence::new("In the lobby"),
    }
}

/// In story mode, with the scene's game when one is open.
pub fn in_story(data: &ParlorData) -> Presence {
    let presence = Presence::new("In story mode");
    match data.open_game_name {
        Some(name) => presence.detail(format!("Playing {name}")),
        None => presence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lobby_and_an_open_game() {
        let mut data = ParlorData::default();
        assert_eq!(outside_story(&data), Presence::new("In the lobby"));
        data.open_game_name = Some("Belfry");
        assert_eq!(outside_story(&data), Presence::new("Playing Belfry"));
    }

    #[test]
    fn story_mode_names_the_scenes_game() {
        let mut data = ParlorData::default();
        assert_eq!(in_story(&data), Presence::new("In story mode"));
        data.open_game_name = Some("Tightrope");
        assert_eq!(
            in_story(&data),
            Presence::new("In story mode").detail("Playing Tightrope")
        );
    }
}
