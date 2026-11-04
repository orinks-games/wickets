use hearken::{Action, Chord, ControllerButton, Key, PadChord};

/// The controls every game shares, rebindable from Settings.
///
/// Games read them with `ctx.action_for(event)` and `ctx.action_held(..)`,
/// and name them with `ctx.control_name(..)` so spoken instructions follow
/// the player's own bindings. A game may also read keys of its own (number
/// keys, letters) straight from the event.
///
/// On a controller, Hearken turns A into Enter, B and Start into Escape and
/// the d-pad into the arrows before a game sees them, so the shared
/// actions avoid B and Start; A is both Enter and the main action, and a
/// game that reads Enter during play should check its actions first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParlorAction {
    Left,
    Right,
    Up,
    Down,
    /// The main action: launch, fire, grab, ring.
    Primary,
    /// A second action: boost, shake, use a tool.
    Secondary,
    /// Speak the current state: score, lives, what is around you.
    Status,
    /// Repeat the last prompt or pattern.
    Repeat,
    /// Ask for a hint, where the game has one.
    Hint,
}

impl Action for ParlorAction {
    fn all() -> &'static [Self] {
        use ParlorAction::*;
        &[
            Left, Right, Up, Down, Primary, Secondary, Status, Repeat, Hint,
        ]
    }
    fn id(self) -> &'static str {
        match self {
            ParlorAction::Left => "left",
            ParlorAction::Right => "right",
            ParlorAction::Up => "up",
            ParlorAction::Down => "down",
            ParlorAction::Primary => "primary",
            ParlorAction::Secondary => "secondary",
            ParlorAction::Status => "status",
            ParlorAction::Repeat => "repeat",
            ParlorAction::Hint => "hint",
        }
    }
    fn label(self) -> &'static str {
        match self {
            ParlorAction::Left => "Left",
            ParlorAction::Right => "Right",
            ParlorAction::Up => "Up",
            ParlorAction::Down => "Down",
            ParlorAction::Primary => "Main action",
            ParlorAction::Secondary => "Second action",
            ParlorAction::Status => "Status",
            ParlorAction::Repeat => "Repeat",
            ParlorAction::Hint => "Hint",
        }
    }
    fn default_keys(self) -> &'static [Chord] {
        match self {
            ParlorAction::Left => const { &[Chord::plain(Key::Left)] },
            ParlorAction::Right => const { &[Chord::plain(Key::Right)] },
            ParlorAction::Up => const { &[Chord::plain(Key::Up)] },
            ParlorAction::Down => const { &[Chord::plain(Key::Down)] },
            ParlorAction::Primary => const { &[Chord::plain(Key::Space)] },
            ParlorAction::Secondary => const { &[Chord::plain(Key::Tab)] },
            ParlorAction::Status => const { &[Chord::plain(Key::S)] },
            ParlorAction::Repeat => const { &[Chord::plain(Key::R)] },
            ParlorAction::Hint => const { &[Chord::plain(Key::H)] },
        }
    }
    fn default_pad(self) -> &'static [PadChord] {
        match self {
            ParlorAction::Left => const { &[PadChord::plain(ControllerButton::DPadLeft)] },
            ParlorAction::Right => const { &[PadChord::plain(ControllerButton::DPadRight)] },
            ParlorAction::Up => const { &[PadChord::plain(ControllerButton::DPadUp)] },
            ParlorAction::Down => const { &[PadChord::plain(ControllerButton::DPadDown)] },
            ParlorAction::Primary => const { &[PadChord::plain(ControllerButton::A)] },
            ParlorAction::Secondary => const { &[PadChord::plain(ControllerButton::X)] },
            ParlorAction::Status => const { &[PadChord::plain(ControllerButton::Y)] },
            ParlorAction::Repeat => const { &[PadChord::plain(ControllerButton::RightStick)] },
            ParlorAction::Hint => const { &[PadChord::plain(ControllerButton::LeftShoulder)] },
        }
    }
}
