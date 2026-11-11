//! The twelve nights: the waves each one sends down, and how they are
//! timed. The first four teach one thing each; night 5 and night 10 bring
//! a mothership; night 12 is the carrier.

use super::lanes::ticks;
use super::raiders::Kind;

/// Nights to hold before Relief lands.
pub const RELIEF_NIGHT: u8 = 12;
/// Nights whose drifters never fire, so the gun is learned before the
/// dodge.
pub const DOCILE_UNTIL: u8 = 1;
/// The first night raiders may come down with a quirk.
pub const QUIRKS_FROM: u8 = 7;

/// How many of a night's raiders may carry a quirk: none before night 7,
/// one on nights 7 and 8, two from night 9.
pub fn quirks_at_most(night: u8) -> usize {
    match night {
        n if n < QUIRKS_FROM => 0,
        n if n < QUIRKS_FROM + 2 => 1,
        _ => 2,
    }
}

/// The waves of a night, in order, each the raiders in it in arrival
/// order. A skitter pair is two entries.
pub fn waves(night: u8) -> Vec<Vec<Kind>> {
    use Kind::*;
    match night {
        0 | 1 => vec![vec![Drifter], vec![Drifter]],
        2 => vec![vec![Drifter, Drifter], vec![Drifter]],
        3 => vec![vec![Drifter, Drifter], vec![Hulk]],
        4 => vec![vec![Drifter, Drifter], vec![Skitter, Skitter], vec![Mule]],
        5 => vec![
            vec![Drifter, Drifter],
            vec![Skitter, Skitter],
            vec![Mothership],
        ],
        6 => vec![vec![Drifter, Drifter], vec![Hulk, Sapper]],
        7 => vec![
            vec![Drifter, Drifter, Drifter],
            vec![Skitter, Skitter, Sapper],
            vec![Mule],
        ],
        8 => vec![vec![Hulk, Drifter, Drifter], vec![Hulk, Skitter, Skitter]],
        9 => vec![
            vec![Drifter, Drifter, Drifter],
            vec![Skitter, Skitter, Skitter, Skitter],
            vec![Hulk, Sapper],
        ],
        10 => vec![
            vec![Hulk, Drifter, Drifter],
            vec![Skitter, Skitter, Sapper],
            vec![Mothership],
        ],
        11 => vec![
            vec![Hulk, Sapper, Drifter, Drifter],
            vec![Skitter, Skitter, Skitter, Skitter],
            vec![Hulk, Sapper, Mule],
        ],
        12 => vec![
            vec![Drifter, Drifter, Mothership],
            vec![Mothership],
            vec![Carrier],
        ],
        n => long_watch(n),
    }
}

/// The raiders of a night, every wave together.
pub fn kinds(night: u8) -> Vec<Kind> {
    waves(night).into_iter().flatten().collect()
}

/// The long watch past Relief: nights 9 to 12 round again, each round
/// with another drifter in the first wave.
fn long_watch(night: u8) -> Vec<Vec<Kind>> {
    let past = u32::from(night - RELIEF_NIGHT);
    let base = RELIEF_NIGHT - 3 + ((past - 1) % 4) as u8;
    let mut waves = waves(base);
    waves[0].extend(std::iter::repeat_n(
        Kind::Drifter,
        past.div_ceil(4) as usize,
    ));
    waves
}

/// Whether the night's drifters hold their fire.
pub fn docile(night: u8) -> bool {
    night <= DOCILE_UNTIL
}

/// Ticks before the first wave is called.
pub const FIRST_WAVE: u32 = ticks(2);
/// Ticks between a wave's call and its first raider.
pub const WAVE_LEAD: u32 = ticks(1);
/// Ticks between one raider of a wave and the next; a skitter pair is
/// half that apart.
pub const WAVE_STAGGER: u32 = ticks(1);
/// Ticks the sky stays empty between one wave's last raider and the next
/// wave's call.
pub const WAVE_LULL: u32 = ticks(4);

/// "2 drifters, a skitter pair, a mule": the forecast's count of a wave.
pub fn describe(kinds: &[Kind]) -> String {
    let mut parts = Vec::new();
    for kind in Kind::ALL {
        let n = kinds.iter().filter(|k| **k == kind).count();
        if n == 0 {
            continue;
        }
        let part = match (kind, n) {
            (Kind::Skitter, 2) => "a skitter pair".to_string(),
            (Kind::Skitter, n) if n % 2 == 0 => format!("{} skitter pairs", n / 2),
            (k, 1) => format!("a {}", k.name()),
            (k, n) => format!("{n} {}s", k.name()),
        };
        parts.push(part);
    }
    parts.join(", ")
}
