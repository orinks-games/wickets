//! The challenge list, the sound check and the home aquarium.

use hearken::menu::{menu_enter, Menu, MenuCore, MenuItem};
use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, Vec3};
use parlor_core::menus::{self, labels, ChallengeGame};
use parlor_core::{Challenge, Parlor, ParlorAction, Period, Rng};

use crate::outing::{challenge_briefing, challenge_name, challenge_outing, challenge_tag};
use crate::rules::{list_words, Lean, Place, Species};
use crate::save::Aquarium;
use crate::sound;

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "tide_pools_challenges";

/// The game's side of the shared challenges menu.
pub struct Shores;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Shores>;

impl ChallengeGame for Shores {
    const ID: &'static str = crate::ID;
    const NAME: &'static str = "Tide Pools";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        challenge_name(period).to_string()
    }

    fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        let challenge = Challenge::current(ctx, crate::ID, period);
        challenge_tag(period, &challenge_outing(&challenge).plan)
    }

    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        let challenge = Challenge::current(ctx, crate::ID, period);
        let plan = challenge_outing(&challenge).plan;
        format!("{} {}", plan.modifier.describe(), plan.summary())
            .trim_start()
            .to_string()
    }

    fn open(ctx: &mut Ctx<Parlor>, period: Period) {
        let pages = challenge_briefing(ctx, period);
        ctx.push(pages);
    }

    fn intro_help() -> String {
        "Up and Down choose a challenge, Enter reads its goal, Escape goes back. Every player gets the same shore: a new daily each day, a weekly each Monday and a monthly event each month.".to_string()
    }
}

/// Stands in for the challenges menu when the launcher opens a challenge:
/// it opens the briefing, and once the trip is over, or left from the
/// pause menu, it steps aside, back to the launcher.
pub struct ChallengeHost {
    period: Period,
    opened: bool,
}

impl ChallengeHost {
    pub fn new(period: Period) -> ChallengeHost {
        ChallengeHost {
            period,
            opened: false,
        }
    }
}

impl State<Parlor> for ChallengeHost {
    /// Named as the challenges menu is, so leaving a trip lands here.
    fn name(&self) -> &'static str {
        CHALLENGES_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.opened {
            self.opened = true;
            let pages = challenge_briefing(ctx, self.period);
            ctx.push(pages);
        }
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.pop();
    }

    fn resume_quiet(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.pop();
    }

    fn resume_with(&mut self, ctx: &mut Ctx<Parlor>, _result: &str) {
        ctx.pop();
    }

    fn help(&self, _ctx: &Ctx<Parlor>) -> String {
        "Opening the Tide Pools challenge.".to_string()
    }
}

/// Every sound with its name and meaning: Enter plays it.
pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
}

impl SoundCheck {
    pub fn new() -> SoundCheck {
        SoundCheck {
            menu: MenuCore::new(labels::SOUND_CHECK).with_intro_help(
                "Up and Down move through the game's sounds, Enter plays one, Escape goes back.",
            ),
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
        "tide_pools_sounds"
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

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items: Vec<MenuItem<Self, Parlor>> = sound::catalog()
            .into_iter()
            .map(|(key, name, meaning)| {
                let help = meaning.clone();
                MenuItem::new(name, move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    if key.contains("/music/") {
                        ctx.audio.play_music(Some(key), false);
                    } else {
                        ctx.audio.play(key, Bus::EFFECTS);
                    }
                    ctx.stop_speech();
                })
                .select_sound(None)
                .help(help.clone())
            })
            .collect();
        items.push(
            MenuItem::new("Left and right", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                sound::play_at(
                    ctx,
                    sound::creature(Species::Crab),
                    sound::pool_at(-1, Lean::Edge),
                );
                sound::play_at(
                    ctx,
                    sound::creature(Species::Periwinkle),
                    sound::pool_at(1, Lean::Edge),
                );
                ctx.stop_speech();
            })
            .select_sound(None)
            .help("Two creatures, one in the pool on each side of you."),
        );
        items.push(
            MenuItem::new("Near and far", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                sound::play_at(
                    ctx,
                    sound::creature(Species::Anemone),
                    sound::pool_at(0, Lean::Back),
                );
                ctx.stop_speech();
            })
            .select_sound(None)
            .help("How a pool sounds from standing back, compared with leaning in."),
        );
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.play_ui(hearken::UiSound::Back);
                ctx.pop()
            })
            .help("Return to the Tide Pools title."),
        );
        items
    }
}

