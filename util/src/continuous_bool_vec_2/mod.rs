pub mod new;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ContinuousBoolVec2<T> {
    start_value: bool,
    len_vec: T,
}

impl<T> ContinuousBoolVec2<T> {
    pub fn len_vec_mut(&mut self) -> &mut T {
        &mut self.len_vec
    }
}
