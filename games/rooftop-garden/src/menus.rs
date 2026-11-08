//! The menus off the rooftop: challenges, the seed shed and the sound check.

use hearken::menu::{menu_enter, Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Sound, State, Vec3};
use parlor_core::menus::{self, labels, ChallengeGame};
use parlor_core::{Challenge, Parlor, Period};

use crate::cues::{self, Play, SOUND_CHECK};
use crate::rules::{number, sentence, Garden, Goal, Species};
use crate::breeding::{self, Keepsake, Variety};
use crate::screens::{challenge_briefing, challenge_twist, load_collection, period_text, GARDEN_KEY};
use crate::song::{Chord, Part};
use crate::ID;

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "rooftop_garden_challenges";

/// The game's side of the shared challenges menu.
pub struct Gardens;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Gardens>;

/// A goal in a few words, for the challenge row.
fn goal_words(goal: Goal) -> String {
    match goal {
        Goal::Harmony(n) => format!("grow a {}-part harmony", number(n)),
        Goal::Harvest(s, n) => format!("harvest {n} {}", s.plural()),
    }
}

impl ChallengeGame for Gardens {
    const ID: &'static str = ID;
    const NAME: &'static str = "Rooftop Garden";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        match period {
            Period::Daily => "Today's garden",
            Period::Weekly => "This week's garden",
            Period::Monthly => "The rooftop festival",
        }
        .to_string()
    }

    fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        let challenge = Challenge::current(ctx, ID, period);
        let garden = Garden::challenge(period, String::new(), challenge.rng());
        let goal = garden.goal.map(goal_words).unwrap_or_default();
        match (period, garden.twist) {
            (Period::Monthly, _) | (_, None) => goal,
            (_, Some(twist)) => format!("{}, {goal}", twist.name()),
        }
    }

    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        let twist = challenge_twist(ctx, period);
        format!("{} {}", period_text(period), twist.describe())
    }

    fn open(ctx: &mut Ctx<Parlor>, period: Period) {
        let pages = challenge_briefing(ctx, period);
        ctx.push(pages);
    }

    fn intro_help() -> String {
        "Up and Down choose a challenge, Enter hears its goal and twist, Escape goes back. Every player gets the same garden: a new daily each day, a weekly each Monday and a festival each month.".to_string()
    }
}

/// Stands in for the challenges menu when the launcher opens a challenge:
/// it opens the briefing, and once the garden closes, or is left from the
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
    /// Named as the challenges menu is, so leaving a garden lands here.
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
        "Opening the Rooftop Garden challenge.".to_string()
    }
}

/// The seed shed: what you can plant, and what unlocks the rest.
pub struct SeedShed {
    menu: MenuCore<Self, Parlor>,
}

impl SeedShed {
    pub fn new() -> SeedShed {
        SeedShed {
            menu: MenuCore::new("The seed shed").with_intro_help(
                "Every kind of seed, your varieties, keepsakes and requests. Up and Down read them, Escape goes back.",
            ),
        }
    }
}

