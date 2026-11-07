//! The stream list, the boatyard and the sound check.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::menus::labels;
use parlor_core::Parlor;

use crate::progress::{best_line, ribbons_of, Progress};
use crate::rules::{stars_phrase, Boat, Kind, Mode, Stream, HALF};
use crate::run::{start, Launch};
use crate::sound;

// --- the streams -----------------------------------------------------------

/// Every stream: its stars and best, or what opens it.
pub struct Streams {
    menu: MenuCore<Self, Parlor>,
    seed: Option<u64>,
}

impl Streams {
    pub fn new(seed: Option<u64>) -> Streams {
        Streams {
            menu: MenuCore::new("Streams").with_intro_help(
                "Up and Down choose a stream, Enter sets your boat on it, Escape goes back.",
            ),
            seed,
        }
    }
}

impl Menu for Streams {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "little_rapids_streams"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let progress = Progress::load(ctx);
        let mut items = Vec::new();
        for stream in Stream::ALL {
            if progress.is_open(stream) {
                let best = best_line(ctx, stream.id());
                let stars = progress.stars_for(stream);
                let ribbons: Vec<String> = ribbons_of(stream)
                    .map(|r| {
                        if progress.has_ribbon(r.id) {
                            format!("{}, done", r.name)
                        } else {
                            r.name.to_string()
                        }
                    })
                    .collect();
                let help = format!(
                    "{}. Ribbons: {}.",
                    hearken::text::sentence_start(stream.twist()),
                    ribbons.join("; ")
                );
                items.push(
                    MenuItem::new(
                        format!("{}, {}, {best}", stream.name(), stars_phrase(stars)),
                        move |s: &mut Streams, ctx: &mut Ctx<Parlor>| {
                            start(
                                ctx,
                                Launch {
                                    mode: Mode::Stream(stream),
                                    seed: s.seed,
                                },
                                false,
                            )
                        },
                    )
                    .help(help),
                );
            } else {
                let need = stream.stars_to_open() - progress.total_stars();
                let text = format!(
                    "{} opens with {} more. You have {} in all.",
                    stream.name(),
                    stars_phrase(need),
                    stars_phrase(progress.total_stars())
                );
                let help = text.clone();
                items.push(
                    MenuItem::new(
                        labels::locked(stream.name()),
                        move |_s: &mut Streams, ctx: &mut Ctx<Parlor>| ctx.say(text.clone()),
                    )
                    .select_sound(None)
                    .help(help),
                );
            }
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Streams, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Little Rapids title."),
        );
        items
    }
}

// --- the boatyard ----------------------------------------------------------

/// Fold boats from paper scraps, and choose which one to sail.
pub struct Boatyard {
    menu: MenuCore<Self, Parlor>,
}

impl Boatyard {
    pub fn new() -> Boatyard {
        Boatyard {
            menu: MenuCore::new("Boatyard").with_intro_help(
                "Up and Down choose a boat, Enter sails it or folds it, Escape goes back.",
            ),
        }
    }
}

impl Default for Boatyard {
    fn default() -> Self {
        Boatyard::new()
    }
}

impl Menu for Boatyard {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "little_rapids_boatyard"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let progress = Progress::load(ctx);
        let mut items = Vec::new();
        for boat in Boat::ALL {
            let (label, help) = if progress.boat() == boat {
                (
                    format!("{}, sailing", boat.name()),
                    format!(
                        "{}. This is the boat you sail.",
                        hearken::text::sentence_start(boat.handling())
                    ),
                )
            } else if progress.has_boat(boat) {
                (
                    boat.name().to_string(),
                    format!(
                        "{}. Enter sails it.",
                        hearken::text::sentence_start(boat.handling())
                    ),
                )
            } else {
                (
                    format!("{}, {} paper scraps", boat.name(), boat.scraps()),
                    format!(
                        "{}. Enter folds it when you have the scraps. You have {}.",
                        hearken::text::sentence_start(boat.handling()),
                        progress.scraps
                    ),
                )
            };
            items.push(
                MenuItem::new(label, move |s: &mut Boatyard, ctx: &mut Ctx<Parlor>| {
                    let mut p = Progress::load(ctx);
                    let folded = !p.has_boat(boat) && p.scraps >= boat.scraps();
                    if p.has_boat(boat) {
                        p.boat = boat.id().to_string();
                        p.save(ctx);
                        ctx.say(format!("Sailing the {}.", boat.name().to_lowercase()));
                    } else if p.scraps >= boat.scraps() {
                        p.scraps -= boat.scraps();
                        p.boats.push(boat.id().to_string());
                        p.boat = boat.id().to_string();
                        p.save(ctx);
                        ctx.audio.play(sound::FOLD, Bus::UI);
                        ctx.say(format!(
                            "{} folded. You sail it now. {} scraps left.",
                            boat.name(),
                            p.scraps
                        ));
                    } else {
                        ctx.say(format!(
                            "{} more paper scraps to fold it. Scraps float down every stream, and ribbons pay them too.",
                            boat.scraps() - p.scraps
                        ));
                    }
                    let _ = folded;
                    Menu::refresh(s, ctx, true);
                })
                .help(help),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Boatyard, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Little Rapids title."),
        );
        items
    }
}

// --- the sound check -------------------------------------------------------

/// How long a floating preview takes to reach you.
pub const DEMO_SECS: f64 = 3.0;

enum Demo {
    None,
    Float { kind: Kind, sound: Sound, left: f64 },
    Banks { step: u32, wait: f64 },
}

