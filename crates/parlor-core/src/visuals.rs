//! Drawing helpers shared by the games. Coordinates are local to a snapshot;
//! none of these functions read records, advance a clock, or play a sound.
pub use hearken::visuals::{Scene, GOLD, PAPER, RED, TEAL};

pub fn stage(lines: Vec<String>, background: [u8; 3]) -> Scene {
    let mut scene = Scene::from_lines(lines);
    scene.rect(0., 0., 1000., 600., background);
    scene
}

pub fn lanes(scene: &mut Scene, count: usize) {
    for i in 0..=count {
        let x = 80. + i as f32 * 840. / count.max(1) as f32;
        scene.line((x, 80.), (x, 560.), 3., [100, 119, 133]);
    }
    scene.line((80., 500.), (920., 500.), 5., PAPER);
}

pub fn lane(index: usize, count: usize) -> f32 {
    80. + (index as f32 + 0.5) * 840. / count.max(1) as f32
}

pub fn marker(scene: &mut Scene, x: f32, y: f32, label: impl Into<String>, color: [u8; 3]) {
    scene.circle(x, y, 20., color);
    scene.label(x + 26., y + 8., label, PAPER);
}

pub fn player(scene: &mut Scene, x: f32, y: f32) {
    scene.circle(x, y - 18., 12., PAPER);
    scene.line((x, y - 5.), (x, y + 22.), 7., PAPER);
    scene.line((x - 15., y + 6.), (x + 15., y + 6.), 5., PAPER);
    scene.line((x, y + 22.), (x - 12., y + 38.), 5., PAPER);
    scene.line((x, y + 22.), (x + 12., y + 38.), 5., PAPER);
}

pub fn gauge(scene: &mut Scene, y: f32, label: &str, fraction: f64, value: impl Into<String>) {
    scene.label(60., y, label, PAPER);
    scene.rect(60., y + 15., 880., 18., [64, 78, 90]);
    scene.rect(
        60.,
        y + 15.,
        880. * fraction.clamp(0., 1.) as f32,
        18.,
        TEAL,
    );
    scene.label(720., y, value, GOLD);
}

pub fn dial(scene: &mut Scene, value: f64, max: f64) {
    scene.circle(500., 290., 205., [80, 94, 103]);
    scene.circle(500., 290., 185., [23, 30, 40]);
    for i in 0..20 {
        let a = i as f32 * std::f32::consts::TAU / 20.;
        scene.line(
            (500. + a.sin() * 165., 290. - a.cos() * 165.),
            (500. + a.sin() * 180., 290. - a.cos() * 180.),
            3.,
            PAPER,
        );
    }
    let a = (value / max.max(1.) * std::f64::consts::TAU) as f32;
    scene.line(
        (500., 290.),
        (500. + a.sin() * 155., 290. - a.cos() * 155.),
        7.,
        GOLD,
    );
    scene.label(465., 550., format!("{value:.0}"), PAPER);
}

pub fn board(scene: &mut Scene, cols: usize, rows: usize) -> (f32, f32) {
    let cw = 880. / cols.max(1) as f32;
    let ch = 460. / rows.max(1) as f32;
    for col in 0..=cols {
        let x = 60. + col as f32 * cw;
        scene.line((x, 70.), (x, 530.), 2., [86, 105, 112]);
    }
    for row in 0..=rows {
        let y = 70. + row as f32 * ch;
        scene.line((60., y), (940., y), 2., [86, 105, 112]);
    }
    (cw, ch)
}

pub fn water(scene: &mut Scene, time: f64) {
    for row in 0..8 {
        let y = 80. + row as f32 * 60.;
        for col in 0..8 {
            let x = 30. + col as f32 * 125. + ((time * 0.5 + row as f64).sin() * 12.) as f32;
            scene.line((x, y), (x + 55., y), 2., [69, 112, 137]);
        }
    }
}
