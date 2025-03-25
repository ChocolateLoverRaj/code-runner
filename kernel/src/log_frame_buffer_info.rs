use limine::framebuffer::MemoryModel;

use crate::limine_requests::FRAME_BUFFER_REQUEST;

pub fn log_frame_buffer_info() {
    let frame_buffer_response = FRAME_BUFFER_REQUEST.get_response().unwrap();
    frame_buffer_response
        .framebuffers()
        .for_each(|frame_buffer| {
            log::info!(
                "Frame buffer at {:?} with size {}x{}. Is RGB? {}",
                frame_buffer.addr(),
                frame_buffer.width(),
                frame_buffer.height(),
                frame_buffer.memory_model() == MemoryModel::RGB
            )
        });
}
