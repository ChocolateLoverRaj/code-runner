use core::ops::{Deref, Range};

use super::ContinuousBoolVec;

impl<T: Deref<Target = [usize]>> ContinuousBoolVec<T> {
    pub fn is_range_available(&self, value: bool, range: Range<usize>) -> bool {
        let mut current_segment_value = self.start_value;
        let mut i = 0;
        let mut current_segment_start_pos = 0;
        loop {
            match self.len_vec.get(i) {
                Some(len) => {
                    let len = *len;
                    let current_segment_end_pos = current_segment_start_pos + len;
                    if current_segment_value == value
                        && current_segment_start_pos <= range.start
                        && current_segment_end_pos >= range.end
                    {
                        break true;
                    }
                    i += 1;
                    current_segment_value = !current_segment_value;
                    current_segment_start_pos = current_segment_end_pos;
                }
                None => break false,
            }
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::ContinuousBoolVec;

    #[test]
    fn blank() {
        let c = ContinuousBoolVec {
            start_value: false,
            len_vec: vec![100],
        };
        let r = c.is_range_available(false, 0..50);
        assert_eq!(r, true);
    }

    #[test]
    fn start_unavailable() {
        let c = ContinuousBoolVec {
            start_value: true,
            len_vec: vec![50, 50],
        };
        let r = c.is_range_available(false, 20..30);
        assert_eq!(r, false);
    }

    #[test]
    fn not_enough_space() {
        let c = ContinuousBoolVec {
            start_value: false,
            len_vec: vec![50, 50],
        };
        let r = c.is_range_available(false, 25..75);
        assert_eq!(r, false);
    }
}
