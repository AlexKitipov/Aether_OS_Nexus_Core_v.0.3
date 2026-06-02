#![allow(dead_code)]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::aetherfs;
use crate::kprintln;

const ELF64_HEADER_SIZE: usize = 64;
const ELF64_PROGRAM_HEADER_SIZE: u16 = 56;
const PT_LOAD: u32 = 1;
const PF_X: u32 = 0x1;
const PF_W: u32 = 0x2;
const LOW_CANONICAL_USER_END: u64 = 0x0000_8000_0000_0000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElfHeader {
    pub entry_point: u64,
    pub program_headers_offset: u64,
    pub num_program_headers: u16,
    pub load_segments: Vec<ElfLoadSegment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfLoadSegment {
    pub virtual_start: u64,
    pub memory_size: u64,
    pub file_offset: u64,
    pub file_size: u64,
    pub writable: bool,
    pub executable: bool,
}

impl ElfLoadSegment {
    pub fn virtual_end(&self) -> Result<u64, String> {
        self.virtual_start
            .checked_add(self.memory_size)
            .ok_or_else(|| String::from("ELF PT_LOAD virtual range overflows u64."))
    }

    pub fn file_end(&self) -> Result<u64, String> {
        self.file_offset
            .checked_add(self.file_size)
            .ok_or_else(|| String::from("ELF PT_LOAD file range overflows u64."))
    }
}

pub struct ElfLoader {
    _private: (),
}

impl ElfLoader {
    pub fn init() {
        kprintln!("[kernel] elf: Initializing ElfLoader...");
        kprintln!("[kernel] elf: ElfLoader initialized.");
    }

    pub fn load_elf(path: &str) -> Result<ElfHeader, String> {
        kprintln!("[kernel] elf: loading ELF from path: {}.", path);
        let elf_data = aetherfs::read_file(path)
            .map_err(|e| format!("Failed to read ELF file '{}': {}", path, e))?;
        Self::parse_elf_bytes(&elf_data)
    }

    pub fn parse_elf_bytes(elf_data: &[u8]) -> Result<ElfHeader, String> {
        kprintln!(
            "[kernel] elf: parsing ELF header and PT_LOAD segments from immutable image bytes."
        );

        if elf_data.len() < ELF64_HEADER_SIZE {
            return Err("ELF header is smaller than expected ELF64 size.".to_string());
        }

        if &elf_data[0..4] != b"\x7FELF" {
            return Err("Invalid ELF magic bytes.".to_string());
        }

        if elf_data[4] != 2 {
            return Err("Unsupported ELF class: expected 64-bit ELF.".to_string());
        }

        if elf_data[5] != 1 {
            return Err("Unsupported ELF endianness: expected little-endian.".to_string());
        }

        let entry_point = read_u64(elf_data, 24, "ELF entry point")?;
        let program_headers_offset = read_u64(elf_data, 32, "ELF program header offset")?;
        let program_header_entry_size = read_u16(elf_data, 54, "ELF program header entry size")?;
        let num_program_headers = read_u16(elf_data, 56, "ELF program header count")?;

        if program_header_entry_size != ELF64_PROGRAM_HEADER_SIZE {
            return Err(format!(
                "Unsupported ELF program header size: expected {}, got {}.",
                ELF64_PROGRAM_HEADER_SIZE,
                program_header_entry_size
            ));
        }

        validate_user_address(entry_point, "ELF entry point")?;

        let load_segments = Self::parse_load_segments(
            elf_data,
            program_headers_offset,
            program_header_entry_size,
            num_program_headers,
        )?;

        if load_segments.is_empty() {
            return Err(String::from("ELF image has no loadable PT_LOAD segments."));
        }

        if !load_segments.iter().any(|segment| {
            segment
                .virtual_end()
                .map(|end| entry_point >= segment.virtual_start && entry_point < end)
                .unwrap_or(false)
        }) {
            return Err(String::from("ELF entry point is not contained in a loadable segment."));
        }

        Ok(ElfHeader {
            entry_point,
            program_headers_offset,
            num_program_headers,
            load_segments,
        })
    }

    fn parse_load_segments(
        elf_data: &[u8],
        program_headers_offset: u64,
        program_header_entry_size: u16,
        num_program_headers: u16,
    ) -> Result<Vec<ElfLoadSegment>, String> {
        let table_size = (program_header_entry_size as u64)
            .checked_mul(num_program_headers as u64)
            .ok_or_else(|| String::from("ELF program header table size overflows u64."))?;
        let table_end = program_headers_offset
            .checked_add(table_size)
            .ok_or_else(|| String::from("ELF program header table range overflows u64."))?;
        if table_end > elf_data.len() as u64 {
            return Err(String::from("ELF program header table extends past the image size."));
        }

        let mut load_segments = Vec::new();
        for index in 0..num_program_headers {
            let header_offset = program_headers_offset
                .checked_add((index as u64) * program_header_entry_size as u64)
                .ok_or_else(|| String::from("ELF program header offset overflows u64."))?
                as usize;
            let header =
                &elf_data[header_offset..header_offset + program_header_entry_size as usize];
            let segment_type = read_u32(header, 0, "ELF program header type")?;
            if segment_type != PT_LOAD {
                continue;
            }

            let flags = read_u32(header, 4, "ELF PT_LOAD flags")?;
            let file_offset = read_u64(header, 8, "ELF PT_LOAD file offset")?;
            let virtual_start = read_u64(header, 16, "ELF PT_LOAD virtual address")?;
            let file_size = read_u64(header, 32, "ELF PT_LOAD file size")?;
            let memory_size = read_u64(header, 40, "ELF PT_LOAD memory size")?;
            let alignment = read_u64(header, 48, "ELF PT_LOAD alignment")?;

            if memory_size == 0 {
                continue;
            }
            if file_size > memory_size {
                return Err(String::from("ELF PT_LOAD file size exceeds memory size."));
            }
            let file_end = file_offset
                .checked_add(file_size)
                .ok_or_else(|| String::from("ELF PT_LOAD file range overflows u64."))?;
            if file_end > elf_data.len() as u64 {
                return Err(String::from("ELF PT_LOAD file range extends past the image size."));
            }
            validate_user_range(virtual_start, memory_size, "ELF PT_LOAD virtual range")?;
            if alignment > 1 {
                if !alignment.is_power_of_two() {
                    return Err(String::from("ELF PT_LOAD alignment is not a power of two."));
                }
                if virtual_start % alignment != file_offset % alignment {
                    return Err(String::from(
                        "ELF PT_LOAD virtual address and file offset alignment mismatch.",
                    ));
                }
            }

            load_segments.push(ElfLoadSegment {
                virtual_start,
                memory_size,
                file_offset,
                file_size,
                writable: flags & PF_W != 0,
                executable: flags & PF_X != 0,
            });
        }

        validate_non_overlapping_segments(&mut load_segments)?;
        Ok(load_segments)
    }
}

fn validate_non_overlapping_segments(segments: &mut [ElfLoadSegment]) -> Result<(), String> {
    segments.sort_by_key(|segment| segment.virtual_start);
    for pair in segments.windows(2) {
        let current_end = pair[0].virtual_end()?;
        if current_end > pair[1].virtual_start {
            return Err(String::from("ELF PT_LOAD virtual ranges overlap."));
        }

        let current_page_end = align_up(current_end)?;
        let next_page_start = align_down(pair[1].virtual_start);
        if current_page_end > next_page_start {
            return Err(String::from("ELF PT_LOAD mapped page ranges overlap."));
        }
    }
    Ok(())
}

fn validate_user_address(address: u64, label: &str) -> Result<(), String> {
    if address >= LOW_CANONICAL_USER_END {
        return Err(format!(
            "{} is non-canonical or in the kernel address range.",
            label
        ));
    }
    Ok(())
}

fn validate_user_range(start: u64, size: u64, label: &str) -> Result<(), String> {
    if size == 0 {
        return Ok(());
    }
    validate_user_address(start, label)?;
    let end = start
        .checked_add(size)
        .ok_or_else(|| format!("{} overflows u64.", label))?;
    if end > LOW_CANONICAL_USER_END {
        return Err(format!(
            "{} is non-canonical or crosses into the kernel address range.",
            label
        ));
    }
    Ok(())
}

const PAGE_SIZE: u64 = 4096;

fn align_down(value: u64) -> u64 {
    value & !(PAGE_SIZE - 1)
}

fn align_up(value: u64) -> Result<u64, String> {
    if value == 0 {
        return Ok(0);
    }
    let adjusted = value
        .checked_add(PAGE_SIZE - 1)
        .ok_or_else(|| String::from("ELF PT_LOAD page-aligned range overflows u64."))?;
    Ok(align_down(adjusted))
}

fn read_u16(bytes: &[u8], offset: usize, label: &str) -> Result<u16, String> {
    let end = offset
        .checked_add(core::mem::size_of::<u16>())
        .ok_or_else(|| format!("{} offset overflows usize.", label))?;
    Ok(u16::from_le_bytes(
        bytes
            .get(offset..end)
            .ok_or_else(|| format!("Failed to parse {} bytes.", label))?
            .try_into()
            .map_err(|_| format!("Failed to parse {} bytes.", label))?,
    ))
}

fn read_u32(bytes: &[u8], offset: usize, label: &str) -> Result<u32, String> {
    let end = offset
        .checked_add(core::mem::size_of::<u32>())
        .ok_or_else(|| format!("{} offset overflows usize.", label))?;
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..end)
            .ok_or_else(|| format!("Failed to parse {} bytes.", label))?
            .try_into()
            .map_err(|_| format!("Failed to parse {} bytes.", label))?,
    ))
}

fn read_u64(bytes: &[u8], offset: usize, label: &str) -> Result<u64, String> {
    let end = offset
        .checked_add(core::mem::size_of::<u64>())
        .ok_or_else(|| format!("{} offset overflows usize.", label))?;
    Ok(u64::from_le_bytes(
        bytes
            .get(offset..end)
            .ok_or_else(|| format!("Failed to parse {} bytes.", label))?
            .try_into()
            .map_err(|_| format!("Failed to parse {} bytes.", label))?,
    ))
}
