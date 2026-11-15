//! Fixed-step simulation under a variable frame time.
//!
//! A frame's `dt` changes with the machine and the display: 1/30 on a slow
//! laptop, 1/144 on a fast monitor. Physics that integrates with that `dt`
//! gives a different result on each. [`FixedStep`] collects frame time and
//! runs the simulation in equal substeps, carrying the leftover time to the
//! next frame, so the same play gives the same result at any frame rate.
//!
//! It lives in `wickets-rules`, so rules a server replays can use it too.
//!
//! ```ignore
//! let mut clock = FixedStep::hz(120.0);
//! clock.advance(dt, |step| world.update(step));
//! ```

pub use wickets_rules::fixed_step::{FixedStep, MAX_FRAME};

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
