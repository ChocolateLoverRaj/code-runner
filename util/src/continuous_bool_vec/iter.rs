use core::ops::Deref;

use super::ContinuousBoolVec;

#[derive(Debug, Clone, Copy)]
pub struct IterData {
    pub position: usize,
    pub value: bool,
    pub len: usize,
}

impl<T: Deref<Target = [usize]>> ContinuousBoolVec<T> {
    pub fn iter(&self) -> impl Iterator<Item = IterData> + '_ {
        let mut value = self.start_value;
        let mut position = 0;
        self.len_vec.iter().map(move |&len| {
            let r = IterData {
                position,
                value,
                len,
            };
            value = !value;
            position += len;
            r
        })
    }
}
