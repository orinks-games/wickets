//! The sound check: every sound a skater must learn, each from where it
//! is heard on the ice.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::menus::labels;
use parlor_core::Parlor;

use crate::rules::lane_x;
use crate::sound::{self, place, AHEAD_FROM};

/// How long a sound takes to come up to you, seconds.
const DEMO_SECS: f64 = 3.0;

enum Demo {
    None,
    /// A looped sound coming toward you in `lane`, then what it does as it
    /// reaches you.
    Approach {
        sound: Sound,
        lane: i32,
        left: f64,
        then: Option<&'static str>,
    },
}

pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
    demo: Demo,
}

impl SoundCheck {
    pub fn new() -> SoundCheck {
        SoundCheck {
            menu: MenuCore::new("Sound check").with_intro_help(
                "Up and Down choose a sound, Enter plays it from where you would hear it, Escape goes back.",
            ),
            demo: Demo::None,
        }
    }

    fn stop(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.stop_speech();
        if let Demo::Approach { sound, .. } = self.demo {
            ctx.audio.stop(sound, 80);
        }
        self.demo = Demo::None;
    }

    fn approach(
        &mut self,
        ctx: &mut Ctx<Parlor>,
        key: &str,
        lane: i32,
        then: Option<&'static str>,
    ) {
        self.stop(ctx);
        let at = place(lane_x(lane), 0.0, AHEAD_FROM);
        let s = ctx.audio.start_loop(key, Some(at), Bus::EFFECTS);
        self.demo = Demo::Approach {
            sound: s,
            lane,
            left: DEMO_SECS,
            then,
        };
    }

    pub fn demo_running(&self) -> bool {
        !matches!(self.demo, Demo::None)
    }
}

impl Default for SoundCheck {
    fn default() -> Self {
        SoundCheck::new()
    }
}

fn coming(
    label: &str,
    key: &'static str,
    lane: i32,
    then: Option<&'static str>,
    help: &str,
) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(label, move |s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        s.approach(ctx, key, lane, then)
    })
    .select_sound(None)
    .help(help)
}

fn plain(label: &str, key: &'static str, at: Vec3, help: &str) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(label, move |s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        s.stop(ctx);
        ctx.audio.play_at(key, at, Bus::EFFECTS);
    })
    .select_sound(None)
    .help(help)
}

impl Menu for SoundCheck {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "snowfall_skate_sounds"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        ctx.audio.set_bed(None);
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        hearken::menu::menu_enter(self, ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop(ctx);
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let ahead = sound::lane_place(0);
        vec![
            plain("A lantern lights", LIGHT, sound::lane_place(-1), "A soft bell, in the lantern's lane. Each lantern rings its own note."),
            coming("A lantern ahead", sound::FLAME, -1, Some(TAKE), "A crackling flame coming toward you, then the chime of taking it. Be in its lane as it reaches you."),
            plain("A lantern taken", TAKE, sound::UNDER, "A bright chime as you skate through the lantern. Your line grows by one."),
            plain("A lantern gone by", sound::PASSED, sound::place(lane_x(2), 0.0, -1.0), "A low falling note behind you: a lantern you missed. Your line starts again."),
            plain("The wrong lantern", sound::WRONG, sound::UNDER, "A flat clink: on the lantern skate, the second lit of a pair. Your line starts again."),
            coming("A drift ahead", sound::DRIFT, 1, Some(sound::DRIFTED), "Soft blown snow coming toward you, then the hush of skating into it. Lean out of its way."),
            plain("Into a drift", sound::DRIFTED, sound::UNDER, "A soft crunch and a slide. You slow, and your line starts again."),
            coming("Thin ice ahead", sound::THIN, -2, Some(sound::CRACK), "Ice creaking and ticking coming toward you, then the crack of skating onto it. Lean out of its way."),
            plain("The ice cracks", sound::CRACK, sound::UNDER, "A sharp crack under you. You stumble for a moment, and the snow gains on the ice."),
            coming("Your partner", sound::PARTNER, 1, None, "Another pair of blades, a little ahead. They skate toward each lantern and round the drifts."),
            plain("A spin, ready", sound::SPIN_READY, ahead, "A rising shimmer while you hold the spin: let go now to land it."),
            plain("A spin landed", sound::SPIN_LAND, sound::UNDER, "A clean scrape and a ring. The spin scores, more as the snow deepens."),
            plain("A wobble", sound::WOBBLE, sound::UNDER, "A scuffing stagger: the spin was held too long. It scores nothing, and your line starts again."),
            MenuItem::new(labels::BACK, |_s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| ctx.pop())
                .help("Return to the Snowfall Skate title."),
        ]
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if let Demo::Approach {
            sound: s,
            lane,
            left,
            then,
        } = &mut self.demo
        {
            *left -= dt;
            let ahead = (*left / DEMO_SECS) as f32 * AHEAD_FROM;
            ctx.audio
                .set_position(*s, Some(place(lane_x(*lane), 0.0, ahead)));
            if *left <= 0.0 {
                let (s, then) = (*s, *then);
                ctx.audio.stop(s, 60);
                if let Some(key) = then {
                    ctx.audio.play_at(key, sound::UNDER, Bus::EFFECTS);
                }
                self.demo = Demo::None;
            }
        }
    }
}

const LIGHT: &str = "snowfall-skate/lantern/light0";
const TAKE: &str = "snowfall-skate/lantern/take0";
