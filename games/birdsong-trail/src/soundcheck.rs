//! The sound check: every voice, step and cue of the game, by name.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx};
use parlor_core::menus::labels;
use parlor_core::Parlor;

use crate::birds::{self, Note, Voice};
use crate::sound::{self, Tune};
use crate::trails::{Ground, Trail};

/// The sound check's `State::name`.
pub const SOUNDS_NAME: &str = "birdsong_trail_sounds";

/// One row: a label, what it means, and what it plays.
pub struct Row {
    pub label: String,
    pub meaning: String,
    pub keys: Vec<(String, f32)>,
    pub bus: Bus,
}

fn row(label: impl Into<String>, meaning: impl Into<String>, key: impl Into<String>) -> Row {
    Row {
        label: label.into(),
        meaning: meaning.into(),
        keys: vec![(key.into(), 1.0)],
        bus: Bus::EFFECTS,
    }
}

/// Which birds sing with a voice, for its row.
fn singers(voice: Voice) -> String {
    let names: Vec<&str> = birds::ALL
        .iter()
        .filter(|s| s.voice == voice)
        .take(3)
        .map(|s| s.name)
        .collect();
    names.join(", ")
}

/// Every row of the sound check, in order.
pub fn rows() -> Vec<Row> {
    let mut out = Vec::new();
    out.push(Row {
        label: "Your whistle".to_string(),
        meaning: "Your whistle: the notes you answer with, low, middle and high.".to_string(),
        keys: Note::ALL
            .iter()
            .map(|n| (sound::you_key(*n), 1.0))
            .collect(),
        bus: Bus::EFFECTS,
    });
    for voice in Voice::ALL {
        let label = match voice {
            Voice::Honk | Voice::Bugle | Voice::Whoop => format!("Flock call, {}", voice.slug()),
            _ => format!("Bird voice, {}", voice.slug()),
        };
        let keys = if voice.is_call() {
            vec![(sound::voice_key(voice, Note::Mid), 1.0)]
        } else {
            Note::ALL
                .iter()
                .map(|n| (sound::voice_key(voice, *n), 1.0))
                .collect()
        };
        let meaning = if voice.is_call() {
            format!("{label}: {}, heard in the migration flocks.", voice.words())
        } else {
            format!(
                "{label}: {}, low, middle and high. Sung by the {} and others, each at its own pitch.",
                voice.words(),
                singers(voice)
            )
        };
        out.push(Row {
            label,
            meaning,
            keys,
            bus: Bus::EFFECTS,
        });
    }
    for ground in Ground::ALL {
        out.push(row(
            format!("Step, {}", ground.slug()),
            format!("Step, {}: {}.", ground.slug(), ground.words()),
            sound::step_key(ground),
        ));
    }
    out.push(row(
        "Turn",
        "Turn: a soft rustle as you turn to face a new way.",
        sound::TURN,
    ));
    out.push(row(
        "Edge of the trail",
        "Edge of the trail: a rustle and a soft thud. Brambles, a hedge or water stop the step.",
        sound::BUMP,
    ));
    out.push(row(
        "Befriended",
        "Befriended: a warm rising chime. Your answer matched, and the bird is your friend.",
        sound::BEFRIEND,
    ));
    out.push(row(
        "New song",
        "New song: a soft page turn and a bell. Something new went into your field guide.",
        sound::NEW_SONG,
    ));
    out.push(row(
        "Not quite",
        "Not quite: two soft, low notes. Your answer did not match, and the bird sings again.",
        sound::AGAIN,
    ));
    out.push(row(
        "Found",
        "Found: a bright two-note chime. You picked out the right bird, or counted a flock exactly.",
        sound::FOUND,
    ));
    out.push(row(
        "Trail's end",
        "Trail's end: a gentle rising chord. You reached the end of the trail, or the last flock has gone.",
        sound::ARRIVE,
    ));
    out.push(row(
        "Wingbeats",
        "Wingbeats: a soft rush of wings. A migrating flock is coming.",
        sound::WINGS,
    ));
    for trail in Trail::ALL {
        let label = format!("End of the {}", trail.name());
        out.push(Row {
            meaning: format!(
                "{label}: {}. It sounds from the trail's end the whole walk, so you can walk toward it.",
                trail.end_words()
            ),
            label,
            keys: vec![(sound::end_key(trail), 1.0)],
            bus: Bus::AMBIENCE,
        });
    }
    out
}

pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
    tune: Tune,
}

impl SoundCheck {
    pub fn new() -> Self {
        SoundCheck {
            menu: MenuCore::new(labels::SOUND_CHECK).with_intro_help(
                "Up and Down choose a sound, Enter plays it and says what it means, Escape goes back.",
            ),
            tune: Tune::new(),
        }
    }
}

impl Default for SoundCheck {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for SoundCheck {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        SOUNDS_NAME
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        hearken::menu::menu_enter(self, ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.tune.stop(ctx);
        sound::title_music(ctx);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        self.tune.tick(ctx, dt);
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items: Vec<MenuItem<Self, Parlor>> = Vec::new();
        for r in rows() {
            let help = r
                .meaning
                .split_once(": ")
                .map(|(_, m)| hearken::text::sentence_start(m))
                .unwrap_or_else(|| r.meaning.clone());
            let Row {
                label,
                meaning,
                keys,
                bus,
            } = r;
            items.push(
                MenuItem::new(label, move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    if keys.len() == 1 && bus != Bus::EFFECTS {
                        s.tune.one(ctx, &keys[0].0, bus);
                    } else {
                        s.tune.start(ctx, keys.clone());
                    }
                    ctx.say(meaning.clone());
                })
                .select_sound(None)
                .help(help),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Birdsong Trail title."),
        );
        items
    }
}
