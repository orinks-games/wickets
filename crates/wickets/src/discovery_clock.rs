//! The internet date that decides whether a holiday special the player
//! has never played can be found (see `games::discovery_date`).
//!
//! One HTTPS request reads a web server's Date header. That time is then
//! moved on only by the monotonic clock ([`Instant`]), so changing the
//! computer's date after the check does not change it, and nothing is
//! saved, so a restart checks again. This raises the bar for opening a
//! special early; it is not tamper-proof (a player who controls the
//! network or the process can still lie to it). Games already played, and
//! each game's own dated nights and finales, do not use it.
use std::cell::Cell;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use parlor_core::challenges::Date;

/// How long one answer counts before the date must be checked again.
pub const MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
const REQUEST_LIMIT: Duration = Duration::from_secs(8);
/// Asked in turn until one gives a usable Date header.
const PROVIDERS: [&str; 2] = ["https://www.cloudflare.com/", "https://www.google.com/"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Checking,
    Ready(Date),
    Unavailable,
}

/// A server UTC timestamp advanced only by monotonic elapsed time.
#[derive(Clone, Copy, Debug)]
pub struct Anchor {
    unix_seconds: i64,
}

impl Anchor {
    pub fn from_http_date(value: &str) -> Option<Self> {
        parse_http_date(value).map(|unix_seconds| Self { unix_seconds })
    }

    /// An expired anchor cannot authorize a new discovery.
    pub fn date_after(self, elapsed: Duration) -> Option<Date> {
        if elapsed >= MAX_AGE {
            return None;
        }
        let seconds = self.unix_seconds.checked_add(elapsed.as_secs() as i64)?;
        Some(Date::from_days(seconds.div_euclid(86_400)))
    }

    /// [`Anchor::date_after`], unless the computer's clock has moved a
    /// different amount (`wall_elapsed`, `None` if it went backwards) than
    /// the monotonic one since the answer. A changed clock, or a sleep the
    /// monotonic clock did not count, then needs a new check instead of
    /// moving the date.
    pub fn date_after_checked(
        self,
        elapsed: Duration,
        wall_elapsed: Option<Duration>,
    ) -> Option<Date> {
        let wall = wall_elapsed?;
        let drift = wall.abs_diff(elapsed);
        if drift > MAX_DRIFT {
            return None;
        }
        self.date_after(elapsed)
    }
}

/// How far the computer's clock may move apart from the monotonic one
/// before an answer is checked again.
pub const MAX_DRIFT: Duration = Duration::from_secs(10 * 60);

#[derive(Default)]
struct Clock {
    anchor: Option<(Anchor, Instant, SystemTime)>,
    checking: Option<Instant>,
    generation: u64,
}

fn clock() -> &'static Mutex<Clock> {
    static CLOCK: OnceLock<Mutex<Clock>> = OnceLock::new();
    CLOCK.get_or_init(|| Mutex::new(Clock::default()))
}

fn current(clock: &Clock) -> Status {
    if let Some((anchor, received, wall)) = clock.anchor {
        let wall_elapsed = SystemTime::now().duration_since(wall).ok();
        if let Some(date) = anchor.date_after_checked(received.elapsed(), wall_elapsed) {
            return Status::Ready(date);
        }
    }
    if clock
        .checking
        .is_some_and(|started| started.elapsed() < REQUEST_LIMIT)
    {
        Status::Checking
    } else {
        Status::Unavailable
    }
}

