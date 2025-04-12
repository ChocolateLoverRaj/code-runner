use limine::response::MemoryMapResponse;
use rangemap::RangeMap;
use spinning_top::Spinlock;
use util::init_later::{InitLater, TryInitError};
use x86_64::{
    structures::paging::{FrameAllocator, PageSize, PhysFrame},
    PhysAddr,
};

use crate::available_physical_frame_iterator::{
    AvailablePhysicalFrameIterator, AvailablePhysicalRegionsIterator,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsedBy {
    Kernel,
    UserSpace(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalMemoryState {
    Available,
    Used(UsedBy),
}

pub static PHYSICAL_MEMORY: InitLater<Spinlock<RangeMap<PhysAddr, PhysicalMemoryState>>> =
    InitLater::uninit();

/// Allocates
pub fn init(
    mut frames_iterator: AvailablePhysicalFrameIterator,
    memory_map_response: &'static MemoryMapResponse,
) -> Result<&'static Spinlock<RangeMap<PhysAddr, PhysicalMemoryState>>, TryInitError> {
    let mut range_map = RangeMap::default();

    // Add all initially available regions
    AvailablePhysicalRegionsIterator::from(memory_map_response).for_each(|available_region| {
        range_map.insert(available_region, PhysicalMemoryState::Available);
    });

    // Mark already allocated frames as used by kernel
    let first_available_frame = frames_iterator.next();
    log::info!("First available frame: {:?}", first_available_frame);
    AvailablePhysicalFrameIterator::from(AvailablePhysicalRegionsIterator::from(
        memory_map_response,
    ))
    .take_while(|frame| {
        if let Some(first_available_frame) = &first_available_frame {
            frame < first_available_frame
        } else {
            false
        }
    })
    .for_each(|frame| {
        range_map.insert(
            frame.start_address()..(frame + 1).start_address(),
            PhysicalMemoryState::Used(UsedBy::Kernel),
        );
    });
    PHYSICAL_MEMORY.try_init(Spinlock::new(range_map))
}

pub struct PhysicalMemoryFrameAllocator<'a> {
    range_map: &'a mut RangeMap<PhysAddr, PhysicalMemoryState>,
    used_by: UsedBy,
}

impl<'a> PhysicalMemoryFrameAllocator<'a> {
    pub fn new(
        range_map: &'a mut RangeMap<PhysAddr, PhysicalMemoryState>,
        used_by: UsedBy,
    ) -> Self {
        Self { range_map, used_by }
    }
}

unsafe impl<S: PageSize> FrameAllocator<S> for PhysicalMemoryFrameAllocator<'_> {
    fn allocate_frame(&mut self) -> Option<x86_64::structures::paging::PhysFrame<S>> {
        let frame = self.range_map.iter().find_map(|(range, state)| {
            if let PhysicalMemoryState::Available = state {
                let frame =
                    PhysFrame::<S>::from_start_address(range.start.align_up(S::SIZE)).unwrap();
                if range.contains(&(frame + 1).start_address()) {
                    Some(frame)
                } else {
                    None
                }
            } else {
                None
            }
        })?;
        self.range_map.insert(
            frame.start_address()..(frame + 1).start_address(),
            PhysicalMemoryState::Used(self.used_by),
        );
        Some(frame)
    }
}
