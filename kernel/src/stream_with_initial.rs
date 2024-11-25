use futures_util::{stream, Stream, StreamExt};

pub trait StreamWithInitial<T> {
    fn with_initial(self, initial: T) -> impl Stream<Item = T>;
}

// trait F<T>: (Fn() -> T) + Clone {}

impl<S: Stream> StreamWithInitial<S::Item> for S {
    fn with_initial(self, initial: S::Item) -> impl Stream<Item = S::Item> {
        stream::once(async { initial }).chain(self)
    }
}
