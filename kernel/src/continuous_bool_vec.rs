use core::{
    cmp::Ordering,
    ops::{DerefMut, Range},
};

use crate::{insert::Insert, remove::Remove};

pub struct ContinuousBoolVec<T> {
    start_value: bool,
    len_vec: T,
}

impl<T: Default + Insert<usize>> ContinuousBoolVec<T> {
    pub fn new(len: usize, start_value: bool) -> Self {
        Self {
            start_value,
            len_vec: {
                let mut len_vec = T::default();
                len_vec.insert(0, len);
                len_vec
            },
        }
    }
}

impl<T: DerefMut<Target = [usize]> + Insert<usize> + Remove<usize>> ContinuousBoolVec<T> {
    pub fn set(&mut self, range: Range<usize>, value: bool) {
        let mut current_segment_value = self.start_value;
        let mut index = 0;
        let mut i = 0;
        loop {
            let end_index = index + self.len_vec[i];
            match end_index.cmp(&range.start) {
                Ordering::Less => {
                    i += 1;
                    current_segment_value = !current_segment_value;
                    index = end_index;
                }
                Ordering::Equal => {
                    if current_segment_value == value {
                        self.len_vec[i] += range.len();
                    } else {
                        self.len_vec.insert(i + 1, range.len());
                    }
                    // Remove / shrunk next segments
                    let mut i = i + 1;
                    let mut shrink_len = range.len();
                    loop {
                        if self.len_vec[i] > shrink_len {
                            self.len_vec[i] -= shrink_len;
                            break;
                        } else {
                            self.len_vec.remove(i);
                        }
                    }
                    break;
                }
                Ordering::Greater => {
                    if current_segment_value == value {
                        let new_end_index = range.end;
                        self.len_vec[i] += new_end_index - end_index;
                    } else {
                    }
                    todo!()
                }
            }
        }
    }
}

#[cfg(test)]
pub mod test {
    use alloc::vec::Vec;

    use super::ContinuousBoolVec;

    #[test]
    fn set() {
        let mut c = ContinuousBoolVec::<Vec<_>>::new(100, false);
        c.set(0..25, true);
        println!(c);
    }
}
