//! Plays shows back for listening: the target beside the player's own, or a
//! freestyle show for the crowd.

use hearken::{Bus, Ctx, InputEvent, Key, State};
use parlor_core::{Parlor, ParlorAction};

use crate::rules::{Cue, Launch, Phase, Round};
use crate::sound;

/// Seconds between a part's spoken name and its count-in.
const LEAD: f64 = 1.5;

struct Part {
    name: String,
    show: Vec<Launch>,
}

pub struct Replay {
    parts: Vec<Part>,
    at: usize,
    lead: f64,
    round: Option<Round>,
    /// The crowd cheers once the last part is over.
    cheer: bool,
}

impl Replay {
    /// The show the player was asked for, then the one they launched.
    pub fn compare(show: Vec<Launch>, launches: Vec<Launch>) -> Replay {
        Replay {
            parts: vec![
                Part {
                    name: "The show.".to_string(),
                    show,
                },
                Part {
                    name: if launches.is_empty() {
                        "Yours: you launched nothing.".to_string()
                    } else {
                        "Yours.".to_string()
                    },
                    show: launches,
                },
            ],
            at: 0,
            lead: LEAD,
            round: None,
            cheer: false,
        }
    }

    /// A freestyle show, with the crowd's cheer at the end.
    pub fn show(name: &str, show: Vec<Launch>) -> Replay {
        Replay {
            parts: vec![Part {
                name: name.to_string(),
                show,
            }],
            at: 0,
            lead: LEAD,
            round: None,
            cheer: true,
        }
    }

    fn announce(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Some(part) = self.parts.get(self.at) {
            ctx.say(part.name.clone());
        }
        self.lead = LEAD;
        self.round = None;
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.cheer {
            sound::react(ctx, 100);
        }
        ctx.pop_with("That's the show.");
    }
}

impl State<Parlor> for Replay {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        "liberty_lights_replay"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::night(ctx);
        self.announce(ctx);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        self.announce(ctx);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() || (event.pressed().is_none() && ctx.action_for(event).is_none()) {
            return;
        }
        if event.pressed() == Some(Key::Escape) {
            ctx.pop_with("Stopped.");
            return;
        }
        match ctx.action_for(event) {
            Some(ParlorAction::Repeat) => {
                self.announce(ctx);
            }
            Some(ParlorAction::Status) => {
                let n = self.parts.get(self.at).map_or(0, |p| p.show.len());
                let fireworks = if n == 1 {
                    "1 firework".to_string()
                } else {
                    format!("{n} fireworks")
                };
                ctx.say(format!(
                    "Part {} of {}, {fireworks}. Escape stops.",
                    self.at + 1,
                    self.parts.len()
                ));
            }
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let Some(part) = self.parts.get(self.at) else {
            return;
        };
        if self.round.is_none() {
            self.lead -= dt;
            if self.lead > 0.0 {
                return;
            }
            if part.show.is_empty() {
                self.at += 1;
                if self.at >= self.parts.len() {
                    self.finish(ctx);
                } else {
                    self.announce(ctx);
                }
                return;
            }
            let mut round = Round::new(part.show.clone());
            round.listen();
            self.round = Some(round);
        }
        let Some(round) = self.round.as_mut() else {
            return;
        };
        let cues = round.update(dt);
        let done = round.phase == Phase::YourTurn;
        for cue in cues {
            match cue {
                Cue::Tick(_) => {
                    ctx.audio.play(sound::TICK, Bus::EFFECTS);
                }
                Cue::Go => {
                    ctx.audio.play(sound::GO, Bus::EFFECTS);
                }
                Cue::Burst(l) => sound::launch(ctx, l),
                Cue::TurnBegins => {}
            }
        }
        if done {
            self.at += 1;
            if self.at >= self.parts.len() {
                self.finish(ctx);
            } else {
                self.announce(ctx);
            }
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        self.parts
            .get(self.at)
            .map(|p| p.name.clone())
            .into_iter()
            .collect()
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        format!(
            "A replay: each part is named, counted in, then played. {} starts the part again from its name, {} says where the replay is, Escape stops.",
            ctx.control_name(ParlorAction::Repeat),
            ctx.control_name(ParlorAction::Status)
        )
    }
}

mod visuals;
