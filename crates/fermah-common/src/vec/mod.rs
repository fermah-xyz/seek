use rand::{thread_rng, Rng};

pub trait PowerShuffle {
    fn power_shuffle(&mut self, power: f32);
}

impl<T> PowerShuffle for Vec<T> {
    fn power_shuffle(&mut self, power: f32) {
        // Ensure shuffle_power is within the expected range
        assert!(
            (0.0..=1.0).contains(&power),
            "shuffle_power must be between 0 and 1"
        );

        // Determine the shuffle range based on the size of the vector and shuffle_power
        let shuffle_range = (self.len() as f32 * power) as usize;

        // Shuffle elements within determined ranges to introduce controlled randomness
        let mut rng = thread_rng();
        for i in 0..self.len() {
            if shuffle_range > 0 {
                // Ensure there is a range to shuffle within
                let upper_bound = shuffle_range.min(self.len() - i);
                // Prevent the out-of-bounds by using upper_bound
                let swap_with = i + rng.gen_range(0..upper_bound);
                self.swap(i, swap_with);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_shuffle() {
        let mut v = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        v.power_shuffle(0.5);
        println!("{:?}", v);
    }
}
