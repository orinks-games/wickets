//! The launcher's music: lobby pieces that take turns, the Challenges
//! board's own theme, and story mode's soundtrack. Every launcher menu asks for its music as it is
//! entered or revealed, so coming back from a game (whose title stops its
//! own music as it closes) brings the launcher's music back.

use hearken::Ctx;
use parlor_core::story::{self, PartOfDay};
use parlor_core::{Parlor, Rng};

const LOUNGE: &str = "parlor/music/lobby_lounge";
const BOSSA: &str = "parlor/music/lobby_bossa";
const RAG: &str = "parlor/music/lobby_rag";
const SWING: &str = "parlor/music/lobby_swing";
const WALTZ: &str = "parlor/music/lobby_waltz";
const MORNING: &str = "parlor/music/lobby_morning";
const AFTERNOON: &str = "parlor/music/lobby_afternoon";
const EVENING: &str = "parlor/music/lobby_evening";
const LATE: &str = "parlor/music/lobby_small_hours";

/// Every lobby piece: a lo-fi lounge, a bossa nova, a parlor rag, a light
/// swing and a musette waltz for most of the day, and one piece each for
/// the morning, the afternoon, the evening and the small hours.
pub const LOBBY: [&str; 9] = [
    LOUNGE, BOSSA, RAG, SWING, WALTZ, MORNING, AFTERNOON, EVENING, LATE,
];

/// The lobby's pieces for a part of the player's day, in the order they
/// take turns. Each part leads with its own piece; the small hours keep to
/// the two quietest.
pub fn lobby_set(part: PartOfDay) -> &'static [&'static str] {
    match part {
        PartOfDay::Dawn => &[MORNING, LOUNGE, BOSSA, WALTZ],
        PartOfDay::Morning => &[MORNING, BOSSA, RAG, SWING, WALTZ, LOUNGE],
        PartOfDay::Afternoon => &[AFTERNOON, BOSSA, RAG, SWING, WALTZ, LOUNGE],
        PartOfDay::Evening => &[EVENING, LOUNGE, BOSSA, SWING, WALTZ, RAG],
        PartOfDay::Night => &[EVENING, LOUNGE, SWING, WALTZ, BOSSA],
        PartOfDay::SmallHours => &[LATE, LOUNGE],
    }
}

/// The Challenges board and its lists.
pub const CHALLENGES: &str = "parlor/music/challenges";

/// Keeps a lobby piece playing. One already playing carries on; otherwise
/// the next in turn for this part of the day starts, so returning from a
/// game or the Challenges board moves on to a different piece. The
/// launcher opens on the part of the day's own piece.
pub fn lobby(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    if ctx.audio.music().is_some_and(|m| LOBBY.contains(&m)) {
        return;
    }
    let set = lobby_set(PartOfDay::of_hour(story::now(ctx).hour));
    let turn = ctx.game.lobby_turn.unwrap_or(0);
    ctx.game.lobby_turn = Some(turn.wrapping_add(1));
    ctx.audio
        .play_music(Some(set[turn as usize % set.len()]), true);
}

/// Plays the Challenges theme; already playing, it carries on.
pub fn challenges(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(Some(CHALLENGES), true);
}

/// Story mode's own pieces for the sitting, which take turns like the
/// lobby's: the story's theme, Mabel's fireside piece and a lamplit folk
/// tune.
pub const STORY_SITTING: [&str; 3] = [
    "parlor/music/story_theme",
    "parlor/music/story_fireside",
    "parlor/music/story_lamplit",
];

/// Wicket's piece, for a sitting in the small hours.
pub const SMALL_HOURS: &str = "parlor/music/lobby_small_hours";

/// The names a story file's `music:` line may use, and their pieces.
pub const STORY_SCENES: [(&str, &str); 0] = [];

/// The piece a story file's `music:` line names.
pub fn story_piece(name: &str) -> Option<&'static str> {
    STORY_SCENES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, piece)| *piece)
}

/// Keeps story mode's music playing in the sitting: Wicket's piece in the
/// small hours, else a sitting piece. One already playing carries on;
/// otherwise the next in turn starts, so coming back from a game or a
/// scene brings a different one.
pub fn story_sitting(ctx: &mut Ctx<Parlor>, small_hours: bool) {
    // No game's ambience carries on under the story.
    ctx.audio.set_bed(None);
    if small_hours {
        scene(ctx, SMALL_HOURS);
        return;
    }
    if ctx
        .audio
        .music()
        .is_some_and(|m| STORY_SITTING.contains(&m))
    {
        return;
    }
    let turn = ctx
        .game
        .story_turn
        .unwrap_or_else(|| Rng::from_time().below(STORY_SITTING.len() as u32));
    ctx.game.story_turn = Some((turn + 1) % STORY_SITTING.len() as u32);
    ctx.audio.play_music(
        Some(STORY_SITTING[turn as usize % STORY_SITTING.len()]),
        true,
    );
}

/// Plays a story scene's piece; already playing, it carries on.
pub fn scene(ctx: &mut Ctx<Parlor>, piece: &'static str) {
    if ctx.audio.music() != Some(piece) {
        ctx.audio.play_music(Some(piece), true);
    }
}
