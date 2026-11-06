use serde::{Deserialize, Serialize};

/// How long a challenge lasts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Period {
    Daily,
    Weekly,
    Monthly,
}

impl Period {
    pub const ALL: [Period; 3] = [Period::Daily, Period::Weekly, Period::Monthly];

    /// "Daily", "Weekly", "Monthly".
    pub fn label(self) -> &'static str {
        match self {
            Period::Daily => "Daily",
            Period::Weekly => "Weekly",
            Period::Monthly => "Monthly",
        }
    }

    /// "day", "week", "month", for sentences such as "a 3 day streak".
    pub fn unit(self) -> &'static str {
        match self {
            Period::Daily => "day",
            Period::Weekly => "week",
            Period::Monthly => "month",
        }
    }

    /// "daily", "weekly", "monthly": how seeds and saves name it.
    pub fn tag(self) -> &'static str {
        match self {
            Period::Daily => "daily",
            Period::Weekly => "weekly",
            Period::Monthly => "monthly",
        }
    }
}

