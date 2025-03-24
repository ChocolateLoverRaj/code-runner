use core::ops::Deref;

use super::ContinuousBoolVec;

impl<T: Deref<Target = [usize]>> ContinuousBoolVec<T> {
    /// Returns total `false` and total `true`
    pub fn get_sums(&self) -> (usize, usize) {
        let (_, false_sum, true_sum) = self.len_vec.iter().fold(
            (self.start_value, 0, 0),
            |(value, mut false_sum, mut true_sum), len| {
                if value {
                    true_sum += *len;
                } else {
                    false_sum += *len;
                }
                (!value, false_sum, true_sum)
            },
        );
        (false_sum, true_sum)
    }
}
