//! Midsummer night on screen: the synchronous firefly show round the
//! bonfire, and the year's keepsake after it.

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, Vec3};
use parlor_core::challenges::today;
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{Pages, Parlor, ParlorAction};

use crate::almanac;
use crate::progress::{self, KeptSake};
use crate::rules::run_seed;
use crate::show::{Press, Show, ShowNote, FLASHES, ROUNDS, ROUND_BEATS, SWARM};
use crate::sound;
use crate::species::Species;
use crate::{HELP, TITLE_NAME};

/// The show's `State::name`.
pub const NIGHT_NAME: &str = "firefly_meadow_midsummer";

/// Where the walker sits: just south of the bonfire.
const SEAT: Vec3 = Vec3::new(0.0, -2.0, 0.0);

fn v3(p: (f64, f64)) -> Vec3 {
    Vec3::new(p.0 as f32, p.1 as f32, 0.0)
}

/// The page before the show.
pub fn intro_page(ctx: &Ctx<Parlor>) -> String {
    let p = ctx.control_name(ParlorAction::Primary);
    format!(
        "Midsummer night. A bonfire crackles in the middle of the meadow, and the fireflies gather round it. They flash together, {FLASHES} flashes in a row, then rest. Some drift out of step and flash a moment late. Press {p} with each flash to bring a straggler back into the chorus. The show lasts {ROUNDS} rounds."
    )
}

/// Starts midsummer night: its page, then the show.
pub fn start(ctx: &mut Ctx<Parlor>, replace: bool) {
    let page = intro_page(ctx);
    let mut screen = Some(Night::new(run_seed()));
    let pages = Pages::briefing([page]).then(move |ctx| {
        if let Some(screen) = screen.take() {
            ctx.replace(screen);
        }
    });
    if replace {
        ctx.replace(pages);
    } else {
        ctx.push(pages);
    }
}

pub struct Night {
    pub show: Show,
    loops: Vec<Sound>,
    last: String,
    taught_off: bool,
    ended: bool,
}

impl Night {
    pub fn new(seed: u64) -> Night {
        Night {
            show: Show::new(seed),
            loops: Vec::new(),
            last: String::new(),
            taught_off: false,
            ended: false,
        }
    }

    fn say(&mut self, ctx: &mut Ctx<Parlor>, text: String) {
        ctx.say(text.clone());
        self.last = text;
    }

    fn out_words(n: usize) -> String {
        match n {
            0 => "all in step".to_string(),
            1 => "1 out of step".to_string(),
            n => format!("{n} out of step"),
        }
    }

    /// Round, stragglers, harmony: the status line.
    pub fn status(&self) -> String {
        let s = &self.show;
        format!(
            "Round {} of {ROUNDS}, {}, {} percent in step.",
            s.round() + 1,
            Self::out_words(s.stragglers()),
            s.harmony()
        )
    }

    fn controls(&self, ctx: &Ctx<Parlor>) -> String {
        let p = ctx.control_name(ParlorAction::Primary);
        let l = ctx.control_name(ParlorAction::Left);
        let r = ctx.control_name(ParlorAction::Right);
        let s = ctx.control_name(ParlorAction::Status);
        let h = ctx.control_name(ParlorAction::Hint);
        format!(
            "{} with each flash keeps the fireflies in step. {} and {r} listen to each side of the fire. {s} for status, {h} for the rhythm. Escape pauses.",
            sentence_start(&p),
            sentence_start(&l)
        )
    }

    fn start_sounds(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        ctx.audio.set_bed(None);
        ctx.audio.set_listener(SEAT, 0.0);
        let bed = ctx.audio.start_loop(sound::BED_SUMMER, None, Bus::AMBIENCE);
        ctx.audio.set_volume(bed, 0.35);
        let fire = ctx
            .audio
            .start_loop(sound::BONFIRE, Some(Vec3::ZERO), Bus::AMBIENCE);
        ctx.audio.set_volume(fire, 0.6);
        self.loops = vec![bed, fire];
    }

    fn stop_sounds(&mut self, ctx: &mut Ctx<Parlor>) {
        for s in self.loops.drain(..) {
            ctx.audio.stop(s, 600);
        }
    }

    fn play_notes(&mut self, ctx: &mut Ctx<Parlor>) {
        for note in self.show.drain_notes() {
            match note {
                ShowNote::Round { round, .. } => {
                    let out = Self::out_words(self.show.stragglers());
                    let text = if round == 0 {
                        format!("The show begins. {}.", sentence_start(&out))
                    } else {
                        format!("Round {}. {}.", round + 1, sentence_start(&out))
                    };
                    self.say(ctx, text);
                }
                ShowNote::Flash { in_step } => {
                    let s = sound::play(ctx, sound::CHORUS);
                    ctx.audio
                        .set_volume(s, 0.35 + 0.65 * in_step as f32 / SWARM as f32);
                }
                ShowNote::Straggle { pos } => {
                    let s = ctx.audio.play_at(
                        sound::glow(Species::MidsummerChorus),
                        v3(pos),
                        Bus::EFFECTS,
                    );
                    ctx.audio.set_volume(s, 0.7);
                }
                ShowNote::Over => self.finish(ctx),
            }
        }
    }