impl Default for SeedShed {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for SeedShed {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "rooftop_garden_shed"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        parlor_core::shop_music::start(ctx, cues::MUSIC_SHED);
        hearken::menu::menu_enter(self, ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        parlor_core::shop_music::stop(ctx);
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let saved: Option<Garden> = parlor_core::Records::load(ctx, GARDEN_KEY);
        let harvests = saved.as_ref().map_or(0, |g| g.harvests);
        let mut items = Vec::new();
        for s in Species::ALL {
            let open = harvests >= s.unlock_at();
            let label = if open {
                format!("{}: sings {}", sentence(s.name()), s.voice())
            } else {
                let n = s.unlock_at() - harvests;
                format!(
                    "{}: locked, {n} more {} to unlock",
                    sentence(s.name()),
                    if n == 1 { "harvest" } else { "harvests" }
                )
            };
            let tip = s.tip().to_string();
            let say = tip.clone();
            items.push(
                MenuItem::new(label, move |_m: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.say(say.clone())
                })
                .select_sound(None)
                .help(tip),
            );
        }
        // Pumpkin seeds are in the packet in their season only.
        let today = parlor_core::challenges::today(ctx);
        if crate::events::pumpkin_open(today) {
            let tip = "Grows like a squash; in the packet until Thanksgiving.";
            items.push(
                MenuItem::new(
                    "Pumpkin: sings a low bassoon, until Thanksgiving",
                    move |_m: &mut Self, ctx: &mut Ctx<Parlor>| ctx.say(tip),
                )
                .select_sound(None)
                .help(tip),
            );
        }
        let book = load_collection(ctx);
        items.push(
            MenuItem::new(
                format!(
                    "Varieties found, {} of {}",
                    book.found.len(),
                    Variety::count()
                ),
                |_m: &mut Self, ctx: &mut Ctx<Parlor>| ctx.push(Catalogue::new()),
            )
            .help("Every variety your crosses have made. Enter on one plays its part."),
        );
        items.push(
            MenuItem::new(
                format!(
                    "Keepsakes, {} of {}",
                    book.keepsakes.len(),
                    Keepsake::ALL.len()
                ),
                |_m: &mut Self, ctx: &mut Ctx<Parlor>| ctx.push(Keepsakes::new()),
            )
            .help("Things for your rooftop, earned with stars."),
        );
        let requests = book
            .requests
            .iter()
            .map(|r| r.describe())
            .collect::<Vec<_>>()
            .join(" ");
        let label = match book.requests.len() {
            0 => "Requests, none".to_string(),
            n => format!("Requests, {n}"),
        };
        items.push(
            MenuItem::new(label, move |_m: &mut Self, ctx: &mut Ctx<Parlor>| {
                if requests.is_empty() {
                    ctx.say("No requests. Neighbours ask as the days go by.")
                } else {
                    ctx.say(requests.clone())
                }
            })
            .select_sound(None)
            .help("What your neighbours would like from your harvests. Each one filled earns a star."),
        );
        items.push(
            MenuItem::new(labels::BACK, |_m: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.play_ui(hearken::UiSound::Back);
                ctx.pop()
            })
            .help("Return to the Rooftop Garden title."),
        );
        items
    }
}

/// Every variety found: Enter plays its part.
pub struct Catalogue {
    menu: MenuCore<Self, Parlor>,
    playing: Option<Sound>,
}

impl Catalogue {
    pub fn new() -> Catalogue {
        Catalogue {
            menu: MenuCore::new("Varieties found").with_intro_help(
                "Up and Down read the varieties, Enter plays one's part, Escape goes back.",
            ),
            playing: None,
        }
    }

    fn stop(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Some(s) = self.playing.take() {
            ctx.audio.stop(s, 300);
        }
    }
}

impl Default for Catalogue {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for Catalogue {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "rooftop_garden_catalogue"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop(ctx);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let book = load_collection(ctx);
        let mut found = book.found.clone();
        found.sort_by_key(|f| f.variety);
        let mut items: Vec<MenuItem<Self, Parlor>> = found
            .into_iter()
            .map(|f| {
                let v = f.variety;
                let mut label = format!("{}, {}", sentence(&v.name()), v.describe());
                if f.heirloom() {
                    label.push_str(", heirloom");
                } else {
                    let n = f.harvests;
                    label.push_str(&format!(
                        ", {n} of {} harvests to heirloom",
                        breeding::HEIRLOOM_AT
                    ));
                }
                let key = Part::of(v.species, false).phrase_on(Chord::G, 0, v.traits.tone);
                MenuItem::new(label, move |m: &mut Self, ctx: &mut Ctx<Parlor>| {
                    m.stop(ctx);
                    m.playing = Some(ctx.audio.play(&key, Bus::MUSIC));
                })
                .select_sound(None)
                .help(v.species.tip())
            })
            .collect();
        if items.is_empty() {
            items.push(
                MenuItem::new("None yet", |_m: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.say("Grow two different plants side by side. Harvest a ripe plant while its neighbour is in bloom or ripe, and they may cross.")
                })
                .select_sound(None)
                .help("Grow two different plants side by side. Harvest a ripe plant while its neighbour is in bloom or ripe, and they may cross."),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_m: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.play_ui(hearken::UiSound::Back);
                ctx.pop()
            })
            .help("Return to the seed shed."),
        );
        items
    }
}

