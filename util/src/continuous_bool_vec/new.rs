use crate::insert::Insert;

use super::ContinuousBoolVec;

impl<T: Default + Insert<usize>> ContinuousBoolVec<T> {
    pub fn new(len: usize, start_value: bool) -> Self {
        Self {
            start_value,
            len_vec: {
                let mut len_vec = T::default();
                len_vec.insert(0, len);
                len_vec
            },
        }
    }
}
