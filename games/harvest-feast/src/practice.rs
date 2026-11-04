//! The sound check: every cue of the kitchen, by name.

use hearken::menu::{menu_enter, Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Sound};
use parlor_core::Parlor;

use crate::rules::{Action, Station};
use crate::sound;

/// How long a loop plays when previewed.
const PREVIEW_SECONDS: f64 = 4.0;

pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
    preview: Sound,
    left: f64,
}

impl SoundCheck {
    pub fn new() -> Self {
        SoundCheck {
            menu: MenuCore::new("Sound check").with_intro_help(
                "Each row plays one sound of the kitchen when you press Enter, from where you would hear it in the game. Cooking sounds play for a few seconds. In the kitchen, the number keys 1 to 5 jump to these stations from left to right: they belong to this game alone. Escape goes back.",
            ),
            preview: Sound::NONE,
            left: 0.0,
        }
    }

    fn stop(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.left > 0.0 {
            ctx.audio.stop(self.preview, 150);
        }
        self.preview = Sound::NONE;
        self.left = 0.0;
    }

    fn preview(&mut self, ctx: &mut Ctx<Parlor>, key: &str, station: Station) {
        self.stop(ctx);
        self.preview = ctx
            .audio
            .start_loop(key, Some(sound::place(station)), Bus::EFFECTS);
        self.left = PREVIEW_SECONDS;
    }
}

impl Default for SoundCheck {
    fn default() -> Self {
        Self::new()
    }
}

fn looped(
    label: String,
    key: &'static str,
    station: Station,
    help: String,
) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(label, move |s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        s.preview(ctx, key, station)
    })
    .select_sound(None)
    .help(help)
}

fn placed(
    label: &str,
    key: &'static str,
    station: Station,
    help: &str,
) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(label, move |s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        s.stop(ctx);
        sound::at(ctx, key, station);
    })
    .select_sound(None)
    .help(help)
}

fn plain(label: &str, key: &'static str, bus: Bus, help: &str) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(label, move |s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        s.stop(ctx);
        ctx.audio.play(key, bus);
    })
    .select_sound(None)
    .help(help)
}

fn station_rows(station: Station) -> Vec<MenuItem<SoundCheck, Parlor>> {
    let mut rows = Vec::new();
    let title = station.title();
    if let Some(key) = sound::cooking(station) {
        rows.push(looped(
            format!("{title}, cooking: {}", station.cooking_sound()),
            key,
            station,
            format!("All is well {}: nothing to do yet.", station.on()),
        ));
    }
    rows.push(looped(
        format!("{title}, needs you: {}", station.due_sound()),
        sound::due(station),
        station,
        "The dish needs you now: act at once for a perfect score.".to_string(),
    ));
    rows.push(looped(
        format!("{title}, burning: {}", station.burning_sound()),
        sound::burning(station),
        station,
        "Overdue: act fast to save it, or it is ruined.".to_string(),
    ));
    rows
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
        sound::kitchen(ctx);
        menu_enter(self, ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop(ctx);
    }

    /// F1: the row's help, then how the sound check works.
    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let intro = self.menu().intro_help.clone();
        match self.current_help(ctx) {
            Some(row) => format!("{} {intro}", hearken::text::end_sentence(&row)),
            None => intro,
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.left > 0.0 {
            self.left -= dt;
            if self.left <= 0.0 {
                ctx.audio.stop(self.preview, 300);
                self.preview = Sound::NONE;
                self.left = 0.0;
            }
        }
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items = Vec::new();
        for station in Station::ALL {
            items.extend(station_rows(station));
        }
        items.push(looped(
            "Recipe calling a station: a soft double knock".to_string(),
            sound::WAITING,
            Station::Oven,
            "Knocks from a station the recipe has just called for: go there and start the dish."
                .to_string(),
        ));
        items.push(looped(
            "Recipe called a while ago: a fast triple knock".to_string(),
            sound::WAITING_LATE,
            Station::Oven,
            "The station has waited too long: start it now, or someone will.".to_string(),
        ));
        let actions: [(&str, Station, Action); 14] = [
            (
                "Start a pot on a burner",
                Station::LeftBurner,
                Action::Start,
            ),
            (
                "Start a dish on the cutting board",
                Station::Board,
                Action::Start,
            ),
            ("Put a dish in the oven", Station::Oven, Action::Start),
            ("Pour onto the griddle", Station::Griddle, Action::Start),
            ("Stir", Station::LeftBurner, Action::Stir),
            ("Whisk", Station::LeftBurner, Action::Whisk),
            ("Take off the heat", Station::LeftBurner, Action::TakeOff),
            ("Drain", Station::RightBurner, Action::Drain),
            ("Baste", Station::Oven, Action::Baste),
            ("Turn the tray", Station::Oven, Action::Rotate),
            ("Pull out of the oven", Station::Oven, Action::Pull),
            ("Flip", Station::Griddle, Action::Flip),
            ("Plate from the griddle", Station::Griddle, Action::Plate),
            ("Chop, one stroke", Station::Board, Action::Chop),
        ];
        for (label, station, action) in actions {
            items.push(placed(
                label,
                sound::action(station, action),
                station,
                "What your main action sounds like when it does this.",
            ));
        }
        items.push(placed(
            "Dish served: a plate set down",
            sound::PLATE,
            Station::Oven,
            "Plays when a dish's last step is done.",
        ));
        items.push(plain(
            "Perfect timing: three rising bells",
            sound::grade(crate::rules::Grade::Perfect),
            Bus::UI,
            "You acted as soon as the dish needed you.",
        ));
        items.push(plain(
            "Good timing: two rising notes",
            sound::grade(crate::rules::Grade::Good),
            Bus::UI,
            "On time, but not right away.",
        ));
        items.push(plain(
            "Late: two falling notes",
            sound::grade(crate::rules::Grade::Late),
            Bus::UI,
            "Saved while it was burning, for few points.",
        ));
        items.push(placed(
            "Not yet: a low double knock",
            sound::EARLY,
            Station::Oven,
            "You acted before the dish was ready: a fumble, minus 5 for that dish.",
        ));
        items.push(placed(
            "Ruined: the smoke alarm",
            sound::RUINED,
            Station::Oven,
            "A dish waited too long; someone rescues it, for no points.",
        ));
        items.push(placed(
            "Moving to a station: a tick",
            sound::FOCUS,
            Station::Griddle,
            "Plays at the station you move to.",
        ));
        items.push(placed(
            "End of the row: a bump",
            sound::EDGE,
            Station::RightBurner,
            "There is no station further that way.",
        ));
        items.push(placed(
            "Nothing to do: a hollow tap",
            sound::EMPTY,
            Station::Board,
            "The station in front of you has nothing on it.",
        ));
        items.push(plain(
            "Recipe call: a page and two bells",
            sound::RECIPE,
            Bus::UI,
            "Plays as the recipe calls the next dish.",
        ));
        items.push(plain(
            "Dinner bell: every dish served",
            sound::BELL,
            Bus::EFFECTS,
            "The meal is over; the results follow.",
        ));
        items.push(plain(
            "Golden plate: a bright bell and a coin",
            sound::GOLDEN,
            Bus::UI,
            "In the harvest banquet, a dish served at quality 90 or better: 200 more points.",
        ));
        items.push(
            MenuItem::new("Back", |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.pop())
                .help("Return to the Harvest Feast title."),
        );
        items
    }
}
