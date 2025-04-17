use common::ram_disk::RamDisk;
use limine::response::ModuleResponse;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseRamDiskError {
    #[error("The bootloader did not provide a ram disk module")]
    NoModule,
    #[error("Error parsing ram disk")]
    ParseError(postcard::Error),
}

pub fn parse_ram_disk(module_response: &ModuleResponse) -> Result<RamDisk<'_>, ParseRamDiskError> {
    let ram_disk = module_response
        .modules()
        .first()
        .ok_or(ParseRamDiskError::NoModule)?;
    let ram_disk_slice =
        unsafe { core::slice::from_raw_parts(ram_disk.addr(), ram_disk.size() as usize) };
    let ram_disk = postcard::from_bytes(ram_disk_slice).map_err(ParseRamDiskError::ParseError)?;
    Ok(ram_disk)
}
