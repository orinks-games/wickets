//! The menus around the wire: single crossings, challenges and the sound
//! check.

use hearken::menu::{menu_enter, Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::menus::{self as shared, labels, ChallengeGame};
use parlor_core::{Parlor, Period};

use crate::cues::{self, Play, SOUND_CHECK};
use crate::progress;
use crate::rules::{skyline_level, Mode, SKYLINE_LEN};
use crate::screens::{briefing, challenge_plan, period_text, start, Launch, CHALLENGES_NAME};
use crate::ID;

/// "best 23.4 seconds, relaxed best 40.1 seconds", leaving out one with
/// no time, or "no time yet".
pub fn time_text((best, relaxed): (Option<f64>, Option<f64>)) -> String {
    let secs = |t: f64| format!("{t:.1} seconds");
    parlor_core::pace::both("best", best.map(secs), relaxed.map(secs))
        .unwrap_or_else(|| "no time yet".to_string())
}

/// Every skyline crossing, to walk alone for its best time.
pub struct CrossingSelect {
    menu: MenuCore<Self, Parlor>,
    seed: Option<u64>,
}

impl CrossingSelect {
    pub fn new(seed: Option<u64>) -> CrossingSelect {
        CrossingSelect {
            menu: MenuCore::new("Crossing select").with_intro_help(
                "Up and Down choose a crossing, Enter walks it, Escape goes back. Crossings open as you reach them.",
            ),
            seed,
        }
    }
}

impl Menu for CrossingSelect {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "tightrope_crossings"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let open = progress::unlocked(ctx);
        let mut items = Vec::new();
        for n in 1..=SKYLINE_LEN {
            let level = skyline_level(n, false);
            if n <= open {
                let best = progress::best_times(ctx, &level.time_board());
                items.push(
                    MenuItem::new(
                        format!("Crossing {n}, {}, {}", level.route(), time_text(best)),
                        move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                            let launch = Launch {
                                mode: Mode::Single(n),
                                seed: s.seed,
                            };
                            start(ctx, launch, false)
                        },
                    )
                    .help(format!(
                        "{} floors up, {} steps. Par {:.0} seconds.",
                        level.floors,
                        level.steps,
                        level.par()
                    )),
                );
            } else {
                items.push(
                    MenuItem::new(
                        format!("Crossing {n}, locked"),
                        move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                            ctx.say(format!(
                                "Crossing {n} opens once you finish crossing {}.",
                                n - 1
                            ))
                        },
                    )
                    .select_sound(Some(cues::LOCKED))
                    .help("Reach this crossing in a skyline run or finish the one before it."),
                );
            }
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Tightrope title."),
        );
        items
    }
}

/// The game's side of the shared challenges menu.
pub struct Wires;

/// The daily, weekly and monthly challenges, then Back.
pub type ChallengeMenu = shared::ChallengesMenu<Wires>;

impl ChallengeGame for Wires {
    const ID: &'static str = ID;
    const NAME: &'static str = "Tightrope";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        challenge_plan(ctx, period).1.modifier.name().to_string()
    }

    fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        let summary = challenge_plan(ctx, period).1.summary();
        summary.trim_end_matches('.').to_string()
    }

    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        let (_, plan) = challenge_plan(ctx, period);
        format!("{} {}", period_text(period), plan.modifier.describe())
    }

    fn open(ctx: &mut Ctx<Parlor>, period: Period) {
        let pages = briefing(ctx, period);
        ctx.push(pages);
    }

    fn intro_help() -> String {
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. A new daily each day, a weekly each week, a monthly each month.".to_string()
    }
}

/// How long the sound check lets a loop or music play.
const LOOP_PREVIEW: f64 = 4.0;
const MUSIC_PREVIEW: f64 = 10.0;
/// The leans the balance demonstration walks through.
const DEMO_LEANS: [f64; 7] = [0.0, -0.25, -0.5, -0.85, 0.0, 0.5, 0.85];
const DEMO_STEP: f64 = 2.0;

enum Preview {
    None,
    Loop {
        sound: Sound,
        left: f64,
    },
    Music {
        left: f64,
    },
    /// The same sound again from the right.
    Sides {
        key: &'static str,
        wait: f64,
    },
    /// The balance tone at each demonstration lean in turn.
    Tone {
        sound: Sound,
        stage: usize,
        wait: f64,
    },
}

/// Every cue with its name and meaning: Enter plays it.
pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
    preview: Preview,
}

