pub trait TryPush<T> {
    fn try_push(&mut self, item: T) -> Result<(), T>;
}

impl<T, const N: usize> TryPush<T> for heapless::Vec<T, N> {
    fn try_push(&mut self, item: T) -> Result<(), T> {
        self.push(item)
    }
}
