//! The play screens: a night under the stars, and a meteor watch.

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State};
use parlor_core::challenges::today;
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{Challenge, Date, Pages, Parlor, ParlorAction, Records, ScoreResult, Season};

use crate::almanac::{self, ShowerNight};
use crate::challenge::{self, CHALLENGES_NAME};
use crate::journal::Journal;
use crate::rules::{Cue, Dir, Goal, Night};
use crate::shower::{MeteorWatch, WatchCue, Wish};
use crate::sky::{open_tier, tier_needs, CONSTELLATIONS, WONDERS};
use crate::{sound, HELP, ID, TITLE_NAME};

/// The night's `State::name`.
pub const NIGHT_NAME: &str = "stargazer_night";
/// The meteor watch's `State::name`.
pub const SHOWER_NAME: &str = "stargazer_shower";

/// Keys a player presses for their screen reader or as a modifier, which
/// play leaves alone.
fn is_quiet_key(key: Key) -> bool {
    matches!(
        key,
        Key::LCtrl
            | Key::RCtrl
            | Key::LShift
            | Key::RShift
            | Key::LAlt
            | Key::RAlt
            | Key::Insert
            | Key::Other(_)
    )
}

/// "22 October".
pub fn day_words(date: Date) -> String {
    format!("{} {}", date.day, almanac::month_name(date.month))
}

/// "a winter night", "an autumn night".
fn a_night(season: Season) -> String {
    let article = if season == Season::Autumn { "an" } else { "a" };
    format!("{article} {} night", season.name())
}

/// A seed that changes from night to night but is the same for the same
/// date and journal, so a night can be replayed in a test.
fn night_seed(date: Date, n: u32) -> u64 {
    (date.days() as u64).wrapping_mul(0x9E37_79B9) ^ (n as u64).wrapping_mul(0x85EB_CA6B)
}

/// Seconds a first Enter waits for the second that heads home.
pub const HOME_WINDOW: f64 = 4.0;

/// Enter twice heads home, so a double tap from the title does not end
/// the night at once.
#[derive(Default)]
struct HomeKey {
    /// Seconds left for the second Enter.
    armed: f64,
}

impl HomeKey {
    /// An Enter: true when it is the second, inside the window.
    fn press(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        if self.armed > 0.0 {
            self.armed = 0.0;
            return true;
        }
        self.armed = HOME_WINDOW;
        ctx.say("Enter again heads home.");
        false
    }

    fn tick(&mut self, dt: f64) {
        self.armed = (self.armed - dt).max(0.0);
    }
}

/// A night on the hill.
pub struct NightScreen {
    pub night: Night,
    challenge: Option<Challenge>,
    date: Date,
    last: String,
    ended: bool,
    /// Opened under the tutorial, where Escape leaves the lesson and
    /// Enter is kept back.
    tutorial: bool,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
    home: HomeKey,
}

impl NightScreen {
    /// A free night for today, with what the journal has opened.
    pub fn free(ctx: &mut Ctx<Parlor>) -> NightScreen {
        let date = today(ctx);
        let journal = Journal::load(ctx);
        let logged = journal.logged_indices().into_iter().collect();
        let night = Night::free(
            almanac::sky_season(date),
            logged,
            almanac::full_moon(date),
            night_seed(date, journal.nights),
        );
        let mut night = night;
        night.primary_key = ctx.control_name(ParlorAction::Primary);
        NightScreen {
            status_key: Default::default(),
            home: HomeKey::default(),
            night,
            challenge: None,
            date,
            last: String::new(),
            ended: false,
            tutorial: false,
        }
    }

    /// A challenge night, from its seed.
    pub fn challenge(ctx: &mut Ctx<Parlor>, ch: Challenge) -> NightScreen {
        let date = today(ctx);
        let journal = Journal::load(ctx);
        let logged = journal.logged_indices().into_iter().collect();
        let season = almanac::sky_season(date);
        let goal = challenge::goal(&ch, season);
        let night = Night::challenge(season, goal, logged, almanac::full_moon(date), ch.seed);
        let mut night = night;
        night.primary_key = ctx.control_name(ParlorAction::Primary);
        NightScreen {
            status_key: Default::default(),
            home: HomeKey::default(),
            night,
            challenge: Some(ch),
            date,
            last: String::new(),
            ended: false,
            tutorial: false,
        }
    }

