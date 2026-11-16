//! Recorded character lines, and the language they are recorded in.
//!
//! Character voices are recorded in English. In English the recording
//! plays. In any other language the game does what it does with the voice
//! switched off: a scene is read in the player's language, and a line
//! dropped into play is left out, since the game's own speech already says
//! everything a player needs.

use hearken::Ctx;

use crate::Parlor;

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