/// The keepsakes: what each does, and the stars the locked ones need.
pub struct Keepsakes {
    menu: MenuCore<Self, Parlor>,
}

impl Keepsakes {
    pub fn new() -> Keepsakes {
        Keepsakes {
            menu: MenuCore::new("Keepsakes").with_intro_help(
                "Up and Down read the keepsakes, Escape goes back. New varieties, heirlooms and filled requests each earn a star.",
            ),
        }
    }
}

impl Default for Keepsakes {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for Keepsakes {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "rooftop_garden_keepsakes"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let book = load_collection(ctx);
        let stars = book.stars();
        let mut items: Vec<MenuItem<Self, Parlor>> = Keepsake::ALL
            .into_iter()
            .map(|k| {
                let label = if book.has(k) {
                    format!("{}: {}", sentence(k.name()), k.about())
                } else {
                    let n = k.stars().saturating_sub(stars).max(1);
                    format!(
                        "{}, locked: {n} more {}",
                        sentence(k.name()),
                        if n == 1 { "star" } else { "stars" }
                    )
                };
                MenuItem::new(label, move |_m: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.say(k.about())
                })
                .select_sound(None)
                .help(k.about())
            })
            .collect();
        items.insert(
            0,
            MenuItem::new(
                format!("Stars: {stars}"),
                |_m: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.say("New varieties, heirlooms and filled requests each earn a star.")
                },
            )
            .select_sound(None)
            .help("New varieties, heirlooms and filled requests each earn a star."),
        );
        items.push(
            MenuItem::new(labels::BACK, |_m: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.play_ui(hearken::UiSound::Back);
                ctx.pop()
            })
            .help("Return to the seed shed."),
        );
        items
    }
}

/// How long the sound check lets a loop or music play.
const LOOP_PREVIEW: f64 = 5.0;
const MUSIC_PREVIEW: f64 = 12.0;

enum Preview {
    None,
    Held { sound: Sound, left: f64 },
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
            menu: MenuCore::new(labels::SOUND_CHECK).with_intro_help(
                "Up and Down move through the sounds you need in play. Enter plays the selected sound. Escape goes back.",
            ),
            preview: Preview::None,
        }
    }

    fn stop(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Preview::Held { sound, .. } = std::mem::replace(&mut self.preview, Preview::None) {
            ctx.audio.stop(sound, 300);
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
                    .play_at(entry.key, Vec3::new(-4.0, 1.0, 0.0), Bus::EFFECTS);
                self.preview = Preview::Sides {
                    key: entry.key,
                    wait: 1.8,
                };
            }
            Play::Loop => {
                ctx.stop_speech();
                let sound =
                    ctx.audio
                        .start_loop(entry.key, Some(Vec3::new(0.0, 2.0, 0.0)), Bus::EFFECTS);
                self.preview = Preview::Held {
                    sound,
                    left: LOOP_PREVIEW,
                };
            }
            Play::Music => {
                ctx.stop_speech();
                let sound = ctx.audio.start_loop(entry.key, None, Bus::MUSIC);
                self.preview = Preview::Held {
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
        "rooftop_garden_sounds"
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
                ctx.play_ui(hearken::UiSound::Back);
                ctx.pop()
            })
            .help("Return to the Rooftop Garden title."),
        );
        items
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let mut done = false;
        match &mut self.preview {
            Preview::None => {}
            Preview::Held { left, .. } => {
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
            self.preview = Preview::None;
        }
    }
}

/// Every key the sound check plays, for tests.
pub fn all_keys() -> Vec<&'static str> {
    let mut keys: Vec<&'static str> = SOUND_CHECK.iter().map(|e| e.key).collect();
    keys.extend(Species::ALL.map(cues::song));
    keys.sort();
    keys.dedup();
    keys
}
