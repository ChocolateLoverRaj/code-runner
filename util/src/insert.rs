use core::alloc::Allocator;
use core::fmt::Debug;

pub trait Insert<T> {
    fn insert(&mut self, index: usize, element: T);
}

impl<T, A: Allocator> Insert<T> for alloc::vec::Vec<T, A> {
    fn insert(&mut self, index: usize, element: T) {
        self.insert(index, element);
    }
}
impl<T: Debug, const N: usize> Insert<T> for heapless::Vec<T, N> {
    fn insert(&mut self, index: usize, element: T) {
        self.insert(index, element).unwrap();
    }
}
