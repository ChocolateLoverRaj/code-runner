use core::{cell::RefCell, task::Waker};

use alloc::collections::btree_map::BTreeMap;
use common::syscall_uuids::EventId;
use futures::task::AtomicWaker;

#[derive(Debug, Default)]
struct EventData {
    waker: AtomicWaker,
    event_happened: bool,
}

#[derive(Debug, Default)]
pub struct ExecutorContext {
    events: RefCell<BTreeMap<EventId, EventData>>,
}

impl ExecutorContext {
    pub fn register_waker(&self, event_id: EventId, waker: &Waker) {
        self.events
            .borrow_mut()
            .entry(event_id)
            .or_insert(Default::default())
            .waker
            .register(waker);
    }

    /// Returns if the event happened, setting the event happened to `false` if it did happen.
    pub fn take_event(&self, event_id: &EventId) -> bool {
        let mut events = self.events.borrow_mut();
        let event_happened = &mut events.get_mut(event_id).unwrap().event_happened;
        if *event_happened {
            *event_happened = false;
            true
        } else {
            false
        }
    }

    /// Returns the total number of events that we're listening to
    pub fn events_count(&self) -> usize {
        self.events.borrow().len()
    }

    pub fn set_happened_and_wake(&self, event_id: &EventId) {
        let mut events = self.events.borrow_mut();
        let event_data = events.get_mut(event_id).unwrap();
        event_data.event_happened = true;
        event_data.waker.wake();
    }
}
