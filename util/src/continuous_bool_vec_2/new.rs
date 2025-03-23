use super::ContinuousBoolVec2;

impl<T> ContinuousBoolVec2<T> {
    /// The `len_vec` should be empty
    pub fn new(start_value: bool, len_vec: T) -> Self {
        Self {
            start_value,
            len_vec,
        }
    }
}
