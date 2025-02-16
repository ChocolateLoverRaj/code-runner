use crate::context::Context;

// For now since we don't have any kernel tasks and only have 1 user space task this can just be an `Option` instead of a list of tasks
#[derive(Debug)]
pub struct UserSpaceState {
    // TODO: Don't use a fixed size vec
    pub stack_of_saved_contexts: heapless::Vec<Context, 10>,
    pub currently_running: bool,
}

pub type State = Option<UserSpaceState>;
