//! The sound check: every cue of the meadow, by name.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Sound};
use parlor_core::Parlor;

use crate::rules::Kind;
use crate::sound::{self, drop_pos, ground};

/// How long a falling preview takes to land.
pub const DEMO_FALL: f64 = 2.6;
/// Lanes the lane tour walks.
const TOUR_LANES: usize = 5;
/// Seconds per lane of the tour.
const TOUR_STEP: f64 = 1.1;

enum Demo {
    None,
    /// A drop falling in the middle lane, then landing.
    Fall {
        kind: Kind,
        sound: Sound,
        left: f64,
    },
    /// A thump from each lane, left to right.
    Tour {
        next: usize,
        wait: f64,
    },
}

pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
    demo: Demo,
}

impl SoundCheck {
    pub fn new() -> Self {
        SoundCheck {
            menu: MenuCore::new("Sound check").with_intro_help(
                "Up and Down choose a sound, Enter plays it from its place in play, Escape goes back. Falling sounds fall from high to low and land.",
            ),
            demo: Demo::None,
        }
    }

    fn stop(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Demo::Fall { sound, .. } = self.demo {
            ctx.audio.stop(sound, 80);
        }
        self.demo = Demo::None;
    }

    fn fall(&mut self, ctx: &mut Ctx<Parlor>, kind: Kind) {
        ctx.stop_speech();
        self.stop(ctx);
        let pos = drop_pos(1, 3, 0.0);
        let s = ctx
            .audio
            .start_loop(sound::falling(kind), Some(pos), Bus::EFFECTS);
        ctx.audio.set_pitch(s, sound::fall_pitch(0.0));
        ctx.audio.set_volume(s, sound::fall_volume(0.0));
        self.demo = Demo::Fall {
            kind,
            sound: s,
            left: DEMO_FALL,
        };
    }

    /// Whether a demo is running (for tests).
    pub fn demo_running(&self) -> bool {
        !matches!(self.demo, Demo::None)
    }
}

impl Default for SoundCheck {
    fn default() -> Self {
        Self::new()
    }
}

/// The sound a drop makes as it lands in your basket.
fn landing(kind: Kind) -> &'static str {
    match kind {
        Kind::Egg => sound::CATCH_EGG,
        Kind::Golden => sound::CATCH_GOLDEN,
        Kind::Mud => sound::SPLAT,
        Kind::Bee => sound::STING,
    }
}

fn falling_row(label: &str, kind: Kind, help: &str) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(label, move |s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        s.fall(ctx, kind)
    })
    .select_sound(None)
    .help(help)
}

fn placed(label: &str, key: &'static str, help: &str) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(label, move |s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        ctx.stop_speech();
        s.stop(ctx);
        ctx.audio.play_at(key, ground(1, 3), Bus::EFFECTS);
    })
    .select_sound(None)
    .help(help)
}

fn plain(label: &str, key: &'static str, help: &str) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(label, move |s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        ctx.stop_speech();
        s.stop(ctx);
        ctx.audio.play(key, Bus::EFFECTS);
    })
    .select_sound(None)
    .help(help)
}