    /// This night as the tutorial's sky: its controls line names the
    /// tutorial's Escape, not the night's own Enter and pause.
    pub fn for_tutorial(mut self) -> NightScreen {
        self.tutorial = true;
        self
    }

    pub fn is_challenge(&self) -> bool {
        self.challenge.is_some()
    }

    fn say(&mut self, ctx: &mut Ctx<Parlor>, text: String) {
        ctx.say(text.clone());
        self.last = text;
    }

    fn queue(&mut self, ctx: &mut Ctx<Parlor>, text: String) {
        ctx.say_queued(text.clone());
        if self.last.is_empty() {
            self.last = text;
        } else {
            self.last = format!("{} {text}", self.last);
        }
    }

    fn controls(ctx: &Ctx<Parlor>, tutorial: bool) -> String {
        let l = ctx.control_name(ParlorAction::Left);
        let r = ctx.control_name(ParlorAction::Right);
        let u = ctx.control_name(ParlorAction::Up);
        let d = ctx.control_name(ParlorAction::Down);
        let p = ctx.control_name(ParlorAction::Primary);
        let t = ctx.control_name(ParlorAction::Secondary);
        let s = ctx.control_name(ParlorAction::Status);
        let rep = ctx.control_name(ParlorAction::Repeat);
        let h = ctx.control_name(ParlorAction::Hint);
        format!(
            "{} and {r} sweep the listening point round the sky, {u} and {d} raise and lower it. {p} traces the star you rest on. {h} gives a hint, {t} the guide tone. {s} says where you are, twice for more, {rep} repeats. {}",
            sentence_start(&l),
            if tutorial {
                "Escape leaves the tutorial."
            } else {
                "Enter twice heads home, Escape pauses."
            }
        )
    }

    /// What the night is, as it starts.
    fn intro(&self) -> String {
        let season = self.night.season();
        match (&self.challenge, &self.night.goal) {
            (Some(ch), Goal::Stars(_)) => {
                let target = self
                    .night
                    .hunt_target()
                    .map(|(c, s)| crate::rules::star_words(c, s))
                    .unwrap_or_default();
                format!(
                    "{}. Find each star in turn. First, find {target}.",
                    challenge::name(ch.period)
                )
            }
            (Some(ch), _) => format!(
                "{}. Trace {}.",
                challenge::name(ch.period),
                self.night.up_words()
            ),
            (None, _) => format!(
                "{}, {}. You lie back on the hilltop, facing south. Up tonight: {}.",
                sentence_start(&a_night(season)),
                day_words(self.date),
                self.night.up_words()
            ),
        }
    }

    fn apply(&mut self, ctx: &mut Ctx<Parlor>, cues: Vec<Cue>) {
        let mut complete = false;
        for cue in cues {
            match cue {
                Cue::Sound {
                    key,
                    pan,
                    ahead,
                    gain,
                    pitch,
                } => {
                    sound::play_from(ctx, key, pan, ahead, gain, pitch);
                }
                Cue::Say(text) => self.say(ctx, text),
                Cue::Queue(text) => self.queue(ctx, text),
                Cue::Traced(c) => {
                    ctx.audio
                        .play(&sound::motif(CONSTELLATIONS[c].id), Bus::MUSIC);
                    let mut journal = Journal::load(ctx);
                    journal.log(c, self.date);
                    journal.store(ctx);
                }
                Cue::Found(w) => {
                    let mut journal = Journal::load(ctx);
                    journal.find(w, self.date);
                    journal.store(ctx);
                }
                Cue::Greeted => {
                    if let Some(moon) = self.night.moon {
                        let mut journal = Journal::load(ctx);
                        journal.greet(moon.name, self.date);
                        journal.store(ctx);
                    }
                }
                Cue::Complete => complete = true,
            }
        }
        if complete {
            self.finish(ctx);
        }
    }