thread_local! {
    /// A status a test sets for its own thread instead of the network.
    static SIMULATED: Cell<Option<Status>> = const { Cell::new(None) };
    /// When a play test's "offline" preview was last asked to check, so it
    /// spends a moment checking before failing, like a real request.
    static PREVIEW_ASKED: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// Makes [`status`] answer `status` on this thread, and [`request`] act on
/// it without the network, until called with `None`. For tests.
#[doc(hidden)]
pub fn simulate(status: Option<Status>) {
    SIMULATED.with(|s| s.set(status));
}

/// `WICKETS_NETWORK_DATE`, in debug builds only, for play tests: a date
/// ("2026-11-10") the network is taken to say, or "offline".
fn preview() -> Option<Status> {
    if !cfg!(debug_assertions) {
        return None;
    }
    let text = std::env::var("WICKETS_NETWORK_DATE").ok()?;
    if text.trim() == "offline" {
        let checking = PREVIEW_ASKED
            .with(Cell::get)
            .is_some_and(|asked| asked.elapsed() < Duration::from_secs(1));
        return Some(if checking {
            Status::Checking
        } else {
            Status::Unavailable
        });
    }
    Date::parse(&text).map(Status::Ready)
}

/// Never blocks on network IO and never falls back to the computer's date.
pub fn status() -> Status {
    if let Some(status) = SIMULATED.with(Cell::get).or_else(preview) {
        return status;
    }
    clock()
        .lock()
        .map_or(Status::Unavailable, |clock| current(&clock))
}

/// Start or explicitly retry a check. Repeated requests while checking or
/// while an anchor is valid are harmless. The UI remains usable throughout.
pub fn request() {
    if let Some(status) = SIMULATED.with(Cell::get) {
        if status == Status::Unavailable {
            simulate(Some(Status::Checking));
        }
        return;
    }
    if let Some(status) = preview() {
        if status == Status::Unavailable {
            PREVIEW_ASKED.with(|asked| asked.set(Some(Instant::now())));
        }
        return;
    }
    let Ok(mut state) = clock().lock() else {
        return;
    };
    if current(&state) != Status::Unavailable {
        return;
    }
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    let started = Instant::now();
    state.checking = Some(started);
    drop(state);
    let worker = std::thread::Builder::new()
        .name("seasonal-date".into())
        .spawn(move || {
            let result = fetch_anchor();
            let Ok(mut state) = clock().lock() else {
                return;
            };
            // A late response, including a slow DNS lookup, never authorizes
            // discovery or overwrites a newer explicit retry.
            if state.generation == generation {
                state.checking = None;
                if started.elapsed() < REQUEST_LIMIT {
                    state.anchor =
                        result.map(|anchor| (anchor, Instant::now(), SystemTime::now()));
                }
            }
        });
    if worker.is_err() {
        if let Ok(mut state) = clock().lock() {
            if state.generation == generation {
                state.checking = None;
            }
        }
    }
}

fn fetch_anchor() -> Option<Anchor> {
    let agent = ureq::AgentBuilder::new()
        .https_only(true)
        .redirects(0)
        .try_proxy_from_env(true)
        .timeout(Duration::from_secs(3))
        .timeout_connect(Duration::from_secs(2))
        .build();
    PROVIDERS.iter().find_map(|url| ask(&agent, url))
}

fn ask(agent: &ureq::Agent, url: &str) -> Option<Anchor> {
    let response = agent
        .head(url)
        .set("Cache-Control", "no-cache, no-store")
        .call()
        .ok()?;
    if response.status() != 200 {
        return None;
    }
    // Do not anchor to a timestamp from a cached response.
    if let Some(age) = response.header("Age") {
        if age.parse::<u64>().ok()? != 0 {
            return None;
        }
    }
    Anchor::from_http_date(response.header("Date")?)
}

/// Strict IMF-fixdate, the HTTP format sent by the fixed HTTPS provider.
/// Reject malformed, impossible and inconsistent dates instead of guessing.
fn parse_http_date(value: &str) -> Option<i64> {
    let b = value.as_bytes();
    if b.len() != 29
        || !value.is_ascii()
        || b[3..5] != *b", "
        || b[7] != b' '
        || b[11] != b' '
        || b[16] != b' '
        || b[19] != b':'
        || b[22] != b':'
        || &b[25..] != b" GMT"
    {
        return None;
    }
    let weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let weekday = weekdays.iter().position(|&s| s == &value[..3])? as u32;
    let month = months.iter().position(|&s| s == &value[8..11])? as u32 + 1;
    let number = |range: std::ops::Range<usize>| -> Option<u32> {
        let part = &value[range];
        part.bytes().all(|b| b.is_ascii_digit()).then_some(())?;
        part.parse().ok()
    };
    let year = number(12..16)? as i32;
    let day = number(5..7)?;
    let hour = number(17..19)?;
    let minute = number(20..22)?;
    let second = number(23..25)?;
    let date = Date::new(year, month, day);
    if year < 1970
        || Date::from_days(date.days()) != date
        || date.weekday() != weekday
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    Some(date.days() * 86_400 + i64::from(hour * 3600 + minute * 60 + second))
}
