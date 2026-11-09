//! Freestyle: compose a show of your own, hear it back, save it, and take
//! it on as a challenge.

use hearken::menu::{Label, Menu, MenuCore, MenuItem};
use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, State};
use parlor_core::menus::labels;
use parlor_core::{Parlor, ParlorAction, Records};

use crate::controls::{
    aim_for, aim_line, held_pos, is_enter, kind_for, wheel_line, wheel_step, Pick, Wheel,
    KEYS_LINE, SHOW_PICKS,
};
use crate::progress::{self, Progress};
use crate::replay::Replay;
use crate::rules::{self, Composer, Kind, Launch, FREESTYLE_CAP};
use crate::show::{pause_to_title, Mode, Show};
use crate::sound;
use crate::ID;

/// The freestyle corner of the title.
pub struct FreestyleMenu {
    menu: MenuCore<Self, Parlor>,
}

impl FreestyleMenu {
    pub fn new() -> Self {
        FreestyleMenu {
            menu: MenuCore::new("Freestyle show"),
        }
    }
}

impl Default for FreestyleMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for FreestyleMenu {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "liberty_lights_freestyle_menu"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let saved = Progress::load(ctx).saved_show;
        let mut items = vec![MenuItem::new(
            "Compose a new show",
            |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.push(Freestyle::new()),
        )
        .help("Launch your own fireworks without scoring. The clock starts at your first launch.")];
        if saved.is_empty() {
            items.push(
                MenuItem::inert("No saved show yet")
                    .help("Compose a show, then choose Save this show to keep it here."),
            );
        } else {
            let n = saved.len();
            let len = rules::seconds(saved.last().map_or(0, |l| l.ms));
            let best = Records::best(ctx, ID, progress::MY_SHOW_BOARD);
            let s1 = saved.clone();
            items.push(
                MenuItem::new(
                    format!("Hear your saved show, {n} fireworks, {len}"),
                    move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        ctx.push(Replay::show("Your saved show.", s1.clone()))
                    },
                )
                .help("Plays the show you saved, with the crowd watching."),
            );
            let s2 = saved.clone();
            let best = match best {
                Some(b) => format!(" Best {b}."),
                None => String::new(),
            };
            items.push(
                MenuItem::new(
                    "Match your saved show",
                    move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        ctx.push(Show::with_show(Mode::MyShow, s2.clone()))
                    },
                )
                .help(format!(
                    "Your own show as a puzzle: hear it, then launch it back, scored like a level.{best}"
                )),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Liberty Lights title."),
        );
        items
    }
}

/// Composing a show.
pub struct Freestyle {
    pub composer: Composer,
    /// What the main action does; Up and Down change it.
    pub wheel: Wheel,
}

impl Freestyle {
    pub fn new() -> Self {
        Freestyle {
            composer: Composer::new(),
            wheel: Wheel::new(&SHOW_PICKS),
        }
    }

    fn prompt(&self, ctx: &Ctx<Parlor>) -> String {
        format!(
            "Freestyle show. {KEYS_LINE} {} Press Enter when your show is finished. Backspace takes back the last firework. {} {} now: {}",
            aim_line(ctx),
            wheel_line(ctx, true),
            sentence_start(&ctx.control_name(ParlorAction::Primary)),
            self.wheel.current().spoken()
        )
    }

    fn fire(&mut self, ctx: &mut Ctx<Parlor>, kind: Kind) {
        let pos = held_pos(ctx);
        match self.composer.launch(kind, pos) {
            Some(l) => sound::launch(ctx, l),
            None => ctx.say(format!(
                "Your show is full at {FREESTYLE_CAP} fireworks. Press Enter to finish it."
            )),
        }
    }

    fn take_back(&mut self, ctx: &mut Ctx<Parlor>) {
        match self.composer.undo() {
            Some(l) => {
                ctx.audio.play(sound::UNDO, Bus::UI);
                ctx.say(format!("Last firework removed: {}.", l.kind.name()));
            }
            None => ctx.say("Nothing to take back."),
        }
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        let n = self.composer.launches.len();
        sound::react(ctx, if n == 0 { 30 } else { 100 });
        let summary = format!(
            "Freestyle show complete. You launched {n} firework{}.",
            if n == 1 { "" } else { "s" }
        );
        ctx.replace(FreestyleDone::new(summary, self.composer.launches.clone()));
    }
}

impl Default for Freestyle {
    fn default() -> Self {
        Self::new()
    }
}

