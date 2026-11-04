//! The menus around the run: the sound check, the story archive and the
//! workshop.

use hearken::menu::{menu_enter, Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::{Pages, Parlor};

use crate::cues::{self, Play, SOUND_CHECK};
use crate::profile::{Profile, PARTS};
use crate::screens::{lane_x, APPROACH_DISTANCE};
use crate::story::{LOGS, STRANDS};

/// How long the sound check lets a loop or music play.
const LOOP_PREVIEW: f64 = 3.0;
const MUSIC_PREVIEW: f64 = 10.0;
/// How long a previewed arrival takes to reach you.
const APPROACH_PREVIEW: f64 = 2.4;

/// What the sound check is playing.
enum Preview {
    None,
    Sound {
        sound: Sound,
        left: f64,
        approach: bool,
    },
    Music {
        left: f64,
    },
    /// The lane demonstration: which lane is next, and how long until.
    Lanes {
        next: usize,
        wait: f64,
    },
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
                "Up and Down choose a sound, Enter plays it and says what it means, Escape goes back.",
            ),
            preview: Preview::None,
        }
    }

    fn stop(&mut self, ctx: &mut Ctx<Parlor>) {
        match std::mem::replace(&mut self.preview, Preview::None) {
            Preview::Sound { sound, .. } => ctx.audio.stop(sound, 80),
            Preview::Music { .. } => ctx.audio.play_music(None, false),
            _ => {}
        }
    }

    fn play(&mut self, ctx: &mut Ctx<Parlor>, index: usize) {
        self.stop(ctx);
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        let entry = &SOUND_CHECK[index];
        ctx.say(format!("{}. {}", entry.name, entry.meaning));
        match entry.play {
            Play::Once => {
                ctx.audio.play(entry.key, Bus::EFFECTS);
            }
            Play::Loop => {
                let sound = ctx.audio.start_loop(entry.key, None, Bus::EFFECTS);
                self.preview = Preview::Sound {
                    sound,
                    left: LOOP_PREVIEW,
                    approach: false,
                };
            }
            Play::Approach => {
                let pos = Vec3::new(0.0, APPROACH_DISTANCE, 0.0);
                let sound = ctx.audio.start_loop(entry.key, Some(pos), Bus::EFFECTS);
                self.preview = Preview::Sound {
                    sound,
                    left: APPROACH_PREVIEW,
                    approach: true,
                };
            }
            Play::Music => {
                ctx.audio.play_music(Some(entry.key), true);
                self.preview = Preview::Music {
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
        "neon_rail_sounds"
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
        let mut items = vec![MenuItem::new(
            "Lane positions: a chime from the left, center, then right lane",
            |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                s.stop(ctx);
                ctx.audio.set_listener(Vec3::ZERO, 0.0);
                ctx.say("Lane positions. You are in the center lane.");
                s.preview = Preview::Lanes {
                    next: 0,
                    wait: 1.6,
                };
            },
        )
        .help("Hear where each lane sounds from while you are in the center lane: left, straight ahead, then right.")];
        for (i, entry) in SOUND_CHECK.iter().enumerate() {
            items.push(
                MenuItem::new(entry.name, move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    s.play(ctx, i)
                })
                .select_sound(None)
                .help(entry.meaning),
            );
        }
        items
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let mut done = false;
        match &mut self.preview {
            Preview::None => {}
            Preview::Sound {
                sound,
                left,
                approach,
            } => {
                *left -= dt;
                if *approach {
                    let y = APPROACH_DISTANCE * (*left / APPROACH_PREVIEW).max(0.0) as f32;
                    ctx.audio.set_position(*sound, Some(Vec3::new(0.0, y, 0.0)));
                }
                done = *left <= 0.0;
            }
            Preview::Music { left } => {
                *left -= dt;
                done = *left <= 0.0;
            }
            Preview::Lanes { next, wait } => {
                *wait -= dt;
                if *wait <= 0.0 {
                    let lane = *next;
                    let name = ["Left lane.", "Center lane, straight ahead.", "Right lane."][lane];
                    ctx.say(name);
                    let pos = Vec3::new(lane_x(lane), 2.0, 0.0);
                    ctx.audio.play_at(cues::CLEAN, pos, Bus::EFFECTS);
                    if lane >= 2 {
                        done = true;
                    } else {
                        *next += 1;
                        *wait = 1.6;
                    }
                }
            }
        }
        if done {
            self.stop(ctx);
        }
    }
}

