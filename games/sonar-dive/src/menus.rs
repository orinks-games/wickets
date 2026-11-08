//! The title's side rooms: challenges, the sound check and the tutorial.

use hearken::menu::{menu_enter, Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::menus::{self as shared, labels, ChallengeGame};
use parlor_core::{Challenge, Parlor, Period};

use crate::cues::{self, Play, CUES};
use crate::screens::{briefing, challenge_name, challenge_rule, challenge_tag, CHALLENGES_NAME};
use crate::ID;

/// The game's side of the shared challenges menu.
pub struct Dives;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = shared::ChallengesMenu<Dives>;

impl ChallengeGame for Dives {
    const ID: &'static str = ID;
    const NAME: &'static str = "Sonar Dive";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        challenge_name(period).to_string()
    }

    fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        challenge_tag(&Challenge::current(ctx, ID, period))
    }

    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        challenge_rule(&Challenge::current(ctx, ID, period))
    }

    fn open(ctx: &mut Ctx<Parlor>, period: Period) {
        let pages = briefing(ctx, period);
        ctx.push(pages);
    }

    fn intro_help() -> String {
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. A new daily each day, a weekly each Monday, a monthly each month.".to_string()
    }
}

/// Every cue with its name, and a demonstration of echo directions.
pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
    preview: Sound,
    demo: Vec<(f64, Vec3, String)>,
    demo_in: f64,
}

impl SoundCheck {
    pub fn new() -> SoundCheck {
        SoundCheck {
            menu: MenuCore::new("Sound check").with_intro_help(
                "Up and Down move through the cues, Enter plays one, Escape goes back.",
            ),
            preview: Sound::NONE,
            demo: Vec::new(),
            demo_in: 0.0,
        }
    }

    fn stop_preview(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.stop(self.preview, 200);
        self.preview = Sound::NONE;
        self.demo.clear();
    }
}

impl Default for SoundCheck {
    fn default() -> Self {
        Self::new()
    }
}

/// The direction demonstration: a wreck echo placed around you. A `#`
/// in the words is the distance, said in the player's units.
const DEMO: [(f32, f32, &str); 5] = [
    (0.0, 10.0, "Straight ahead, #."),
    (10.0, 0.0, "To your right, at 3 o'clock."),
    (0.0, -10.0, "Behind you, at 6 o'clock: muffled."),
    (-10.0, 0.0, "To your left, at 9 o'clock."),
    (0.0, 28.0, "Far ahead, #: quieter, and it comes back later."),
];

impl Menu for SoundCheck {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "sonar_dive_sounds"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        ctx.audio.spatial = cues::spatial();
        menu_enter(self, ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_preview(ctx);
        ctx.audio.play_music(None, false);
        ctx.audio.spatial = hearken::SpatialConfig::default();
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.demo.is_empty() {
            return;
        }
        self.demo_in -= dt;
        if self.demo_in <= 0.0 {
            let (wait, pos, _text) = self.demo.remove(0);
            ctx.audio.play_at(cues::ECHO_WRECK, pos, Bus::EFFECTS);
            self.demo_in = wait;
        }
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items = vec![MenuItem::new(
            "Echo directions",
            |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                s.stop_preview(ctx);
                ctx.audio.play_music(None, false);
                s.demo = DEMO
                    .iter()
                    .map(|&(x, y, text)| {
                        // Speech counts 2 metres for each unit of the sound's place.
                        let d = parlor_core::units::length(f64::from(2.0 * x.hypot(y)));
                        (2.5, Vec3::new(x, y, 0.0), text.replace('#', &d))
                    })
                    .collect();
                s.demo_in = 0.0;
                ctx.stop_speech();
            },
        )
        .select_sound(None)
        .help(
            "A wreck echo from five places around you, to learn how direction and distance sound.",
        )];
        for cue in CUES {
            items.push(
                MenuItem::new(cue.name, move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    s.stop_preview(ctx);
                    match cue.play {
                        Play::Ahead => {
                            ctx.audio
                                .play_at(cue.key, Vec3::new(0.0, 4.0, 0.0), Bus::EFFECTS);
                        }
                        Play::Flat => {
                            ctx.audio.play(cue.key, Bus::EFFECTS);
                        }
                        Play::Loop => {
                            ctx.audio.play_music(None, false);
                            let bus = if cue.key.contains("/amb/") {
                                Bus::AMBIENCE
                            } else {
                                Bus::EFFECTS
                            };
                            s.preview = ctx.audio.start_loop(cue.key, None, bus);
                        }
                        Play::Music => {
                            ctx.audio
                                .play_music(Some(cue.key), cue.key == cues::MUSIC_TITLE);
                        }
                    }
                    ctx.stop_speech();
                })
                .select_sound(None)
                .help(cue.meaning),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Sonar Dive title."),
        );
        items
    }
}
