pub mod get_continuous_range;
pub mod get_sums;
pub mod is_range_available;
pub mod iter;
pub mod new;
pub mod set;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ContinuousBoolVec<T> {
    pub start_value: bool,
    pub len_vec: T,
}