    /// The night ends: the results, and back to the title or challenges.
    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.ended {
            return;
        }
        self.ended = true;
        ctx.audio.set_bed(None);
        ctx.audio.play_music(Some(sound::MUSIC_HOME), false);
        let mut journal = Journal::load(ctx);
        journal.nights += 1;
        journal.store(ctx);
        if let Some(ch) = self.challenge.take() {
            let score = self.night.score();
            let pages = challenge::finish(ctx, &ch, &self.night);
            let summary = format!("{} result: {score} points", ch.title());
            let period = ch.period;
            let results = Results::new(summary, pages.clone(), Back::Challenges(CHALLENGES_NAME))
                .again(
                    labels::challenge_again(period),
                    "The same sky and goals, straight away. Only your best result is kept.",
                    move |ctx| {
                        let ch = Challenge::current(ctx, ID, period);
                        let screen = NightScreen::challenge(ctx, ch);
                        ctx.replace(screen)
                    },
                );
            let mut results = Some(results);
            ctx.replace(Pages::new(pages).then(move |ctx| {
                if let Some(results) = results.take() {
                    ctx.replace(results);
                }
            }));
            return;
        }
        let pages = night_pages(&self.night, &journal);
        let summary = night_summary(&self.night);
        let results = Results::new(summary, pages.clone(), Back::Title(TITLE_NAME)).again(
            labels::PLAY_AGAIN,
            "Another night under the same season's sky.",
            |ctx| {
                let screen = NightScreen::free(ctx);
                ctx.replace(screen)
            },
        );
        let mut results = Some(results);
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(results);
            }
        }));
    }
}

/// The results menu's title after a free night.
pub fn night_summary(night: &Night) -> String {
    match night.done.len() {
        0 => "Night over. No constellations traced".to_string(),
        1 => "Night over. 1 constellation traced".to_string(),
        n => format!("Night over. {n} constellations traced"),
    }
}

fn and_list(names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => names[0].clone(),
        n => format!("{} and {}", names[..n - 1].join(", "), names[n - 1]),
    }
}

/// The pages after a free night.
pub fn night_pages(night: &Night, journal: &Journal) -> Vec<String> {
    let mut first = Vec::new();
    let traced: Vec<String> = night
        .done
        .iter()
        .map(|&c| CONSTELLATIONS[c].name.to_string())
        .collect();
    first.push(match traced.len() {
        0 => "You head home. Nothing traced tonight.".to_string(),
        _ => format!("You head home. Tonight you traced {}.", and_list(&traced)),
    });
    if !night.found.is_empty() {
        let found: Vec<String> = night
            .found
            .iter()
            .map(|&w| WONDERS[w].name.to_string())
            .collect();
        first.push(format!("You found {}.", and_list(&found)));
    }
    if night.greeted {
        if let Some(m) = night.moon {
            first.push(format!("You greeted {}.", m.name));
        }
    }
    if !night.newly.is_empty() {
        let new: Vec<String> = night
            .newly
            .iter()
            .map(|&c| CONSTELLATIONS[c].name.to_string())
            .collect();
        first.push(format!("New in your star journal: {}.", and_list(&new)));
    }
    let mut second = format!("Your star journal holds {}.", journal.tally());
    let logged = journal.logged.len();
    let tier = open_tier(logged);
    if tier < 2 {
        let more = tier_needs(tier + 1) - logged;
        second.push_str(&match more {
            1 => " Trace 1 more new constellation and more will rise.".to_string(),
            n => format!(" Trace {n} more new constellations and more will rise."),
        });
    }
    vec![first.join(" "), second]
}