/// Where a species lives in the aquarium, around the listener.
pub fn tank_spot(s: Species) -> Vec3 {
    let i = Species::ALL.iter().position(|x| *x == s).unwrap_or(0) as f32;
    Vec3::new((i - 6.5) * 0.6, 2.0, 0.0)
}

/// The places a common species lives.
fn homes(s: Species) -> Vec<&'static str> {
    Place::ALL
        .into_iter()
        .filter(|p| p.commons().contains(&s))
        .map(|p| p.name())
        .collect()
}

/// The home aquarium, as a soundscape you walk along.
pub struct AquariumScreen {
    tank: Aquarium,
    index: usize,
    pump: Option<Sound>,
    call_in: f64,
    rng: Rng,
    tour: Vec<(f64, Species)>,
    last: String,
}

impl AquariumScreen {
    pub fn new() -> AquariumScreen {
        AquariumScreen {
            tank: Aquarium::default(),
            index: 0,
            pump: None,
            call_in: 1.5,
            rng: Rng::seeded(0xA0A0),
            tour: Vec::new(),
            last: String::new(),
        }
    }

    fn owned(&self) -> Vec<Species> {
        self.tank.species()
    }

    /// The tank's contents in a sentence.
    pub fn summary(&self) -> String {
        let kinds = self.tank.kinds();
        if kinds == 0 {
            return "Your aquarium is empty. Gather creatures at the beach and they come home with you.".to_string();
        }
        let missing = self.tank.missing();
        let rare_missing = missing.iter().filter(|s| s.rare()).count();
        let common_missing = missing.len() - rare_missing;
        let still = if missing.is_empty() {
            " Every kind is here.".to_string()
        } else {
            format!(
                " Still to find: {common_missing} common {} and {rare_missing} rare.",
                if common_missing == 1 { "kind" } else { "kinds" }
            )
        };
        format!(
            "Your aquarium holds {kinds} of {} kinds, {} creatures in all, from {} {}.{still}",
            Species::ALL.len(),
            self.tank.creatures(),
            self.tank.outings,
            if self.tank.outings == 1 {
                "trip"
            } else {
                "trips"
            }
        )
    }

    /// What H says: one kind still missing, and where to look.
    pub fn hint(&self) -> String {
        let missing = self.tank.missing();
        let Some(s) = missing.first() else {
            return "Your aquarium has every kind. Every trip still adds more to the tank."
                .to_string();
        };
        match s.when() {
            Some(when) => format!(
                "You have not found a {} yet. It only turns up {when}. Listen for {}.",
                s.name(),
                s.sound_words()
            ),
            None => format!(
                "You have not found a {} yet. It lives at {}. Listen for {}.",
                s.name(),
                list_words(&homes(*s)),
                s.sound_words()
            ),
        }
    }

    fn describe(&self, s: Species, at: usize, of: usize) -> String {
        let first = self.tank.first_note(s).unwrap_or_default();
        format!(
            "{}, {} in the tank. {}. {first} {} of {of}.",
            sentence_start(s.name()),
            self.tank.count(s),
            sentence_start(s.sound_words()),
            at + 1
        )
    }

    fn show(&mut self, ctx: &mut Ctx<Parlor>) {
        let owned = self.owned();
        let Some(&s) = owned.get(self.index) else {
            let text = self.summary();
            self.last = text.clone();
            ctx.say(text);
            return;
        };
        ctx.audio.play(sound::TANK, Bus::UI);
        ctx.audio
            .play_at(sound::creature(s), tank_spot(s), Bus::EFFECTS);
        let text = self.describe(s, self.index, owned.len());
        self.last = text.clone();
        ctx.say(text);
    }

    fn controls(ctx: &Ctx<Parlor>) -> String {
        format!(
            "{} and {} move along the tanks, {} plays one again, {} plays the whole tank, {} sums it up, {} says what is still missing, Escape goes back.",
            sentence_start(&ctx.control_name(ParlorAction::Left)),
            ctx.control_name(ParlorAction::Right),
            ctx.control_name(ParlorAction::Primary),
            ctx.control_name(ParlorAction::Secondary),
            ctx.control_name(ParlorAction::Status),
            ctx.control_name(ParlorAction::Hint),
        )
    }
}

