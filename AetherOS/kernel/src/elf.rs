#![allow(dead_code)]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};

use crate::aetherfs;
use crate::kprintln;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfHeader {
    pub entry_point: u64,
    pub program_headers_offset: u64,
    pub num_program_headers: u16,
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
        kprintln!("[kernel] elf: parsing ELF header from immutable image bytes.");

        if elf_data.len() < 64 {
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

        let entry_point = u64::from_le_bytes(
            elf_data[24..32]
                .try_into()
                .map_err(|_| "Failed to parse ELF entry point bytes.".to_string())?,
        );
        let program_headers_offset = u64::from_le_bytes(
            elf_data[32..40]
                .try_into()
                .map_err(|_| "Failed to parse ELF program header offset bytes.".to_string())?,
        );
        let num_program_headers = u16::from_le_bytes(
            elf_data[56..58]
                .try_into()
                .map_err(|_| "Failed to parse ELF program header count bytes.".to_string())?,
        );

        Ok(ElfHeader {
            entry_point,
            program_headers_offset,
            num_program_headers,
        })
    }
}