impl State<Parlor> for NightScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        NIGHT_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::hill(ctx, self.night.season());
        let intro = self.intro();
        self.say(ctx, intro);
        let mut extra = Vec::new();
        if let Some(moon) = self.night.moon {
            extra.push(format!(
                "{} rises low in the east tonight.",
                sentence_start(moon.name)
            ));
        }
        if self.challenge.is_none() {
            if let Some(shower) = almanac::shower_on(self.date) {
                extra.push(format!(
                    "Tonight the {} are falling. Meteor shower, on the title, opens the watch.",
                    shower.shower.name()
                ));
            }
        }
        let known = menus::tutorial_read(ctx, ID) || Journal::load(ctx).nights > 0;
        if self.tutorial || !known {
            extra.push(NightScreen::controls(ctx, self.tutorial));
        } else {
            extra.push("F1 lists the controls.".to_string());
        }
        let text = extra.join(" ");
        self.queue(ctx, text);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.ended {
            return;
        }
        sound::hill(ctx, self.night.season());
        let s = self.night.status_layers().remove(0);
        self.say(ctx, format!("Back on the hill. {s}"));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.ended {
            return;
        }
        let action = ctx.action_for(event);
        if action.is_none() && event.pressed() == Some(Key::Escape) {
            if !event.is_repeat() {
                let leave = if self.is_challenge() {
                    Leave::Challenge(CHALLENGES_NAME)
                } else {
                    Leave::Title(TITLE_NAME)
                };
                menus::pause(ctx, HELP, leave);
            }
            return;
        }
        let stepping = matches!(
            action,
            Some(ParlorAction::Left | ParlorAction::Right | ParlorAction::Up | ParlorAction::Down)
        );
        if event.is_repeat() && !stepping {
            return;
        }
        let cues = match action {
            Some(ParlorAction::Left) => self.night.step(Dir::Left),
            Some(ParlorAction::Right) => self.night.step(Dir::Right),
            Some(ParlorAction::Up) => self.night.step(Dir::Up),
            Some(ParlorAction::Down) => self.night.step(Dir::Down),
            Some(ParlorAction::Primary) => self.night.primary(),
            Some(ParlorAction::Secondary) => self.night.toggle_guide(),
            Some(ParlorAction::Hint) => self.night.hint(),
            Some(ParlorAction::Status) => {
                let layers = self.night.status_layers();
                self.status_key.speak(ctx, layers);
                return;
            }
            Some(ParlorAction::Repeat) => {
                if self.last.is_empty() {
                    ctx.say(self.night.status_layers().remove(0));
                } else {
                    ctx.say(self.last.clone());
                }
                return;
            }
            None => {
                match event {
                    InputEvent::KeyDown {
                        key: Key::Return | Key::KpEnter,
                        repeat: false,
                        ..
                    } => {
                        if self.home.press(ctx) {
                            self.finish(ctx);
                        }
                    }
                    InputEvent::KeyDown {
                        key, repeat: false, ..
                    } if !is_quiet_key(*key) => {
                        sound::play(ctx, sound::NOTHING);
                        ctx.say("Nothing happens. F1 lists the controls.");
                    }
                    _ => {}
                }
                return;
            }
        };
        self.apply(ctx, cues);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.ended {
            return;
        }
        self.home.tick(dt);
        let cues = self.night.update(dt);
        self.apply(ctx, cues);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let mut out = vec![match &self.challenge {
            Some(ch) => format!("Stargazer: {}", challenge::name(ch.period)),
            None => format!("Stargazer: {}", a_night(self.night.season())),
        }];
        out.push(self.night.status());
        if let Some(words) = self.night.rest_words() {
            out.push(words);
        }
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let p = ctx.control_name(ParlorAction::Primary);
        let mut now = format!(
            "{} Rest the point on a star that is next in its constellation, and {p} traces it.",
            self.night.status()
        );
        if let Some(ch) = &self.challenge {
            now.push_str(&format!(
                " This is the {} challenge, {}. Leaving from the pause menu does not count it.",
                ch.period.label().to_lowercase(),
                challenge::name(ch.period)
            ));
        }
        format!("{now} {}", NightScreen::controls(ctx, self.tutorial))
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

/// Lines said now and then as wishes add up.
const WISH_LINES: [&str; 8] = [
    "That one left a glowing trail.",
    "A long one, right across the sky.",
    "Somewhere a dog barks, far down the hill.",
    "The grass is cool and damp under you.",
    "Each shooting star is a speck of dust, no bigger than a grain of sand.",
    "A breeze moves through the grass and is gone.",
    "The sky seems deeper the longer you watch.",
    "Two in a row, nearly.",
];

/// A night's meteor watch.
pub struct ShowerScreen {
    pub watch: MeteorWatch,
    pub tonight: ShowerNight,
    date: Date,
    streak: Option<Sound>,
    last: String,
    told_first: bool,
    ended: bool,
    home: HomeKey,
}

impl ShowerScreen {
    /// Tonight's watch, if a shower is falling.
    pub fn open(ctx: &mut Ctx<Parlor>) -> Option<ShowerScreen> {
        let date = today(ctx);
        let tonight = almanac::shower_on(date)?;
        let journal = Journal::load(ctx);
        Some(ShowerScreen {
            watch: MeteorWatch::new(tonight.strength, night_seed(date, journal.watches) ^ 0x5157),
            tonight,
            date,
            streak: None,
            last: String::new(),
            told_first: false,
            ended: false,
            home: HomeKey::default(),
        })
    }

    fn say(&mut self, ctx: &mut Ctx<Parlor>, text: String) {
        ctx.say(text.clone());
        self.last = text;
    }

