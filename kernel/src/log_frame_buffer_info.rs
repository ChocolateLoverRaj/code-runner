use limine::{framebuffer::MemoryModel, response::FramebufferResponse};

pub fn log_frame_buffer_info(frame_buffer_response: Option<&FramebufferResponse>) {
    if let Some(frame_buffer_response) = frame_buffer_response {
        frame_buffer_response
            .framebuffers()
            .for_each(|frame_buffer| {
                log::info!(
                    "Frame buffer at {:?} with size {}x{}. Is RGB? {}. BPP: {}",
                    frame_buffer.addr(),
                    frame_buffer.width(),
                    frame_buffer.height(),
                    frame_buffer.memory_model() == MemoryModel::RGB,
                    frame_buffer.bpp()
                )
            });
    }
}
