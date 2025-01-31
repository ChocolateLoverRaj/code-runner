use core::{fmt::Debug, ops::RangeBounds, usize};

pub trait Splice<T> {
    fn splice<R, I>(&mut self, range: R, replace_with: I)
    where
        R: RangeBounds<usize>,
        I: IntoIterator<Item = T>;
}

impl<T> Splice<T> for alloc::vec::Vec<T> {
    fn splice<R, I>(&mut self, range: R, replace_with: I)
    where
        R: RangeBounds<usize>,
        I: IntoIterator<Item = T>,
    {
        self.splice(range, replace_with).skip(usize::MAX);
    }
}

// impl<T: Debug, const N: usize> Splice<T> for heapless::Vec<T, N> {
//     fn splice<R, I>(&mut self, range: R, replace_with: I)
//     where
//         R: RangeBounds<usize>,
//         I: IntoIterator<Item = T>,
//     {
//         let start = match range.start_bound() {

//         }
//     }
// }
