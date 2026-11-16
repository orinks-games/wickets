//! Wicket's own language rules: recorded character lines, which are in
//! English, and moving old English settings to Automatic once.
//!
//! Character voices are recorded in English. In English the recording
//! plays. In any other language the game does what it does with the voice
//! switched off: a scene is read in the player's language, and a line
//! dropped into play is left out, since the game's own speech already says
//! everything a player needs.

use hearken::Ctx;

use crate::{Parlor, Records};

/// How a recorded line reaches the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineVoice {
    /// Its recording plays.
    Recording,
    /// The player's speech says it, or it is left out.
    Speech,
}

impl LineVoice {
    /// For the language the game is speaking now.
    pub fn now(ctx: &Ctx<Parlor>) -> LineVoice {
        if ctx.translator.is_english() {
            LineVoice::Recording
        } else {
            LineVoice::Speech
        }
    }
}

/// Whether recordings play: the game is speaking English.
pub fn recordings_play(ctx: &Ctx<Parlor>) -> bool {
    LineVoice::now(ctx) == LineVoice::Recording
}

/// The record that says the language setting has been moved to Automatic.
const AUTO_ONCE_KEY: &str = "language_auto_once";

/// Players from before translations have English saved, though they never
/// chose it: there was no Language row. Once, the first time the lobby
/// opens, an English setting becomes Automatic, so a player whose computer
/// speaks Spanish hears Spanish. A language chosen after that is kept.
pub fn follow_system_once(ctx: &mut Ctx<Parlor>) {
    if Records::load::<bool>(ctx, AUTO_ONCE_KEY) {
        return;
    }
    Records::store(ctx, AUTO_ONCE_KEY, &true);
    if ctx.settings.language == hearken::l10n::ENGLISH {
        ctx.set_language(hearken::l10n::AUTO);
    }
}
