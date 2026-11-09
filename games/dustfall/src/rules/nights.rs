//! The twelve nights: what each one sends down. The first four teach one
//! thing each; night 5 and night 10 bring a mothership; night 12 is the
//! carrier.

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

/// The raiders of a night, in arrival order. A skitter pair is two
/// entries.
pub fn wave(night: u8) -> Vec<Kind> {
    use Kind::*;
    match night {
        0 | 1 => vec![Drifter, Drifter],
        2 => vec![Drifter, Drifter, Drifter],
        3 => vec![Drifter, Drifter, Hulk],
        4 => vec![Drifter, Drifter, Skitter, Skitter, Mule],
        5 => vec![Mothership, Drifter, Drifter, Skitter, Skitter],
        6 => vec![Drifter, Drifter, Hulk, Sapper],
        7 => vec![Drifter, Drifter, Drifter, Skitter, Skitter, Sapper, Mule],
        8 => vec![Hulk, Hulk, Drifter, Drifter, Skitter, Skitter],
        9 => vec![
            Drifter, Drifter, Drifter, Skitter, Skitter, Skitter, Skitter, Hulk, Sapper,
        ],
        10 => vec![Mothership, Hulk, Drifter, Drifter, Skitter, Skitter, Sapper],
        11 => vec![
            Hulk, Hulk, Sapper, Sapper, Skitter, Skitter, Skitter, Skitter, Drifter, Drifter, Mule,
        ],
        12 => vec![Carrier, Mothership, Mothership, Drifter, Drifter],
        n => long_watch(n),
    }
}

/// The long watch past Relief: nights 9 to 12 round again, each round
/// with another drifter.
fn long_watch(night: u8) -> Vec<Kind> {
    let past = u32::from(night - RELIEF_NIGHT);
    let base = RELIEF_NIGHT - 3 + ((past - 1) % 4) as u8;
    let mut kinds = wave(base);
    kinds.extend(std::iter::repeat_n(
        Kind::Drifter,
        past.div_ceil(4) as usize,
    ));
    kinds
}

/// Whether the night's drifters hold their fire.
pub fn docile(night: u8) -> bool {
    night <= DOCILE_UNTIL
}

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
