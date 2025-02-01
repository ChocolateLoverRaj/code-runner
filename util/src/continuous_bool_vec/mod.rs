pub mod get_continuous_range;
pub mod is_range_available;
pub mod new;
pub mod set;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ContinuousBoolVec<T> {
    start_value: bool,
    len_vec: T,
}