impl Default for AquariumScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl State<Parlor> for AquariumScreen {
    fn name(&self) -> &'static str {
        "tide_pools_aquarium"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        self.tank = Aquarium::load(ctx);
        ctx.audio.play_music(None, false);
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        let pump = ctx.audio.start_loop(sound::AQUARIUM, None, Bus::AMBIENCE);
        ctx.audio.set_volume(pump, 0.6);
        self.pump = Some(pump);
        let text = format!("Your aquarium. {}", self.summary());
        self.last = text.clone();
        ctx.say(text);
        if self.tank.kinds() > 0 {
            let controls = Self::controls(ctx);
            ctx.say_queued(controls);
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Some(p) = self.pump.take() {
            ctx.audio.stop(p, 500);
        }
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        let text = self.last.clone();
        ctx.say(text);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() {
            return;
        }
        if event.pressed() == Some(Key::Escape) {
            ctx.play_ui(hearken::UiSound::Back);
            ctx.pop();
            return;
        }
        let n = self.owned().len();
        match ctx.action_for(event) {
            Some(ParlorAction::Left | ParlorAction::Up) => {
                if n > 0 {
                    self.index = (self.index + n - 1) % n;
                }
                self.show(ctx);
            }
            Some(ParlorAction::Right | ParlorAction::Down) => {
                if n > 0 {
                    self.index = (self.index + 1) % n;
                }
                self.show(ctx);
            }
            Some(ParlorAction::Primary) => self.show(ctx),
            Some(ParlorAction::Secondary) => {
                let owned = self.owned();
                if owned.is_empty() {
                    ctx.say("The tank is empty and still: only the pump bubbles.");
                } else {
                    self.tour = owned
                        .iter()
                        .enumerate()
                        .map(|(i, s)| (i as f64 * 1.3, *s))
                        .collect();
                    let names: Vec<&str> = owned.iter().map(|s| s.name()).collect();
                    ctx.say(format!(
                        "The whole tank, from left to right: {}.",
                        list_words(&names)
                    ));
                }
            }
            Some(ParlorAction::Status) => {
                let text = self.summary();
                ctx.say(text);
            }
            Some(ParlorAction::Hint) => {
                let text = self.hint();
                ctx.say(text);
            }
            Some(ParlorAction::Repeat) => {
                let text = self.last.clone();
                ctx.say(text);
            }
            None => {}
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if !self.tour.is_empty() {
            let mut due = Vec::new();
            self.tour.retain_mut(|(t, s)| {
                *t -= dt;
                if *t <= 0.0 {
                    due.push(*s);
                    false
                } else {
                    true
                }
            });
            for s in due {
                ctx.audio
                    .play_at(sound::creature(s), tank_spot(s), Bus::EFFECTS);
            }
            return;
        }
        let owned = self.owned();
        if owned.is_empty() {
            return;
        }
        self.call_in -= dt;
        if self.call_in <= 0.0 {
            let creatures = self.tank.creatures() as f64;
            self.call_in = (7.0 - creatures * 0.25).max(1.5) + self.rng.unit() * 2.5;
            let total: u32 = owned.iter().map(|s| self.tank.count(*s)).sum();
            let mut pick = self.rng.below(total.max(1));
            let mut chosen = owned[0];
            for s in &owned {
                let c = self.tank.count(*s);
                if pick < c {
                    chosen = *s;
                    break;
                }
                pick -= c;
            }
            let h = ctx
                .audio
                .play_at(sound::creature(chosen), tank_spot(chosen), Bus::AMBIENCE);
            ctx.audio.set_volume(h, 0.6);
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let mut out = vec!["Your aquarium".to_string()];
        for (i, s) in self.owned().iter().enumerate() {
            let marker = if i == self.index { "> " } else { "  " };
            out.push(format!("{marker}{}: {}", s.name(), self.tank.count(*s)));
        }
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        if self.tank.kinds() == 0 {
            return "Your aquarium is empty. Gather creatures at the beach and they come home with you. Escape goes back.".to_string();
        }
        format!(
            "Your aquarium: everything you have gathered lives here, each kind in its own spot from left to right, and they call now and then. {}",
            Self::controls(ctx)
        )
    }
}
