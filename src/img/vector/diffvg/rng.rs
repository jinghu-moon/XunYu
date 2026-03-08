pub(super) struct Rng {
    s: u64,
}

impl Rng {
    pub(super) fn new(seed: u64) -> Self {
        Self { s: seed }
    }

    pub(super) fn next(&mut self) -> u64 {
        self.s = self
            .s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.s >> 33
    }

    pub(super) fn f64(&mut self) -> f64 {
        self.next() as f64 / (u32::MAX as f64)
    }

    pub(super) fn next_bool(&mut self) -> bool {
        self.next() & 1 == 0
    }
}
