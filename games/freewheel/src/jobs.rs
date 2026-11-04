//! The job board: courier deliveries across the districts you own, each a
//! timed ride paid in dollars. Full pay on time, half pay late.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::menus::labels;
use parlor_core::{Parlor, Rng};

use crate::course::{Course, District, Mix};
use crate::progress::{dollar_words, span_words, Choices, Progress};
use crate::ride::Ride;

/// Jobs on the board at a time.
pub const JOBS: usize = 3;
/// The pace a deadline allows, metres a second: about 22 kilometres an hour.
pub const DELIVERY_PACE: f64 = 6.0;
/// Dollars a job pays per metre, before the district's rate.
pub const PAY_PER_M: f64 = 1.0 / 20.0;

/// Where parcels go from and to.
const STOPS: [&str; 10] = [
    "the bakery",
    "the library",
    "the post office",
    "the station",
    "a flower stall",
    "the town hall",
    "a phone repair shop",
    "the dentist",
    "a corner café",
    "the music school",
];

/// One delivery on the board.
#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    pub district: District,
    pub length: f64,
    pub seed: u64,
    pub from: &'static str,
    pub to: &'static str,
}

impl Job {
    /// The board's jobs, the same until the next delivery is made, in the
    /// districts the rider owns.
    pub fn board(progress: &Progress) -> Vec<Job> {
        let mut rng =
            Rng::seeded(0x6a6f_6273 ^ (progress.deliveries as u64).wrapping_mul(0x9e37_79b9));
        let districts = progress.my_districts();
        (0..JOBS)
            .map(|_| {
                let district = districts[rng.below(districts.len() as u32) as usize];
                let length = 500.0 + 100.0 * rng.below(11) as f64;
                let from = rng.below(STOPS.len() as u32) as usize;
                let to = (from + 1 + rng.below(STOPS.len() as u32 - 1) as usize) % STOPS.len();
                Job {
                    district,
                    length,
                    seed: rng.next_u64(),
                    from: STOPS[from],
                    to: STOPS[to],
                }
            })
            .collect()
    }

    /// Seconds allowed for full pay.
    pub fn deadline(&self) -> f64 {
        (self.length / DELIVERY_PACE).round()
    }

    /// Full pay: 1 dollar for every 20 metres, more in harder districts.
    pub fn pay(&self) -> u32 {
        (self.length * PAY_PER_M * pay_rate(self.district)).round() as u32
    }

    /// What a delivery in `secs` earns: full pay on time, half late.
    pub fn paid(&self, secs: f64) -> u32 {
        if secs <= self.deadline() {
            self.pay()
        } else {
            self.pay() / 2
        }
    }

    /// The route: the district's streets with one junction turn.
    pub fn course(&self) -> Course {
        let mix = Mix {
            junctions: 1,
            ..self.district.mix()
        };
        Course {
            surface: self.district.surface(),
            ..Course::generate(&self.title(), self.seed, self.length, mix)
        }
    }

    /// "The bakery to the library".
    pub fn title(&self) -> String {
        sentence_start(&format!("{} to {}", self.from, self.to))
    }
}

/// How much a district's jobs pay, against Downtown's.
pub fn pay_rate(district: District) -> f64 {
    match district {
        District::Downtown => 1.0,
        District::Riverside => 1.25,
        District::OldTown => 1.5,
    }
}

/// Pick a delivery.
pub struct JobBoard {
    menu: MenuCore<Self, Parlor>,
}

impl JobBoard {
    pub fn new() -> Self {
        JobBoard {
            menu: MenuCore::new("Job board").with_intro_help(
                "Up and Down choose a delivery, Enter rides it, Escape goes back.",
            ),
        }
    }
}

impl Default for JobBoard {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for JobBoard {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "freewheel_jobs"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let progress = Progress::load(ctx);
        let bike = Choices::load(ctx).bike;
        let mut items: Vec<MenuItem<Self, Parlor>> = Job::board(&progress)
            .into_iter()
            .map(|job| {
                let label = format!(
                    "{}, {}, {}, {}",
                    job.title(),
                    job.district.name(),
                    parlor_core::units::route(job.length),
                    dollar_words(job.pay())
                );
                let help = format!(
                    "Deliver within {} for full pay. Late pays half.",
                    span_words(job.deadline())
                );
                MenuItem::new(label, move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.replace(Ride::delivery(job.clone(), bike))
                })
                .help(help)
            })
            .collect();
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Freewheel title."),
        );
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_job_pays_by_its_formula() {
        let mut progress = Progress::default();
        for n in 0..20 {
            progress.deliveries = n;
            for job in Job::board(&progress) {
                assert_eq!(job.district, District::Downtown, "only owned districts");
                let want = (job.length / 20.0).round() as u32;
                assert_eq!(job.pay(), want);
                assert_eq!(job.paid(job.deadline()), want);
                assert_eq!(job.paid(job.deadline() + 1.0), want / 2);
                assert_ne!(job.from, job.to);
                assert!((500.0..=1500.0).contains(&job.length));
            }
        }
        let job = Job {
            district: District::OldTown,
            length: 1000.0,
            seed: 1,
            from: STOPS[0],
            to: STOPS[1],
        };
        assert_eq!(job.pay(), 75);
        assert_eq!(job.deadline(), 167.0);
        assert_eq!(job.course().surface, crate::bike::Surface::Cobbles);
        assert_eq!(job.course().junctions.len(), 1);
    }

    #[test]
    fn the_board_changes_after_each_delivery() {
        let mut progress = Progress::default();
        let a = Job::board(&progress);
        assert_eq!(a, Job::board(&progress));
        progress.deliveries = 1;
        assert_ne!(a, Job::board(&progress));
    }
}
