//! The sound check, every cue in the game by name, and the practice room,
//! a launch range where each firework is named as it goes up.

use hearken::menu::{Label, Menu, MenuCore, MenuItem};
use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, State};
use parlor_core::menus::labels;
use parlor_core::{Parlor, ParlorAction};

use crate::controls::{
    aim_for, aim_line, held_pos, kind_for, wheel_line, wheel_step, Pick, Wheel, KEYS_LINE,
    RANGE_PICKS,
};
use crate::rules::{Kind, Launch, Pos};
use crate::show::pause_to_title;
use crate::sound;

/// The sound check: every sound of the game by name.
pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
    /// Where the last firework played, for the row's read-back.
    last_pos: Pos,
}

impl SoundCheck {
    pub fn new() -> Self {
        SoundCheck {
            menu: MenuCore::new(labels::SOUND_CHECK).with_intro_help(
                "Each row plays a sound of the game when you press Enter. On a firework row, Left and Right play it on that side. Escape goes back.",
            ),
            last_pos: Pos::Center,
        }
    }
}

impl Default for SoundCheck {
    fn default() -> Self {
        Self::new()
    }
}

fn firework_row(kind: Kind) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(
        format!("{}, key {}", sentence_start(kind.name()), kind.digit()),
        move |s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
            ctx.stop_speech();
            s.last_pos = Pos::Center;
            sound::launch(ctx, Launch::new(kind, Pos::Center, 0));
        },
    )
    .select_sound(None)
    .silent_adjust()
    .adjustable(move |s: &mut SoundCheck, ctx: &mut Ctx<Parlor>, d: i32| {
        let pos = if d < 0 { Pos::Left } else { Pos::Right };
        ctx.stop_speech();
        s.last_pos = pos;
        sound::launch(ctx, Launch::new(kind, pos, 0));
    })
    .adjusted_text(Label::dynamic(move |s: &SoundCheck, _ctx: &Ctx<Parlor>| {
        format!("{} {}", sentence_start(kind.name()), s.last_pos.phrase())
    }))
    .help("Enter plays it in the center. Left and Right play it on that side.")
}

fn cue_row(label: &str, key: &'static str, bus: Bus, help: &str) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(label, move |_s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        ctx.stop_speech();
        ctx.audio.play(key, bus);
    })
    .select_sound(None)
    .help(help)
}

fn crowd_row(label: &str, accuracy: u32) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(label, move |_s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        ctx.stop_speech();
        sound::react(ctx, accuracy);
    })
    .select_sound(None)
    .help("The crowd reacts like this when a show ends; the better the show, the louder.")
}

