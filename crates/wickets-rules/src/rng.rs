/// A small, fast, seedable random source (xorshift64*), so games need no
/// extra dependency and tests can replay a run exactly.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    /// A source that always gives the same sequence for the same seed.
    pub fn seeded(seed: u64) -> Rng {
        // Mix the seed with splitmix64 so every bit of it counts and close
        // seeds start far apart; zero is xorshift's one fixed point, so it
        // is never the state.
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        Rng(if z == 0 { 0x5EED } else { z })
    }

    /// A source seeded from the clock, for real play.
    pub fn from_time() -> Rng {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5EED);
        Rng::seeded(nanos)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A whole number in `0..n`. `n` of zero gives zero.
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        (((self.next_u64() >> 32) * n as u64) >> 32) as u32
    }

    /// A whole number in `lo..=hi`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + self.below((hi - lo + 1) as u32) as i32
    }

    /// A number in `0.0..1.0`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// True with probability `p`.
    pub fn chance(&mut self, p: f64) -> bool {
        self.unit() < p
    }

    /// One item of a non-empty slice.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u32) as usize]
    }

    /// Shuffles a slice in place.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i as u32 + 1) as usize;
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Rng;

    #[test]
    fn the_same_seed_gives_the_same_run() {
        let mut a = Rng::seeded(7);
        let mut b = Rng::seeded(7);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn neighbouring_seeds_give_different_runs() {
        assert_ne!(Rng::seeded(2).next_u64(), Rng::seeded(3).next_u64());
        assert_ne!(Rng::seeded(0).next_u64(), Rng::seeded(1).next_u64());
    }

    #[test]
    fn below_and_range_stay_in_bounds() {
        let mut r = Rng::seeded(1);
        for _ in 0..10_000 {
            assert!(r.below(3) < 3);
            let v = r.range(-2, 2);
            assert!((-2..=2).contains(&v));
            let u = r.unit();
            assert!((0.0..1.0).contains(&u));
        }
    }
}
