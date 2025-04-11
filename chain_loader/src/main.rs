#![no_main]
#![no_std]

extern crate alloc;

use alloc::string::ToString;
use log::info;
use uefi::boot::{self, LoadImageSource, SearchType};
use uefi::prelude::*;
use uefi::proto::device_path::DevicePath;
use uefi::proto::device_path::text::{AllowShortcuts, DevicePathToText, DisplayOnly};
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::file::{Directory, File, FileAttribute, FileMode, FileSystemInfo};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::proto::media::load_file::LoadFile;
use uefi::{Identify, Result};

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    print_image_path().unwrap();

    let fs_handle = boot::get_handle_for_protocol::<SimpleFileSystem>().unwrap();
    let mut fs = boot::open_protocol_exclusive::<SimpleFileSystem>(fs_handle).unwrap();
    let mut root_dir = fs.open_volume().unwrap();
    let info = root_dir.get_boxed_info::<FileSystemInfo>().unwrap();
    info!("Info: {:#?}", info);

    fn print_dir(mut dir: Directory) {
        while let Some(entry) = dir.read_entry_boxed().unwrap() {
            if [".", ".."].contains(&entry.file_name().to_string().as_str())
                || entry.attribute().contains(FileAttribute::ARCHIVE)
                || entry.attribute().contains(FileAttribute::HIDDEN)
            {
                continue;
            }
            info!("Entry: {:#?}", entry);
            if entry.is_directory() {
                let sub_dir = dir
                    .open(entry.file_name(), FileMode::Read, FileAttribute::DIRECTORY)
                    .unwrap()
                    .into_directory()
                    .unwrap();
                print_dir(sub_dir);
            }
        }
    }

    print_dir(root_dir);

    let mut image = boot::load_image(
        boot::image_handle(),
        LoadImageSource::FromBuffer {
            buffer: include_bytes!("../../limine/share/limine/BOOTX64.EFI"),
            file_path: None,
        },
    )
    .unwrap();
    // boot::start_image(image).unwrap();

    loop {
        boot::stall(10_000_000);
    }
    Status::SUCCESS
}

fn print_image_path() -> Result {
    let loaded_image = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle())?;

    let device_path_to_text_handle =
        *boot::locate_handle_buffer(SearchType::ByProtocol(&DevicePathToText::GUID))?
            .first()
            .expect("DevicePathToText is missing");

    let device_path_to_text =
        boot::open_protocol_exclusive::<DevicePathToText>(device_path_to_text_handle)?;

    let image_device_path = loaded_image.file_path().expect("File path is not set");
    let image_device_path_text = device_path_to_text
        .convert_device_path_to_text(image_device_path, DisplayOnly(true), AllowShortcuts(false))
        .expect("convert_device_path_to_text failed");

    info!("Image path: {}", &*image_device_path_text);
    Ok(())
}
