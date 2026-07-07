/// HyperLogLog cardinality estimator (precision=14 → ~0.81% std error, 16 KB registers).
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const PRECISION: u8 = 14;
const NUM_REGISTERS: usize = 1 << PRECISION; // 16384
const ALPHA: f64 = 0.7213 / (1.0 + 1.079 / NUM_REGISTERS as f64);

pub struct HyperLogLog {
    registers: Vec<u8>,
}

impl Default for HyperLogLog {
    fn default() -> Self {
        Self { registers: vec![0u8; NUM_REGISTERS] }
    }
}

impl HyperLogLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add<T: Hash>(&mut self, value: &T) {
        let hash = Self::hash(value);
        let index = (hash >> (64 - PRECISION)) as usize;
        let remaining = hash << PRECISION;
        let leading_zeros = remaining.leading_zeros() as u8 + 1;
        self.registers[index] = self.registers[index].max(leading_zeros);
    }

    pub fn add_str(&mut self, value: &str) {
        self.add(&value);
    }

    /// Estimate the number of distinct elements.
    pub fn cardinality(&self) -> u64 {
        let m = NUM_REGISTERS as f64;
        let raw = ALPHA * m * m / self.registers.iter().map(|&r| 2f64.powi(-(r as i32))).sum::<f64>();

        // Small-range correction
        let estimate = if raw <= 2.5 * m {
            let zeros = self.registers.iter().filter(|&&r| r == 0).count();
            if zeros > 0 {
                m * (m / zeros as f64).ln()
            } else {
                raw
            }
        // Large-range correction (2^32 threshold)
        } else if raw > (1u64 << 32) as f64 / 30.0 {
            let pow32 = (1u64 << 32) as f64;
            -pow32 * (1.0 - raw / pow32).ln()
        } else {
            raw
        };

        estimate.round() as u64
    }

    fn hash<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hll_empty() {
        let hll = HyperLogLog::new();
        assert_eq!(hll.cardinality(), 0);
    }

    #[test]
    fn test_hll_approximate_cardinality() {
        let mut hll = HyperLogLog::new();
        let n = 10_000u64;
        for i in 0..n {
            hll.add(&i);
        }
        let est = hll.cardinality();
        // Within 2% of true value
        let error = (est as f64 - n as f64).abs() / n as f64;
        assert!(error < 0.02, "error {error:.4} exceeds 2%");
    }
}