impl Menu for SoundCheck {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "basket_run_sounds"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        ctx.audio
            .set_listener(hearken::Vec3::new(0.0, 0.0, 0.0), 0.0);
        hearken::menu::menu_enter(self, ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.stop_speech();
        self.stop(ctx);
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        vec![
            falling_row("An egg falling", Kind::Egg, "A soft whistle that falls from high to low and grows louder as it nears the ground, ending as it drops into your basket. Lower and louder means sooner."),
            falling_row("A golden egg falling", Kind::Golden, "Sparkling little bells: a golden egg, worth five eggs."),
            falling_row("A mud clod falling", Kind::Mud, "A low, gloopy wobble, ending in the splat of mud landing in your basket. Dodge it: it spoils the eggs inside."),
            falling_row("A bee diving", Kind::Bee, "A buzz, ending in a sting. Dodge it: a sting breaks a basket."),
            MenuItem::new("Lanes, left to right", |s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
                ctx.stop_speech();
        s.stop(ctx);
                s.demo = Demo::Tour { next: 0, wait: 0.0 };
            })
            .select_sound(None)
            .help("A basket thump from each of five lanes in turn, from lane 1 on the far left to lane 5 on the far right, heard from the middle lane."),
            placed("Egg caught", sound::CATCH_EGG, "A woody thump: an egg in your basket."),
            placed("Golden egg caught", sound::CATCH_GOLDEN, "A thump and rising bells: a golden egg in your basket."),
            placed("Egg cracked", sound::CRACK_EGG, "A crunchy crack: an egg missed your basket and broke on the grass."),
            placed("Golden egg cracked", sound::CRACK_GOLDEN, "A crack and falling bells: a golden egg lost."),
            placed("Mud in your basket", sound::SPLAT, "A wet splat: mud landed in your basket and spoiled the eggs."),
            placed("Mud in the grass", sound::MUD_THUD, "A dull thud: a mud clod missed you."),
            placed("Bee sting", sound::STING, "A zap and a yelp: a bee stung you, and a basket breaks."),
            placed("Bee flying off", sound::BEE_AWAY, "A buzz fading away: a bee missed you."),
            plain("Basket full", sound::FULL, "Three wooden knocks: your basket is full. Take it to the cart."),
            plain("Eggs banked", sound::BANK, "Eggs rolling into the cart and a bright chime: banked as points."),
            plain("Nothing to bank", sound::EMPTY, "Two hollow knocks: the basket is empty, or the cart is not in your lane."),
            plain("Eggs spilled", sound::SPILL, "A tumble: eggs lost from the basket, to mud or to the bell."),
            placed("The cart's bell", sound::BEACON, "A soft bell every few seconds from the cart's lane."),
            plain("A step", sound::STEP, "Rustling grass: you moved one lane."),
            placed("Reaching the cart", sound::AT_CART, "A wood block: you stepped into the cart's lane."),
            plain("The fence", sound::EDGE, "A bump: the edge of the meadow."),
            plain("Go", sound::READY, "A rising thumb piano: the round begins."),
            plain("Ten seconds left", sound::WARNING, "A vibraphone call: ten seconds left in the round. Bank your eggs."),
            plain("Last seconds", sound::TICK, "A tick for each of the last five seconds."),
            plain("The bell", sound::BELL, "A deep bell: the round is over. Eggs still in the basket are lost."),
            plain("Streak", sound::STREAK, "A rising run: five eggs in a row, and every five more."),
            plain("A basket breaks", sound::LIFE_LOST, "Falling plucks: you lost a basket."),
            plain("Festival pattern begins", sound::PATTERN_CALL, "Pan pipes: in the Easter festival, a pattern of drops begins to fall. A voice names it: an egg sweep, a bee fence or a nest."),
            plain("Festival pattern caught", sound::PATTERN_DONE, "A cascade of bells: every egg of a festival pattern caught and every hazard dodged, for a bonus."),
            plain("Festival pattern broken", sound::PATTERN_BROKEN, "A low wooden flop: a festival pattern had a cracked egg, or one of its hazards hit you."),
            MenuItem::new("Back", |_s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| ctx.pop())
                .help("Return to the Basket Run title."),
        ]
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        match &mut self.demo {
            Demo::None => {}
            Demo::Fall {
                kind,
                sound: s,
                left,
            } => {
                *left -= dt;
                let p = (1.0 - *left / DEMO_FALL).clamp(0.0, 1.0);
                ctx.audio.set_position(*s, Some(drop_pos(1, 3, p)));
                ctx.audio.set_pitch(*s, sound::fall_pitch(p));
                ctx.audio.set_volume(*s, sound::fall_volume(p));
                if *left <= 0.0 {
                    let (kind, s) = (*kind, *s);
                    ctx.audio.stop(s, 40);
                    ctx.audio.play_at(landing(kind), ground(1, 3), Bus::EFFECTS);
                    self.demo = Demo::None;
                }
            }
            Demo::Tour { next, wait } => {
                *wait -= dt;
                if *wait <= 0.0 {
                    if *next >= TOUR_LANES {
                        self.demo = Demo::None;
                        return;
                    }
                    let lane = *next;
                    *next += 1;
                    *wait = TOUR_STEP;
                    let pos = ground(lane, TOUR_LANES);
                    ctx.audio.set_listener(
                        hearken::Vec3::new(sound::lane_x(2, TOUR_LANES), 0.0, 0.0),
                        0.0,
                    );
                    ctx.audio.play_at(sound::CATCH_EGG, pos, Bus::EFFECTS);
                }
            }
        }
    }
}