impl Menu for SoundCheck {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "sound_check"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_bed(None);
        ctx.audio.play_music(None, false);
        hearken::menu::menu_enter(self, ctx);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items: Vec<MenuItem<Self, Parlor>> = Vec::new();
        let left = ctx.control_name(ParlorAction::Left);
        let right = ctx.control_name(ParlorAction::Right);
        items.extend(Kind::ALL.into_iter().map(firework_row));
        items.push(
            MenuItem::new(
                "Aim tick",
                |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.stop_speech();
                    sound::aim(ctx, Pos::Center);
                },
            )
            .select_sound(None)
            .silent_adjust()
            .adjustable(|s: &mut Self, ctx: &mut Ctx<Parlor>, d: i32| {
                let pos = if d < 0 { Pos::Left } else { Pos::Right };
                ctx.stop_speech();
                s.last_pos = pos;
                sound::aim(ctx, pos);
            })
            .adjusted_text(Label::dynamic(|s: &Self, _ctx: &Ctx<Parlor>| {
                format!("Aim {}", s.last_pos.phrase())
            }))
            .help(format!(
                "Here Left and Right play it on that side. During a show, pressing {left} or A, or {right} or D, ticks on that side, so you know where your next launch will go."
            )),
        );
        items.push(cue_row(
            "Launch thump",
            sound::LAUNCH,
            Bus::EFFECTS,
            "Plays under every firework you launch.",
        ));
        items.push(cue_row(
            "Count-in tick: one, two, three",
            sound::TICK,
            Bus::EFFECTS,
            "Three of these, half a second apart, start every show.",
        ));
        items.push(cue_row(
            "Go: time zero",
            sound::GO,
            Bus::EFFECTS,
            "Comes after the third tick. Show timings are measured from it.",
        ));
        items.push(cue_row(
            "Fuse: your turn",
            sound::FUSE,
            Bus::EFFECTS,
            "Fizzes when the show is over and it is your turn. Then Enter, a key of this game's own, finishes your show.",
        ));
        items.push(crowd_row("Crowd cheer: 80 percent or better", 100));
        items.push(crowd_row("Crowd ooh: 60 to 79 percent, a pass", 70));
        items.push(crowd_row("Crowd aah: 40 to 59 percent, not quite", 50));
        items.push(crowd_row("Crowd murmur: under 40 percent, confused", 20));
        items.push(cue_row(
            "Combo chime: two or more near-perfect fireworks in a row",
            sound::COMBO,
            Bus::UI,
            "Plays with the results when your show had a combo.",
        ));
        items.push(cue_row(
            "Undo: a firework taken back",
            sound::UNDO,
            Bus::UI,
            "Backspace, a key of this game's own, takes back your last firework with this sound.",
        ));
        items.push(cue_row(
            "Choice tick: the main action's firework changed",
            sound::SELECT,
            Bus::UI,
            "Plays as Up and Down choose what the main action launches, with the choice spoken.",
        ));
        items.push(
            MenuItem::new(
                "Night fog boom",
                |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.stop_speech();
                    sound::fogged(ctx, Launch::new(Kind::Boom, Pos::Center, 0))
                },
            )
            .select_sound(None)
            .silent_adjust()
            .adjustable(|s: &mut Self, ctx: &mut Ctx<Parlor>, d: i32| {
                let pos = if d < 0 { Pos::Left } else { Pos::Right };
                ctx.stop_speech();
                s.last_pos = pos;
                sound::fogged(ctx, Launch::new(Kind::Boom, pos, 0));
            })
            .adjusted_text(Label::dynamic(|s: &Self, _ctx: &Ctx<Parlor>| {
                format!("Fog boom {}", s.last_pos.phrase())
            }))
            .help("Under a challenge's night fog twist, the show's fireworks sound like this: far off and quiet, still on their side. Left and Right play it on that side."),
        );
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Liberty Lights title."),
        );
        items
    }
}

/// Fire freely; each firework is named once it is up.
pub struct Range {
    launched: u32,
    wheel: Wheel,
}

impl Range {
    pub fn new() -> Self {
        Range {
            launched: 0,
            wheel: Wheel::new(&RANGE_PICKS),
        }
    }

    fn prompt(&self, ctx: &Ctx<Parlor>) -> String {
        format!(
            "Practice room, a launch range with nothing to score. {KEYS_LINE} {} {} {} launches it. Each firework is named after it goes up. Escape pauses.",
            aim_line(ctx),
            wheel_line(ctx, false),
            sentence_start(&ctx.control_name(ParlorAction::Primary))
        )
    }

    fn fire(&mut self, ctx: &mut Ctx<Parlor>, kind: Kind) {
        let pos = held_pos(ctx);
        sound::launch(ctx, Launch::new(kind, pos, 0));
        self.launched += 1;
        ctx.say(format!("{} {}.", sentence_start(kind.name()), pos.phrase()));
    }
}

impl Default for Range {
    fn default() -> Self {
        Self::new()
    }
}

impl State<Parlor> for Range {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        "range"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::night(ctx);
        let p = self.prompt(ctx);
        ctx.say(p);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.say("Practice room.");
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() || (event.pressed().is_none() && ctx.action_for(event).is_none()) {
            return;
        }
        if event.pressed() == Some(Key::Escape) {
            pause_to_title(ctx);
            return;
        }
        if let Some(kind) = kind_for(event) {
            self.wheel.select(kind);
            self.fire(ctx, kind);
            return;
        }
        if ctx.action_for(event) == Some(ParlorAction::Primary) {
            if let Pick::Fire(kind) = self.wheel.current() {
                self.fire(ctx, kind);
            }
            return;
        }
        if let Some(step) = wheel_step(ctx, event) {
            let pick = self.wheel.step(step);
            ctx.audio.play(sound::SELECT, Bus::UI);
            ctx.say(pick.spoken());
            return;
        }
        if let Some(pos) = aim_for(ctx, event) {
            sound::aim(ctx, pos);
            ctx.say(format!("Aiming {}.", pos.name()));
            return;
        }
        match ctx.action_for(event) {
            Some(ParlorAction::Status) => ctx.say(if self.launched == 1 {
                "1 firework launched here.".to_string()
            } else {
                format!("{} fireworks launched here.", self.launched)
            }),
            _ => {
                let p = self.prompt(ctx);
                ctx.say(p);
            }
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        vec![format!("Practice room: {} launched", self.launched)]
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        self.prompt(ctx)
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

mod visuals;
