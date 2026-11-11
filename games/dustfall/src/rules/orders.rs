//! Orders: one risk a night, taken in the colony phase, each a gain now
//! for a cost in the night to come.

use serde::{Deserialize, Serialize};

use parlor_core::Rng;

/// Shield Overclock puts up at once.
pub const OVERCLOCK_SHIELD: u32 = 20;
/// Power Overclock costs after the night.
pub const OVERCLOCK_POWER: u32 = 4;
/// Food Rationing brings in.
pub const RATIONING_FOOD: u32 = 4;
/// Orders offered each night.
pub const OFFERED: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Order {
    /// 20 shield now, 4 power less after the night.
    Overclock,
    /// 4 food now, and tonight's raiders arrive closer together.
    Rationing,
    /// Twice the metal from kills tonight, and the skitters are faster.
    Salvage,
}

impl Order {
    pub const ALL: [Order; 3] = [Order::Overclock, Order::Rationing, Order::Salvage];

    pub fn name(self) -> &'static str {
        match self {
            Order::Overclock => "Overclock",
            Order::Rationing => "Rationing",
            Order::Salvage => "Salvage",
        }
    }

    /// What it gives, then what it costs, as the row reads it.
    pub fn terms(self) -> String {
        match self {
            Order::Overclock => format!(
                "{OVERCLOCK_SHIELD} shield now, {OVERCLOCK_POWER} power less after the night"
            ),
            Order::Rationing => {
                format!("{RATIONING_FOOD} food now, and tonight's raiders come closer together")
            }
            Order::Salvage => {
                "twice the metal from kills tonight, and the skitters come faster".to_string()
            }
        }
    }

    /// The two orders offered before `night`, the same for the same seed.
    pub fn offered(seed: u64, night: u8) -> Vec<Order> {
        let mut rng = Rng::seeded(super::mix(seed, night, 2));
        let mut pool = Order::ALL.to_vec();
        rng.shuffle(&mut pool);
        pool.truncate(OFFERED);
        pool.sort_by_key(|o| Order::ALL.iter().position(|a| a == o));
        pool
    }
}