/// The story archive: the data core's logs, then a row for each strand,
/// then the event log. Enter on a strand opens its logs.
pub struct Archive {
    menu: MenuCore<Self, Parlor>,
}

impl Archive {
    pub fn new() -> Archive {
        Archive {
            menu: MenuCore::new("Story archive"),
        }
    }
}

impl Default for Archive {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for Archive {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "neon_rail_archive"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let profile = Profile::load(ctx);
        let mut items: Vec<MenuItem<Self, Parlor>> = vec![MenuItem::new(
            format!(
                "Data core, {} of {}",
                profile.story_stage.min(LOGS.len()),
                LOGS.len()
            ),
            |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.push(StrandMenu::new(None)),
        )
        .help("The skimmer's own logs, recovered run by run and as intel adds up.")];
        for (i, strand) in STRANDS.iter().enumerate() {
            let have = profile.strand_recovered(i);
            if have > 0 {
                items.push(
                    MenuItem::new(
                        format!("{}, {have} of {}", strand.name, strand.logs.len()),
                        move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                            ctx.push(StrandMenu::new(Some(i)))
                        },
                    )
                    .help(strand.blurb),
                );
            } else {
                let how = strand.logs[0].unlock.how();
                items.push(
                    MenuItem::new(
                        format!("{}, locked", strand.name),
                        move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                            ctx.say(format!("Locked. {}", strand.logs[0].unlock.how()))
                        },
                    )
                    .select_sound(Some(cues::DENIED))
                    .help(format!("{} {how}", strand.blurb)),
                );
            }
        }
        if crate::challenge::event_log_open(ctx) {
            items.push(
                MenuItem::new(
                    format!("Event log. {}", crate::challenge::EVENT_LOG_TITLE),
                    |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        let pages: Vec<String> = crate::challenge::EVENT_LOG
                            .iter()
                            .map(|p| p.to_string())
                            .collect();
                        ctx.push(Pages::new(pages))
                    },
                )
                .help("The Black Sky event's own log. Enter reads it."),
            );
        } else {
            items.push(
                MenuItem::new(
                    "Locked event log",
                    |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        ctx.say("Event log locked. Fly a monthly Black Sky event to its end.")
                    },
                )
                .select_sound(Some(cues::DENIED))
                .help(
                    "Locked. Fly a monthly Black Sky event to its end, from the Challenges menu.",
                ),
            );
        }
        items.push(
            MenuItem::new("Back", |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.pop())
                .help("Return to the Neon Rail menu."),
        );
        items
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let row = self.current_help(ctx).unwrap_or_default();
        format!("{row} Each row is a strand of the story. Enter opens its logs. A locked strand says how its first log is recovered.")
    }
}