    fn controls(ctx: &Ctx<Parlor>) -> String {
        let p = ctx.control_name(ParlorAction::Primary);
        let t = ctx.control_name(ParlorAction::Secondary);
        let s = ctx.control_name(ParlorAction::Status);
        format!(
            "Listen for the whoosh of a shooting star. {} makes a wish while one passes. {t} tells the shower's story, {s} counts your wishes. Enter twice heads home, Escape pauses.",
            sentence_start(&p)
        )
    }

    pub fn status(&self) -> String {
        let w = &self.watch;
        let wishes = match w.wishes {
            1 => "1 wish".to_string(),
            n => format!("{n} wishes"),
        };
        let seen = match w.seen {
            1 => "1 shooting star seen".to_string(),
            n => format!("{n} shooting stars seen"),
        };
        let now = if w.passing() {
            "one passing now"
        } else {
            "the sky quiet for the moment"
        };
        format!(
            "The {}, {}, {wishes}, {seen}, {now}.",
            self.tonight.shower.name(),
            self.tonight.strength_words()
        )
    }

    fn wish(&mut self, ctx: &mut Ctx<Parlor>) {
        match self.watch.wish() {
            Wish::Made { bright } => {
                sound::play(ctx, sound::WISH);
                let n = self.watch.wishes;
                let count = match n {
                    1 => "1 wish tonight".to_string(),
                    n => format!("{n} wishes tonight"),
                };
                let text = if bright {
                    format!("Wish made, on a fireball. {}.", sentence_start(&count))
                } else {
                    format!("Wish made. {}.", sentence_start(&count))
                };
                self.say(ctx, text);
                if n % 4 == 3 {
                    let line = WISH_LINES[(n as usize / 4) % WISH_LINES.len()];
                    ctx.say_queued(line);
                }
            }
            Wish::Already => {
                sound::play(ctx, sound::NOT_YET);
                self.say(ctx, "Already wished on that one.".to_string());
            }
            Wish::NoMeteor => {
                sound::play(ctx, sound::NOTHING);
                self.say(ctx, "No shooting star just now.".to_string());
            }
        }
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.ended {
            return;
        }
        self.ended = true;
        if let Some(s) = self.streak.take() {
            ctx.audio.stop(s, 200);
        }
        ctx.audio.set_bed(None);
        ctx.audio.play_music(Some(sound::MUSIC_HOME), false);
        let wishes = self.watch.wishes;
        let mut journal = Journal::load(ctx);
        let total = journal.wish(&self.tonight.wish_key(), wishes);
        journal.store(ctx);
        let board = format!("wishes {}", self.tonight.shower.id());
        // An empty watch is no best: nothing goes on the board.
        let record = (wishes > 0).then(|| Records::record(ctx, ID, &board, wishes as u64));
        let name = self.tonight.shower.name();
        let count = match wishes {
            1 => "1 wish".to_string(),
            n => format!("{n} wishes"),
        };
        let seen = match self.watch.seen {
            1 => "1 shooting star".to_string(),
            n => format!("{n} shooting stars"),
        };
        let mut first = format!(
            "You head home. {} on the {name} tonight, from {seen}.",
            sentence_start(&count)
        );
        if let Some(ScoreResult::NewBest { previous }) = record {
            first.push_str(&format!(
                " Your best watch of the {name}, beating {previous}."
            ));
        }
        let year = self.tonight.peak.year;
        // On a run-up night the peak is still to come; after it, the next
        // shower, counted from today.
        let now = today(ctx);
        let to_peak = self.tonight.peak.days() - now.days();
        let coming = if to_peak > 0 {
            format!(
                "The peak is {}.",
                parlor_core::calendar::days_phrase(to_peak)
            )
        } else {
            almanac::next_shower_words_after(Date::from_days(self.tonight.peak.days() + 2), now)
        };
        let second = format!(
            "That makes {total} {} on the {name} in {year}. {}",
            if total == 1 { "wish" } else { "wishes" },
            coming
        );
        let pages = vec![first, second];
        let summary = format!("Watch over. {}", sentence_start(&count));
        let results = Results::new(summary, pages.clone(), Back::Title(TITLE_NAME)).again(
            labels::PLAY_AGAIN,
            "Watch the shower again tonight.",
            |ctx| match ShowerScreen::open(ctx) {
                Some(screen) => ctx.replace(screen),
                None => {
                    sound::play(ctx, sound::NOTHING);
                    ctx.say("The shower is over for this year.");
                }
            },
        );
        let mut results = Some(results);
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(results);
            }
        }));
    }
}

