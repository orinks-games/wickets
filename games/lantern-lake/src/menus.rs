//! The menus beside the lake: challenges, the journal and the sound check.

use hearken::menu::{menu_enter, Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::menus::{self as shared, labels, ChallengeGame};
use parlor_core::{Parlor, Period};

use crate::cues::{self, Play, SOUND_CHECK};
use crate::journal::Journal;
use crate::rules::Feature;
use crate::screens::{briefing, challenge_plan, period_text, CHALLENGES_NAME};
use crate::ID;

/// The game's side of the shared challenges menu.
pub struct Lakes;

/// The daily, weekly and monthly challenges, then Back.
pub type ChallengeMenu = shared::ChallengesMenu<Lakes>;

impl ChallengeGame for Lakes {
    const ID: &'static str = ID;
    const NAME: &'static str = "Lantern Lake";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        let (_, plan) = challenge_plan(ctx, period);
        plan.twist.map(|t| t.name().to_string()).unwrap_or_default()
    }

    fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        let summary = challenge_plan(ctx, period).1.summary();
        summary.trim_end_matches('.').to_string()
    }

    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        let (_, plan) = challenge_plan(ctx, period);
        let twist = plan.twist.map(|t| t.describe()).unwrap_or_default();
        format!("{} {twist}", period_text(period))
    }

    fn open(ctx: &mut Ctx<Parlor>, period: Period) {
        let pages = briefing(ctx, period);
        ctx.push(pages);
    }

    fn intro_help() -> String {
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. A new daily each day, a weekly each week, a monthly each month.".to_string()
    }
}

/// The journal: every place, found or still waiting, and the tallies.
pub struct JournalMenu {
    menu: MenuCore<Self, Parlor>,
}

impl JournalMenu {
    pub fn new() -> JournalMenu {
        JournalMenu {
            menu: MenuCore::new("Your journal").with_intro_help(
                "Everything you have found on the lake. Up and Down read the pages, Escape goes back.",
            ),
        }
    }
}

impl Default for JournalMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for JournalMenu {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "lantern_lake_journal"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let j = Journal::load(ctx);
        let mut items = vec![MenuItem::new(
            format!(
                "Nights finished: {}. Outings: {}. Lanterns gathered: {}.",
                j.nights, j.outings, j.lanterns
            ),
            |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.say("Your tallies."),
        )
        .select_sound(None)];
        for f in Feature::ALL {
            let (label, help) = match j.found.get(f.id()) {
                Some(&n) => {
                    let times = if n == 1 {
                        "found once".to_string()
                    } else {
                        format!("found {n} times")
                    };
                    (
                        format!("{}, {times}", crate::rules::sentence(f.name())),
                        f.found_text().to_string(),
                    )
                }
                None => (
                    "A blank page: not yet found".to_string(),
                    format!("Somewhere on some night, listen for {}.", f.heard()),
                ),
            };
            let say = help.clone();
            items.push(
                MenuItem::new(label, move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.say(say.clone())
                })
                .select_sound(None)
                .help(help),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Lantern Lake title."),
        );
        items
    }
}

/// How long the sound check lets a loop or music play.
const LOOP_PREVIEW: f64 = 5.0;
const MUSIC_PREVIEW: f64 = 9.0;

enum Preview {
    None,
    Loop { sound: Sound, left: f64 },
    Music { sound: Sound, left: f64 },
    Sides { key: &'static str, wait: f64 },
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
                "Up and Down move through the sounds you need in play. Enter plays the one you are on. Escape goes back.",
            ),
            preview: Preview::None,
        }
    }

    fn stop(&mut self, ctx: &mut Ctx<Parlor>) {
        match std::mem::replace(&mut self.preview, Preview::None) {
            Preview::Loop { sound, .. } | Preview::Music { sound, .. } => {
                ctx.audio.stop(sound, 300)
            }
            Preview::Sides { .. } | Preview::None => {}
        }
    }

    fn play(&mut self, ctx: &mut Ctx<Parlor>, index: usize) {
        ctx.stop_speech();
        self.stop(ctx);
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        let entry = &SOUND_CHECK[index];
        match entry.play {
            Play::Once => {
                ctx.audio.play(entry.key, Bus::EFFECTS);
            }
            Play::Sides => {
                ctx.audio
                    .play_at(entry.key, Vec3::new(-4.0, 1.0, 0.0), Bus::EFFECTS);
                self.preview = Preview::Sides {
                    key: entry.key,
                    wait: 2.0,
                };
            }
            Play::Loop => {
                let sound =
                    ctx.audio
                        .start_loop(entry.key, Some(Vec3::new(0.0, 3.0, 0.0)), Bus::EFFECTS);
                self.preview = Preview::Loop {
                    sound,
                    left: LOOP_PREVIEW,
                };
            }
            Play::Music => {
                let sound = ctx.audio.start_loop(entry.key, None, Bus::MUSIC);
                self.preview = Preview::Music {
                    sound,
                    left: MUSIC_PREVIEW,
                };
            }
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
        "lantern_lake_sounds"
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
        let mut items = Vec::new();
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
            .help("Return to the Lantern Lake title."),
        );
        items
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let mut done = false;
        match &mut self.preview {
            Preview::None => {}
            Preview::Loop { left, .. } | Preview::Music { left, .. } => {
                *left -= dt;
                done = *left <= 0.0;
            }
            Preview::Sides { key, wait } => {
                *wait -= dt;
                if *wait <= 0.0 {
                    ctx.audio
                        .play_at(key, Vec3::new(4.0, 1.0, 0.0), Bus::EFFECTS);
                    done = true;
                }
            }
        }
        if done {
            self.stop(ctx);
        }
    }
}

/// Every cue's key, for tests.
pub fn all_keys() -> Vec<&'static str> {
    let mut keys: Vec<&'static str> = SOUND_CHECK.iter().map(|e| e.key).collect();
    keys.push(cues::SONG_BASE);
    keys.sort();
    keys.dedup();
    keys
}
