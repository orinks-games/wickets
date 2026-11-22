//! The run's song: each sector after a break opens the next section, and
//! the corridor quickens as it goes.
use neon_rail::rules::{Loadout, Mode, Run};
use neon_rail::screens::{song_plan, song_speed};
use parlor_core::Rng;

#[test]
fn each_sector_after_a_break_opens_the_next_section() {
    let mut run = Run::roguelite(Mode::BlackSky, Loadout::default(), 0, Rng::seeded(2));
    assert_eq!(song_plan(&run).unwrap().section, 0);
    // Back from the route menu: the cleared sector is still current.
    run.sectors_cleared = 1;
    run.current_sector = Some(0);
    assert_eq!(song_plan(&run).unwrap().section, 2);
    run.sectors_cleared = 2;
    run.current_sector = Some(1);
    assert_eq!(song_plan(&run).unwrap().section, 3);
}

#[test]
fn the_corridor_quickens_each_sector_and_other_runs_keep_their_speed() {
    let mut run = Run::challenge(Loadout::default(), 0);
    assert_eq!(song_speed(&run), 1.0);
    run.current_sector = Some(3);
    assert!((song_speed(&run) - 1.096).abs() < 1e-4);
    run.current_sector = Some(5);
    let top = song_speed(&run);
    run.current_sector = Some(9);
    assert_eq!(song_speed(&run), top);
    let other = Run::roguelite(Mode::Endless, Loadout::default(), 0, Rng::seeded(4));
    assert_eq!(song_speed(&other), 1.0);
    assert!(song_plan(&Run::tutorial(Loadout::default(), 0)).is_none());
}
