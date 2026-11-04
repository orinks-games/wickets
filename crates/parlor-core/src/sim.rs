//! Fixed-step simulation under a variable frame time.
//!
//! A frame's `dt` changes with the machine and the display: 1/30 on a slow
//! laptop, 1/144 on a fast monitor. Physics that integrates with that `dt`
//! gives a different result on each. [`FixedStep`] collects frame time and
//! runs the simulation in equal substeps, carrying the leftover time to the
//! next frame, so the same play gives the same result at any frame rate.
//!
//! ```ignore
//! let mut clock = FixedStep::hz(120.0);
//! clock.advance(dt, |step| world.update(step));
//! ```

/// The longest frame the clock will catch up on. A longer stall (a
/// breakpoint, a suspended laptop) is cut to this, so the game does not
/// run hundreds of substeps at once and fall further behind.
pub const MAX_FRAME: f64 = 0.25;

/// How close to a whole step the stored time must come to count as one.
/// Summing 1/144 a hundred and forty-four times lands just short of 1.0 in
/// floating point; without this slack that frame would run one step fewer
/// and the frame rates would drift apart.
const SLACK: f64 = 1e-9;

/// Runs a simulation in equal substeps under a variable frame `dt`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedStep {
    step: f64,
    stored: f64,
    max_frame: f64,
    steps: u64,
}

impl FixedStep {
    /// A clock with substeps of `step` seconds. Panics unless `step` is
    /// positive and finite.
    pub fn new(step: f64) -> Self {
        assert!(
            step.is_finite() && step > 0.0,
            "FixedStep needs a positive step, got {step}"
        );
        FixedStep {
            step,
            stored: 0.0,
            max_frame: MAX_FRAME,
            steps: 0,
        }
    }

    /// A clock with `rate` substeps a second: `FixedStep::hz(120.0)`.
    pub fn hz(rate: f64) -> Self {
        FixedStep::new(1.0 / rate)
    }

    /// Changes the longest frame the clock catches up on (default
    /// [`MAX_FRAME`]). It is never less than one step.
    pub fn with_max_frame(mut self, seconds: f64) -> Self {
        self.max_frame = seconds.max(self.step);
        self
    }

    /// The substep length in seconds.
    pub fn step(&self) -> f64 {
        self.step
    }

    /// Time stored toward the next substep, in seconds, always less than
    /// one step.
    pub fn remainder(&self) -> f64 {
        self.stored
    }

    /// How far the stored time is toward the next substep, from 0 to 1,
    /// for blending a sound's position between two substeps.
    pub fn alpha(&self) -> f64 {
        (self.stored / self.step).clamp(0.0, 1.0)
    }

    /// Substeps run since the clock was made or reset.
    pub fn steps(&self) -> u64 {
        self.steps
    }

    /// Simulated time so far: substeps run times the step.
    pub fn elapsed(&self) -> f64 {
        self.steps as f64 * self.step
    }

    /// Drops stored time and the step count, for a restart.
    pub fn reset(&mut self) {
        self.stored = 0.0;
        self.steps = 0;
    }

    /// Adds one frame's `dt` and calls `substep(step)` once for each whole
    /// substep it completes. Returns how many ran. A negative or
    /// non-finite `dt` counts as 0.
    pub fn advance(&mut self, dt: f64, mut substep: impl FnMut(f64)) -> u32 {
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        self.stored += dt.min(self.max_frame);
        let mut ran = 0;
        while self.stored + SLACK >= self.step {
            self.stored = (self.stored - self.step).max(0.0);
            self.steps += 1;
            ran += 1;
            substep(self.step);
        }
        ran
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{same_at_frame_rates, FRAME_RATES};

    /// A ball dropped from 100 metres, with semi-implicit Euler.
    #[derive(Debug, PartialEq)]
    struct Fall {
        height: f64,
        speed: f64,
    }

    fn drop_for(seconds: f64, dt: f64) -> Fall {
        let mut clock = FixedStep::hz(120.0);
        let mut ball = Fall {
            height: 100.0,
            speed: 0.0,
        };
        let frames = (seconds / dt).round() as usize;
        for _ in 0..frames {
            clock.advance(dt, |step| {
                ball.speed -= 9.81 * step;
                ball.height += ball.speed * step;
            });
        }
        ball
    }

    #[test]
    fn fixed_step_matches_across_frame_rates() {
        let ball = same_at_frame_rates(|dt| drop_for(2.0, dt));
        // About 19.6 metres fallen after 2 seconds.
        assert!((ball.height - 80.4).abs() < 0.2, "{ball:?}");
        // The same number of substeps at every rate.
        for dt in FRAME_RATES {
            let mut clock = FixedStep::hz(120.0);
            for _ in 0..(2.0 / dt).round() as usize {
                clock.advance(dt, |_| {});
            }
            assert_eq!(clock.steps(), 240, "dt {dt}");
        }
    }

    #[test]
    fn fixed_step_carries_remainder() {
        let mut clock = FixedStep::new(0.1);
        assert_eq!(clock.advance(0.25, |_| {}), 2);
        assert!((clock.remainder() - 0.05).abs() < 1e-12);
        assert!((clock.alpha() - 0.5).abs() < 1e-9);
        // The carried 0.05 and this 0.05 make one more step.
        assert_eq!(clock.advance(0.05, |_| {}), 1);
        assert!(clock.remainder() < 1e-9);
        // Too short for a step: stored, nothing run.
        assert_eq!(clock.advance(0.04, |_| {}), 0);
        assert!((clock.remainder() - 0.04).abs() < 1e-12);
        assert_eq!(clock.steps(), 3);
        assert!((clock.elapsed() - 0.3).abs() < 1e-12);
        clock.reset();
        assert_eq!((clock.steps(), clock.remainder()), (0, 0.0));
    }

    #[test]
    fn fixed_step_caps_a_long_stall() {
        let mut clock = FixedStep::new(0.01);
        assert_eq!(clock.advance(10.0, |_| {}), 25);
        let mut clock = FixedStep::new(0.01).with_max_frame(0.05);
        assert_eq!(clock.advance(1.0, |_| {}), 5);
        assert_eq!(clock.advance(-1.0, |_| {}), 0);
        assert_eq!(clock.advance(f64::NAN, |_| {}), 0);
    }

    #[test]
    fn fixed_step_passes_the_step_to_the_sim() {
        let mut clock = FixedStep::hz(50.0);
        let mut seen = Vec::new();
        clock.advance(0.05, |step| seen.push(step));
        assert_eq!(seen, vec![0.02, 0.02]);
    }
}
