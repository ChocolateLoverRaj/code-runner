use alloc::collections::btree_map::BTreeMap;
use spinning_top::Spinlock;

pub static IO_PORT_USAGE: Spinlock<BTreeMap<u16, usize>> = Spinlock::new(BTreeMap::new());
