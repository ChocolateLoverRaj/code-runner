use core::{cell::SyncUnsafeCell, mem::MaybeUninit, sync::atomic::Ordering};

use atomic_enum::atomic_enum;
use thiserror::Error;

#[atomic_enum]
#[derive(PartialEq)]
enum State {
    Uninit,
    Initializing,
    Initialized,
}

#[derive(Debug)]
pub struct InitLater<T> {
    state: AtomicState,
    data: SyncUnsafeCell<MaybeUninit<T>>,
}

#[derive(Debug, Error)]
pub enum TryInitError {
    #[error("Already initialized or being initialized from a different thread")]
    NotUninitialized,
}

#[derive(Debug, Error)]
pub enum TryGetError {
    #[error("Not initialized")]
    NotInitialized,
}

impl<T> InitLater<T> {
    pub const fn uninit() -> Self {
        Self {
            state: AtomicState::new(State::Uninit),
            data: SyncUnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    pub fn try_init(&self, val: T) -> Result<&T, TryInitError> {
        match self.state.compare_exchange(
            State::Uninit,
            State::Initializing,
            Ordering::Acquire,
            Ordering::Acquire,
        ) {
            Ok(_) => Ok({
                let maybe_uninit = unsafe { &mut *self.data.get() };
                let r = maybe_uninit.write(val);
                self.state.store(State::Initialized, Ordering::Release);
                r
            }),
            Err(_) => Err(TryInitError::NotUninitialized),
        }
    }

    pub fn try_get(&self) -> Result<&T, TryGetError> {
        if self.state.load(Ordering::Acquire) == State::Initialized {
            Ok(unsafe { (*self.data.get()).assume_init_ref() })
        } else {
            Err(TryGetError::NotInitialized)
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.state.load(Ordering::Acquire) == State::Initialized
    }
}