    fn press(&mut self, ctx: &mut Ctx<Parlor>) {
        match self.show.press() {
            Press::OnBeat { rejoined, left } => {
                sound::play(ctx, sound::IN_STEP);
                if rejoined && left == 0 {
                    self.say(ctx, "All in step.".to_string());
                }
            }
            Press::OffBeat => {
                sound::play(ctx, sound::OFF_BEAT);
                if !self.taught_off {
                    self.taught_off = true;
                    self.say(
                        ctx,
                        "Off the beat. Press with the chorus flash, not between.".to_string(),
                    );
                }
            }
            Press::Already => {
                sound::play(ctx, sound::OFF_BEAT);
            }
            Press::NotYet => {
                sound::play(ctx, sound::OFF_BEAT);
                self.say(ctx, "The show starts in a moment.".to_string());
            }
            Press::Over => {}
        }
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.ended {
            return;
        }
        self.ended = true;
        self.stop_sounds(ctx);
        ctx.audio.play_music(Some(sound::KEEPSAKE_MUSIC), false);
        let year = today(ctx).year;
        let name = almanac::keepsake(year);
        let harmony = self.show.harmony();
        progress::note_catch(ctx, Species::MidsummerChorus);
        let kept = progress::note_keepsake(ctx, year, name, harmony);
        let pages = result_pages(year, harmony, self.show.rejoined, &kept);
        let summary = format!("Midsummer night. {harmony} percent in step");
        let mut results = Some(after_show(summary, pages.clone()));
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(results);
            }
        }));
    }
}

/// The pages after the show.
pub fn result_pages(year: i32, harmony: u32, rejoined: u32, kept: &KeptSake) -> Vec<String> {
    let brought = match rejoined {
        1 => "You brought 1 straggler back into step.".to_string(),
        n => format!("You brought {n} stragglers back into step."),
    };
    let first = format!(
        "The last flash fades, and the bonfire settles to embers. The meadow was {harmony} percent in step. {brought}"
    );
    let second = if kept.new {
        format!(
            "Your {year} keepsake: {}. It goes in your journal, with tonight's {harmony} percent.",
            kept.name
        )
    } else {
        let best = kept.previous.unwrap_or(0).max(harmony);
        format!(
            "You already hold the {year} keepsake, {}. The year's best is {best} percent in step.",
            kept.name
        )
    };
    vec![first, second]
}

/// The results menu after the show.
pub fn after_show(summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
        labels::PLAY_AGAIN,
        "The show again. The year's keepsake keeps your best.",
        move |ctx| start(ctx, true),
    )
}

impl State<Parlor> for Night {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        NIGHT_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        self.start_sounds(ctx);
        self.say(
            ctx,
            "Midsummer night. The fireflies gather round the fire.".to_string(),
        );
        let c = self.controls(ctx);
        ctx.say_queued(c);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_sounds(ctx);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.ended {
            return;
        }
        if self.loops.is_empty() {
            self.start_sounds(ctx);
        }
        let s = self.status();
        ctx.say(format!("Back by the fire. {s}"));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.ended || event.is_repeat() {
            return;
        }
        let action = ctx.action_for(event);
        if action.is_none() && event.pressed() == Some(Key::Escape) {
            menus::pause(ctx, HELP, Leave::Title(TITLE_NAME));
            return;
        }
        match action {
            Some(ParlorAction::Primary) => self.press(ctx),
            Some(ParlorAction::Left | ParlorAction::Right) => {
                let left = action == Some(ParlorAction::Left);
                let n = self.show.stragglers_on(left);
                let side = if left { "left" } else { "right" };
                sound::play(ctx, sound::NEAR);
                self.say(ctx, format!("On your {side}, {}.", Self::out_words(n)));
            }
            Some(ParlorAction::Up) => {
                sound::play(ctx, sound::NEAR);
                self.say(
                    ctx,
                    "Overhead, sparks rise from the fire into the short summer night.".to_string(),
                );
            }
            Some(ParlorAction::Down) => {
                sound::play(ctx, sound::STEP_GRASS);
                self.say(ctx, "You sit in the warm grass by the fire.".to_string());
            }
            Some(ParlorAction::Secondary) => {
                sound::play(ctx, sound::FULL);
                self.say(
                    ctx,
                    "The jar stays shut tonight. These fireflies belong to the chorus.".to_string(),
                );
            }
            Some(ParlorAction::Status) => {
                let s = self.status();
                ctx.say(s);
            }
            Some(ParlorAction::Repeat) => {
                if self.last.is_empty() {
                    let s = self.status();
                    ctx.say(s);
                } else {
                    ctx.say(self.last.clone());
                }
            }
            Some(ParlorAction::Hint) => {
                sound::play(ctx, sound::CHORUS);
                let dark = ROUND_BEATS - FLASHES;
                self.say(
                    ctx,
                    format!(
                        "The chorus flashes {FLASHES} times, a little over half a second apart, then rests for {dark} beats. Press with each flash you hear."
                    ),
                );
            }
            None => {
                if let InputEvent::KeyDown { key, .. } = event {
                    if !matches!(
                        key,
                        Key::LCtrl
                            | Key::RCtrl
                            | Key::LShift
                            | Key::RShift
                            | Key::LAlt
                            | Key::RAlt
                            | Key::Insert
                            | Key::Other(_)
                    ) {
                        sound::play(ctx, sound::NOTHING);
                        ctx.say("Nothing happens. F1 lists the controls.");
                    }
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.ended {
            return;
        }
        self.show.update(dt);
        self.play_notes(ctx);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        vec!["Firefly Meadow: midsummer night".to_string(), self.status()]
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        format!(
            "{} The whole meadow flashes together. A late flash is a straggler: press {} with the chorus to bring it back. {}",
            self.status(),
            ctx.control_name(ParlorAction::Primary),
            self.controls(ctx)
        )
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

mod visuals_night;