impl SoundCheck {
    pub fn new() -> SoundCheck {
        SoundCheck {
            menu: MenuCore::new("Sound check").with_intro_help(
                "Up and Down move through the sounds you need in play. Enter plays the selected sound. Escape goes back.",
            ),
            preview: Preview::None,
        }
    }

    fn stop(&mut self, ctx: &mut Ctx<Parlor>) {
        match std::mem::replace(&mut self.preview, Preview::None) {
            Preview::Loop { sound, .. } | Preview::Tone { sound, .. } => ctx.audio.stop(sound, 100),
            Preview::Music { .. } => ctx.audio.play_music(None, false),
            _ => {}
        }
    }

    fn play(&mut self, ctx: &mut Ctx<Parlor>, index: usize) {
        self.stop(ctx);
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        let entry = &SOUND_CHECK[index];
        match entry.play {
            Play::Once => {
                ctx.stop_speech();
                ctx.audio.play(entry.key, Bus::EFFECTS);
            }
            Play::Sides => {
                ctx.stop_speech();
                ctx.audio
                    .play_at(entry.key, Vec3::new(-3.0, 1.0, 0.0), Bus::EFFECTS);
                self.preview = Preview::Sides {
                    key: entry.key,
                    wait: 1.8,
                };
            }
            Play::Ahead => {
                ctx.stop_speech();
                ctx.audio
                    .play_at(entry.key, Vec3::new(0.0, 4.0, 0.0), Bus::EFFECTS);
            }
            Play::Loop => {
                ctx.stop_speech();
                let sound =
                    ctx.audio
                        .start_loop(entry.key, Some(Vec3::new(0.0, 2.0, 0.0)), Bus::EFFECTS);
                self.preview = Preview::Loop {
                    sound,
                    left: LOOP_PREVIEW,
                };
            }
            Play::Music => {
                ctx.stop_speech();
                ctx.audio
                    .play_music(Some(entry.key), entry.key == cues::MUSIC_TITLE);
                self.preview = Preview::Music {
                    left: MUSIC_PREVIEW,
                };
            }
        }
    }

    fn tone_at(ctx: &mut Ctx<Parlor>, sound: Sound, lean: f64) {
        let a = lean.abs() as f32;
        ctx.audio.set_volume(sound, 0.3 + 0.45 * a);
        ctx.audio.set_pitch(sound, 1.0 + 0.6 * a);
        ctx.audio
            .set_position(sound, Some(Vec3::new(lean as f32 * 4.0, 1.0, 0.0)));
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
        "tightrope_sounds"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        menu_enter(self, ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop(ctx);
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items = vec![MenuItem::new(
            "Balance tone",
            |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                s.stop(ctx);
                ctx.audio.set_listener(Vec3::ZERO, 0.0);
                let sound = ctx
                    .audio
                    .start_loop(cues::TONE, Some(Vec3::new(0.0, 1.0, 0.0)), Bus::EFFECTS);
                SoundCheck::tone_at(ctx, sound, 0.0);
                ctx.stop_speech();
                s.preview = Preview::Tone {
                    sound,
                    stage: 0,
                    wait: DEMO_STEP,
                };
            },
        )
        .select_sound(None)
        .help("The tone that is your balance, at each lean in turn: it moves to the side you lean to and rises as you lean further.")];
        for (i, entry) in SOUND_CHECK.iter().enumerate() {
            items.push(
                MenuItem::new(entry.name, move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    s.play(ctx, i)
                })
                .select_sound(None)
                .help(entry.meaning),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Tightrope title."),
        );
        items
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let mut done = false;
        match &mut self.preview {
            Preview::None => {}
            Preview::Loop { left, .. } | Preview::Music { left } => {
                *left -= dt;
                done = *left <= 0.0;
            }
            Preview::Sides { key, wait } => {
                *wait -= dt;
                if *wait <= 0.0 {
                    ctx.audio
                        .play_at(key, Vec3::new(3.0, 1.0, 0.0), Bus::EFFECTS);
                    done = true;
                }
            }
            Preview::Tone { sound, stage, wait } => {
                *wait -= dt;
                if *wait <= 0.0 {
                    *stage += 1;
                    *wait = DEMO_STEP;
                    match DEMO_LEANS.get(*stage) {
                        Some(&lean) => {
                            let sound = *sound;
                            SoundCheck::tone_at(ctx, sound, lean);
                        }
                        None => done = true,
                    }
                }
            }
        }
        if done {
            self.stop(ctx);
        }
    }
}
