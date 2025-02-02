use alloc::vec::Vec;
use anyhow::{anyhow, Context};
use elf::{endian::NativeEndian, ElfBytes};

pub const FLEXIBLE_VIRT_MEM_START: u64 = 0xFFFF_8000_0000_0000;

pub fn jmp_to_elf(elf_bytes: &[u8]) -> anyhow::Result<()> {
    let elf = ElfBytes::<NativeEndian>::minimal_parse(elf_bytes).unwrap();
    let (headers, string_table) = elf.section_headers_with_strtab().unwrap();
    let headers = headers
        .unwrap()
        .into_iter()
        .map(|name| string_table.unwrap().get(name.sh_name as usize))
        .collect::<Vec<_>>();
    let segments = elf.segments().unwrap();
    let loadable_segments = segments
        .into_iter()
        .filter(|segment| segment.p_type == 1)
        .collect::<Vec<_>>();
    let (symbols_parsing_table, symbols_strings) = elf.symbol_table().unwrap().unwrap();
    let start_symbol = symbols_parsing_table
        .into_iter()
        .filter(|symbol| !symbol.is_undefined())
        .find_map(
            |symbol| match symbols_strings.get(symbol.st_name as usize) {
                Ok(symbol_string) => match symbol_string {
                    "_start" => Some(Ok(symbol)),
                    _ => None,
                },
                Err(e) => Some(Err(e)),
            },
        )
        .ok_or(anyhow!("_start not found"))?
        .context("Error finding _start symbol")?;
    log::info!("ELF: {:#?}", loadable_segments);
    log::info!("Symbols: {:#?}", start_symbol);
    Err(anyhow!("Noe imp"))
}
