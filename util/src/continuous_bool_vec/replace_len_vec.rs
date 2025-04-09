use super::ContinuousBoolVec;

impl<T> ContinuousBoolVec<T> {
    pub fn replace_len_vec<U, F: FnOnce(T) -> U>(self, f: F) -> ContinuousBoolVec<U> {
        ContinuousBoolVec {
            start_value: self.start_value,
            len_vec: f(self.len_vec),
        }
    }
}
