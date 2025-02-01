pub mod get_continuous_range;
pub mod new;
pub mod set;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ContinuousBoolVec<T> {
    start_value: bool,
    len_vec: T,
}
