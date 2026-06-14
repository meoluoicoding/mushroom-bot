#[derive(Clone)]
pub struct ZooRng(u64);

impl ZooRng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn reseed(&mut self, seed: u64) {
        self.0 = seed;
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    pub fn gen_range(&mut self, upper: usize) -> usize {
        if upper <= 1 {
            0
        } else {
            (self.next_u64() as usize) % upper
        }
    }

    pub fn roll_f64(&mut self) -> f64 {
        (self.next_u64() as f64 / u64::MAX as f64).clamp(0.0, 1.0)
    }
}
