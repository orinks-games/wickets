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

    /// "Daily challenge", "Weekly challenge", "Monthly challenge": whole
    /// names, so a translation never joins the period to the word.
    pub fn challenge_title(self) -> &'static str {
        match self {
            Period::Daily => "Daily challenge",
            Period::Weekly => "Weekly challenge",
            Period::Monthly => "Monthly challenge",
        }
    }

    /// "Daily challenges", "Weekly challenges", "Monthly challenges".
    pub fn challenges_title(self) -> &'static str {
        match self {
            Period::Daily => "Daily challenges",
            Period::Weekly => "Weekly challenges",
            Period::Monthly => "Monthly challenges",
        }
    }

    /// "a 3 day streak", as a menu row ends with it.
    pub fn streak_phrase(self, n: impl std::fmt::Display) -> String {
        let n = n.to_string();
        let an = n.starts_with('8')
            || ((n.len() == 2 || n.len() == 5) && (n.starts_with("11") || n.starts_with("18")));
        match (self, an) {
            (Period::Daily, false) => format!("a {n} day streak"),
            (Period::Daily, true) => format!("an {n} day streak"),
            (Period::Weekly, false) => format!("a {n} week streak"),
            (Period::Weekly, true) => format!("an {n} week streak"),
            (Period::Monthly, false) => format!("a {n} month streak"),
            (Period::Monthly, true) => format!("an {n} month streak"),
        }
    }

    /// "A 3 day streak.", "An 8 week streak.": a whole sentence, so a
    /// translation never joins the period's word into it.
    pub fn streak_sentence(self, n: impl std::fmt::Display) -> String {
        let n = n.to_string();
        let an = n.starts_with('8')
            || ((n.len() == 2 || n.len() == 5) && (n.starts_with("11") || n.starts_with("18")));
        match (self, an) {
            (Period::Daily, false) => format!("A {n} day streak."),
            (Period::Daily, true) => format!("An {n} day streak."),
            (Period::Weekly, false) => format!("A {n} week streak."),
            (Period::Weekly, true) => format!("An {n} week streak."),
            (Period::Monthly, false) => format!("A {n} month streak."),
            (Period::Monthly, true) => format!("An {n} month streak."),
        }
    }

    /// "Daily streak: 3 days.", "Weekly streak: 1 week.", whole.
    pub fn streak_count(self, n: u32) -> String {
        match (self, n) {
            (Period::Daily, 1) => "Daily streak: 1 day.".to_string(),
            (Period::Daily, n) => format!("Daily streak: {n} days."),
            (Period::Weekly, 1) => "Weekly streak: 1 week.".to_string(),
            (Period::Weekly, n) => format!("Weekly streak: {n} weeks."),
            (Period::Monthly, 1) => "Monthly streak: 1 month.".to_string(),
            (Period::Monthly, n) => format!("Monthly streak: {n} months."),
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