impl State<Parlor> for ShowerScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        SHOWER_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::hill(ctx, almanac::sky_season(self.date));
        ctx.audio.play_music(Some(sound::MUSIC_SHOWER), true);
        let intro = format!(
            "The {}, {}. You lie back on the hilltop. They seem to come from {}.",
            self.tonight.shower.name(),
            self.tonight.strength_words(),
            self.tonight.shower.radiant()
        );
        self.say(ctx, intro);
        // The tutorial never teaches wishes or Tab, so only a past watch counts.
        let known = Journal::load(ctx).watches > 0;
        if known {
            ctx.say_queued("F1 lists the controls.");
        } else {
            ctx.say_queued(ShowerScreen::controls(ctx));
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Some(s) = self.streak.take() {
            ctx.audio.stop(s, 200);
        }
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.ended {
            return;
        }
        sound::hill(ctx, almanac::sky_season(self.date));
        ctx.audio.play_music(Some(sound::MUSIC_SHOWER), true);
        let s = self.status();
        self.say(ctx, format!("Back on the hill. {s}"));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.ended {
            return;
        }
        let action = ctx.action_for(event);
        if action.is_none() && event.pressed() == Some(Key::Escape) {
            if !event.is_repeat() {
                menus::pause(ctx, HELP, Leave::Title(TITLE_NAME));
            }
            return;
        }
        if event.is_repeat() {
            return;
        }
        match action {
            Some(ParlorAction::Primary) => self.wish(ctx),
            Some(ParlorAction::Secondary) => {
                let lore = self.tonight.shower.lore().to_string();
                self.say(ctx, lore);
            }
            Some(ParlorAction::Status) => {
                let s = self.status();
                ctx.say(s);
            }
            Some(ParlorAction::Repeat) => {
                if self.last.is_empty() {
                    ctx.say(self.status());
                } else {
                    ctx.say(self.last.clone());
                }
            }
            Some(ParlorAction::Hint) => {
                let p = ctx.control_name(ParlorAction::Primary);
                let often = if self.tonight.is_peak() {
                    "every few seconds"
                } else {
                    "every 15 seconds or so"
                };
                self.say(
                    ctx,
                    format!("Shooting stars come {often} tonight. Press {p} during the whoosh."),
                );
            }
            Some(_) => {
                sound::play(ctx, sound::NOTHING);
                self.say(ctx, "No need to move: you watch the whole sky.".to_string());
            }
            None => match event {
                InputEvent::KeyDown {
                    key: Key::Return | Key::KpEnter,
                    ..
                } => {
                    if self.home.press(ctx) {
                        self.finish(ctx);
                    }
                }
                InputEvent::KeyDown { key, .. } if !is_quiet_key(*key) => {
                    sound::play(ctx, sound::NOTHING);
                    ctx.say("Nothing happens. F1 lists the controls.");
                }
                _ => {}
            },
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.ended {
            return;
        }
        self.home.tick(dt);
        for cue in self.watch.update(dt) {
            match cue {
                WatchCue::Start { pan, bright } => {
                    let key = if bright {
                        sound::FIREBALL
                    } else {
                        sound::METEOR
                    };
                    let s =
                        sound::play_from(ctx, key, pan, 0.6, if bright { 1.0 } else { 0.8 }, 1.0);
                    self.streak = Some(s);
                    if !self.told_first {
                        self.told_first = true;
                        let p = ctx.control_name(ParlorAction::Primary);
                        ctx.say(format!(
                            "A shooting star. {} makes a wish.",
                            sentence_start(&p)
                        ));
                    }
                }
                WatchCue::Move { pan } => {
                    if let Some(s) = self.streak {
                        ctx.audio
                            .set_position(s, Some(hearken::Vec3::new(pan, 0.6, 0.0)));
                    }
                }
                WatchCue::Gone { .. } => self.streak = None,
            }
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        vec![
            format!("Stargazer: the {}", self.tonight.shower.name()),
            self.status(),
        ]
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        format!("{} {}", self.status(), ShowerScreen::controls(ctx))
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

mod visuals_nightscreen;

mod visuals_showerscreen;
