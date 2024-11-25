use futures_util::{Stream, StreamExt};

pub trait StreamChanges<T> {
    fn changes(self) -> impl Stream<Item = T>;
}

impl<S: Stream> StreamChanges<S::Item> for S
where
    S::Item: Clone + PartialEq,
{
    fn changes(self) -> impl Stream<Item = S::Item> {
        let mut last_value = None::<S::Item>;
        self.filter(move |value| {
            let ret = match last_value.replace(value.clone()) {
                Some(previous_value) => value != &previous_value,
                None => true,
            };
            async move { ret }
        })
    }
}
