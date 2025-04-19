use alloc::collections::btree_map::BTreeMap;
use spinning_top::Spinlock;

use crate::tasks::TaskId;

pub enum IoPortUsedBy {
    Kernel,
    User(TaskId),
}

pub static IO_PORT_USAGE: Spinlock<BTreeMap<u16, IoPortUsedBy>> = Spinlock::new(BTreeMap::new());