/// Every sound a player has to learn, by name.
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
        if let Demo::Float { sound, .. } = self.demo {
            ctx.audio.stop(sound, 80);
        }
        self.demo = Demo::None;
    }

    fn float(&mut self, ctx: &mut Ctx<Parlor>, kind: Kind) {
        self.stop(ctx);
        let s = ctx.audio.start_loop(
            sound::floating(kind),
            Some(sound::thing_pos(0.0, 6.0)),
            Bus::EFFECTS,
        );
        self.demo = Demo::Float {
            kind,
            sound: s,
            left: DEMO_SECS,
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

/// What a kind sounds like as it reaches you.
fn arrival(kind: Kind) -> Option<&'static str> {
    match kind {
        Kind::Scrap => Some(sound::SCRAP),
        Kind::Trinket | Kind::Lantern => Some(sound::TRINKET),
        Kind::Stone => Some(sound::BUMP_STONE),
        Kind::Gate => Some(sound::GATE),
        Kind::Falls => Some(sound::FALLS),
        Kind::Mill => Some(sound::BUMP_MILL),
        Kind::Duck => Some(sound::BUMP_DUCK),
        _ => None,
    }
}

fn float_row(label: &str, kind: Kind, help: &str) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(label, move |s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        s.float(ctx, kind)
    })
    .select_sound(None)
    .help(help)
}

fn plain(label: &str, key: &'static str, help: &str) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(label, move |s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        s.stop(ctx);
        ctx.audio.play(key, Bus::EFFECTS);
    })
    .select_sound(None)
    .help(help)
}

impl Menu for SoundCheck {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "little_rapids_sounds"
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
        hearken::menu::menu_enter(self, ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop(ctx);
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        vec![
            float_row("A petal", Kind::Petal, "A soft bell that rises in pitch as it floats up to you. Catch it."),
            float_row("A leaf", Kind::Leaf, "A low wooden note. Catch it."),
            float_row("A paper scrap", Kind::Scrap, "A papery flutter. Catch it: scraps fold new boats."),
            float_row("A trinket", Kind::Trinket, "A bright sparkle. Rare, and it goes on your shelf."),
            float_row("A lantern", Kind::Lantern, "A glassy chime, on lantern float days. Worth five petals."),
            float_row("A stone", Kind::Stone, "A low gurgle. Steer clear."),
            float_row("A gate", Kind::Gate, "Chiming water between two stones. Go through the middle for points."),
            float_row("A waterfall", Kind::Falls, "A roar of water through a gap. Steer into the loudest part."),
            float_row("A mill wheel", Kind::Mill, "A creaking thump on one side. Be on the other side."),
            float_row("A duck", Kind::Duck, "Quacking and paddling across. Steer clear."),
            MenuItem::new("The banks", |s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
                s.stop(ctx);
                s.demo = Demo::Banks { step: 0, wait: 0.0 };
            })
            .select_sound(None)
            .help("Lapping water at the left bank, then the right. It gets louder as you get close."),
            plain("The current", sound::CURRENT_IN, "A rising swish as you slip into the current. Its hiss is ahead of you while you ride it."),
            plain("A paddle stroke", sound::PADDLE[0], "A dip and pull through the water, on the side you paddle toward."),
            plain("A hard stroke", sound::PADDLE_HARD, "A deeper splash and a long pull: Left Shift or Right Shift, for a quick dodge."),
            plain("A clean catch", sound::CLEAN, "A bright ring on top of the note: you caught it dead centre, for double points."),
            plain("Bumping the bank", sound::BUMP_BANK, "A wooden knock and a splash. A bump resets your combo, or halves it in the leaf boat."),
            plain("Combo goes up", sound::MULTIPLIER, "A rising chime: your multiplier goes up by one."),
            MenuItem::new(labels::BACK, |_s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| ctx.pop())
                .help("Return to the Little Rapids title."),
        ]
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        match &mut self.demo {
            Demo::None => {}
            Demo::Float {
                kind,
                sound: s,
                left,
            } => {
                *left -= dt;
                let beats = (*left / DEMO_SECS * 6.0).max(0.0);
                ctx.audio
                    .set_position(*s, Some(sound::thing_pos(0.0, beats)));
                if kind.is_catch() {
                    ctx.audio.set_pitch(*s, sound::float_pitch(beats, 6.0));
                }
                ctx.audio.set_volume(*s, sound::float_volume(beats, 6.0));
                if *left <= 0.0 {
                    let (kind, s) = (*kind, *s);
                    ctx.audio.stop(s, 60);
                    if let Some(key) = arrival(kind) {
                        ctx.audio
                            .play_at(key, sound::thing_pos(0.0, 0.0), Bus::EFFECTS);
                    } else {
                        ctx.audio.play("little-rapids/song/note/d_2", Bus::EFFECTS);
                    }
                    self.demo = Demo::None;
                }
            }
            Demo::Banks { step, wait } => {
                *wait -= dt;
                if *wait <= 0.0 {
                    if *step >= 2 {
                        self.demo = Demo::None;
                        return;
                    }
                    let side = if *step == 0 { -1.0 } else { 1.0 };
                    *step += 1;
                    *wait = 1.6;
                    ctx.audio.play_at(
                        sound::BANK,
                        Vec3::new((side * HALF) as f32, 0.0, 0.0),
                        Bus::EFFECTS,
                    );
                    ctx.say(if side < 0.0 {
                        "Left bank."
                    } else {
                        "Right bank."
                    });
                }
            }
        }
    }
}
