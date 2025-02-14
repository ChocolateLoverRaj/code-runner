use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, MaxSize, PartialEq, Eq)]
pub enum FullQueueBehavior {
    DropOldest,
    DropNewest,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, MaxSize, PartialEq, Eq)]
pub struct SyscallStartRecordingKeyboardInput {
    pub queue_size: u64,
    pub behavior_on_full_queue: FullQueueBehavior,
}