/// A log row: its title, pages and how it opens.
type Row = (&'static str, &'static [&'static str], String);

/// The logs of one strand: the data core when `strand` is `None`.
pub struct StrandMenu {
    menu: MenuCore<Self, Parlor>,
    strand: Option<usize>,
}

impl StrandMenu {
    pub fn new(strand: Option<usize>) -> StrandMenu {
        let title = match strand {
            Some(i) => STRANDS[i].name,
            None => "Data core",
        };
        StrandMenu {
            menu: MenuCore::new(title),
            strand,
        }
    }

    /// Each log as (title, pages, how it opens), and how many are open.
    fn logs(&self, profile: &Profile) -> (Vec<Row>, usize) {
        match self.strand {
            None => (
                LOGS.iter()
                    .map(|l| (l.title, l.pages, l.how.to_string()))
                    .collect(),
                profile.story_stage.min(LOGS.len()),
            ),
            Some(i) => (
                STRANDS[i]
                    .logs
                    .iter()
                    .map(|l| (l.title, l.pages, l.unlock.how()))
                    .collect(),
                profile.strand_recovered(i),
            ),
        }
    }
}

impl Menu for StrandMenu {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "neon_rail_strand"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let profile = Profile::load(ctx);
        let (logs, open) = self.logs(&profile);
        let mut items: Vec<MenuItem<Self, Parlor>> = logs
            .into_iter()
            .enumerate()
            .map(|(i, (title, pages, how))| {
                if i < open {
                    MenuItem::new(
                        format!("Log {}. {title}", i + 1),
                        move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                            let pages: Vec<String> = pages
                                .iter()
                                .enumerate()
                                .map(|(n, p)| {
                                    if n == 0 {
                                        format!("{title}. {p}")
                                    } else {
                                        p.to_string()
                                    }
                                })
                                .collect();
                            ctx.push(Pages::new(pages))
                        },
                    )
                    .help("A recovered log. Enter reads it.")
                } else {
                    let said = format!("Story log locked. {how} Logs open in order.");
                    MenuItem::new(
                        format!("Locked log {}", i + 1),
                        move |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.say(said.clone()),
                    )
                    .select_sound(Some(cues::DENIED))
                    .help(format!("Locked. {how}"))
                }
            })
            .collect();
        items.push(
            MenuItem::new("Back", |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.pop())
                .help("Return to the story archive."),
        );
        items
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let row = self.current_help(ctx).unwrap_or_default();
        format!("{row} Recovered logs open with Enter. Locked logs need more runs, and each says how it is recovered.")
    }
}

/// Permanent parts bought with credits and parts.
pub struct Workshop {
    menu: MenuCore<Self, Parlor>,
}

impl Workshop {
    pub fn new() -> Workshop {
        Workshop {
            menu: MenuCore::new("Workshop"),
        }
    }
}

impl Default for Workshop {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for Workshop {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "neon_rail_workshop"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        parlor_core::shop_music::start(ctx, cues::MUSIC_WORKSHOP);
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
        let p = Profile::load(ctx);
        let mut items = vec![MenuItem::inert(format!(
            "You have {} credits and {} parts",
            p.credits, p.parts
        ))
        .help("Credits come from score and extraction, and parts from clearing sectors. Both are paid at every debrief, even after a failed run.")];
        for (i, part) in PARTS.iter().enumerate() {
            let lock = p.part_lock(part);
            let label = if p.owns(part.id) {
                format!("{}, fitted", part.name)
            } else if lock.is_some() {
                parlor_core::menus::labels::locked(part.name)
            } else if p.rank() < part.rank {
                format!(
                    "{}, {} credits and {} parts, needs rank {}",
                    part.name, part.credits, part.parts, part.rank
                )
            } else if part.parts > 0 {
                format!(
                    "{}, {} credits and {} part{}",
                    part.name,
                    part.credits,
                    part.parts,
                    if part.parts == 1 { "" } else { "s" }
                )
            } else {
                format!("{}, {} credits", part.name, part.credits)
            };
            items.push(
                MenuItem::new(label, move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    let part = &PARTS[i];
                    let mut p = Profile::load(ctx);
                    match p.buy(part) {
                        Ok(()) => {
                            p.save(ctx);
                            ctx.audio.play(cues::UNLOCK, Bus::UI);
                            ctx.say(format!(
                                "{} fitted. {} credits and {} left.",
                                part.name,
                                p.credits,
                                crate::profile::parts_phrase(p.parts)
                            ));
                            s.refresh(ctx, true);
                        }
                        Err(why) => {
                            ctx.audio.play(cues::DENIED, Bus::UI);
                            ctx.say(why);
                        }
                    }
                })
                .select_sound(None)
                .help(match &lock {
                    Some(how) => format!("{how} {}", part.blurb),
                    None => part.blurb.to_string(),
                }),
            );
        }
        items.push(
            MenuItem::new("Back", |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.pop())
                .help("Return to the Neon Rail menu."),
        );
        items
    }
}