impl State<Parlor> for Freestyle {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        "liberty_lights_freestyle"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::night(ctx);
        ctx.say(format!(
            "Freestyle show. Compose your own show. {KEYS_LINE} {} Enter finishes.",
            aim_line(ctx)
        ));
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::night(ctx);
        ctx.say(format!(
            "Freestyle show, {} launched.",
            self.composer.launches.len()
        ));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() {
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
        // The main action before Enter: a controller's A is both.
        if ctx.action_for(event) == Some(ParlorAction::Primary) {
            match self.wheel.current() {
                Pick::Fire(kind) => self.fire(ctx, kind),
                Pick::TakeBack => self.take_back(ctx),
                Pick::Finish => self.finish(ctx),
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
            return;
        }
        if is_enter(event) {
            self.finish(ctx);
            return;
        }
        if event.pressed() == Some(Key::Backspace) {
            self.take_back(ctx);
            return;
        }
        match ctx.action_for(event) {
            Some(ParlorAction::Repeat) => {
                let p = self.prompt(ctx);
                ctx.say(p);
            }
            Some(ParlorAction::Status) => {
                let n = self.composer.launches.len();
                ctx.say(format!(
                    "{n} of {FREESTYLE_CAP} fireworks launched, {} from first to last.",
                    rules::seconds(self.composer.length_ms())
                ));
            }
            Some(ParlorAction::Hint) => ctx.say(
                "An idea: a boom on the left, a crackle on the right a beat later, then a whistle and a sparkle together in the center.",
            ),
            _ => {
                if event.pressed().is_some() {
                    ctx.say(format!(
                        "{KEYS_LINE} {} repeats the instructions.",
                        sentence_start(&ctx.control_name(ParlorAction::Repeat))
                    ));
                }
            }
        }
    }

    fn update(&mut self, _ctx: &mut Ctx<Parlor>, dt: f64) {
        self.composer.update(dt);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        vec![format!(
            "Freestyle: {} launched",
            self.composer.launches.len()
        )]
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        format!(
            "Freestyle: no score and no clock to beat. Your show's time starts at your first launch. {KEYS_LINE} {} Backspace takes back the last firework, Enter finishes the show. {} {} does what you chose. {} repeats the instructions, {} counts your fireworks, {} gives an idea. Escape pauses.",
            aim_line(ctx),
            wheel_line(ctx, true),
            sentence_start(&ctx.control_name(ParlorAction::Primary)),
            sentence_start(&ctx.control_name(ParlorAction::Repeat)),
            ctx.control_name(ParlorAction::Status),
            ctx.control_name(ParlorAction::Hint)
        )
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

/// After a freestyle show.
pub struct FreestyleDone {
    menu: MenuCore<Self, Parlor>,
    pub show: Vec<Launch>,
    saved: bool,
}

impl FreestyleDone {
    pub fn new(summary: String, show: Vec<Launch>) -> Self {
        FreestyleDone {
            menu: MenuCore::new(summary),
            show,
            saved: false,
        }
    }
}

impl Menu for FreestyleDone {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "liberty_lights_freestyle_done"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items = Vec::new();
        if !self.show.is_empty() {
            items.push(
                MenuItem::new("Hear your show", |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.push(Replay::show("Your show.", s.show.clone()))
                })
                .help("Plays back the show you just launched."),
            );
            items.push(
                MenuItem::new(
                    Label::dynamic(|s: &Self, _ctx: &Ctx<Parlor>| {
                        if s.saved {
                            "Saved".to_string()
                        } else {
                            "Save this show".to_string()
                        }
                    }),
                    |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        if s.saved {
                            ctx.say("This show is already saved.");
                            return;
                        }
                        let mut p = Progress::load(ctx);
                        p.saved_show = s.show.clone();
                        p.store(ctx);
                        // The old show's best belongs to the old show.
                        progress::forget_my_show_best(ctx);
                        s.saved = true;
                        ctx.audio.play(sound::SAVED, Bus::UI);
                        ctx.say("Show saved. Match it from the freestyle menu.");
                    },
                )
                .help("Keeps this show in place of any saved before, to hear again or match from the freestyle menu.")
                .select_sound(None),
            );
        }
        items.push(
            MenuItem::new(
                labels::PLAY_AGAIN,
                |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.replace(Freestyle::new()),
            )
            .help("Launch a new unscored fireworks show."),
        );
        items.push(
            MenuItem::new(
                labels::BACK_TO_TITLE,
                |_s: &mut Self, ctx: &mut Ctx<Parlor>| back_to_title(ctx),
            )
            .select_sound(None)
            .help("Return to the game's title."),
        );
        items
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if !event.is_repeat() && ctx.action_for(event) == Some(ParlorAction::Repeat) {
            let title = self.menu.title.clone();
            ctx.say(title);
            let current = self.current_text(ctx);
            ctx.say_queued(current);
            return;
        }
        hearken::menu::menu_input(self, ctx, event)
    }

    /// Escape does what Back to the title does.
    fn on_escape(&mut self, ctx: &mut Ctx<Parlor>) {
        back_to_title(ctx);
    }
}

fn back_to_title(ctx: &mut Ctx<Parlor>) {
    ctx.play_ui(hearken::UiSound::Back);
    ctx.pop_to(crate::TITLE_NAME);
}

mod visuals;
