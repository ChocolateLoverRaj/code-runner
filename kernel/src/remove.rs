pub trait Remove<T> {
    fn remove(&mut self, index: usize) -> T;
}

impl<T> Remove<T> for alloc::vec::Vec<T> {
    fn remove(&mut self, index: usize) -> T {
        self.remove(index)
    }
}

impl<T, const N: usize> Remove<T> for heapless::Vec<T, N> {
    fn remove(&mut self, index: usize) -> T {
        self.remove(index)
    }
}
