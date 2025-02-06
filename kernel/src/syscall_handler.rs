use common::Syscall;

pub extern "sysv64" fn syscall_handler(
    input0: u64,
    input1: u64,
    input2: u64,
    input3: u64,
    input4: u64,
    input5: u64,
    input6: u64,
) -> u64 {
    let inputs = [input0, input1, input2, input3, input4, input5, input6];
    match Syscall::deserialize_from_input(inputs) {
        Ok(syscall) => match syscall {
            Syscall::Print(message_ptr) => {
                log::info!("Print: 0x{:x}", message_ptr);
            }
        },
        Err(e) => {
            log::warn!(
                "Failed to parse syscall inputs (displayed in hex) {:x?}: {:?}",
                inputs,
                e
            );
        }
    }
    0xabcdef
}
