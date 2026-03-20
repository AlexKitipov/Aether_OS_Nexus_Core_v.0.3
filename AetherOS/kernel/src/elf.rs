// kernel/src/elf.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use crate::kprintln;
use crate::aetherfs; // To interact with aetherfs for loading binaries

/// Placeholder for an ELF header structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfHeader {
    pub entry_point: u64,
    pub program_headers_offset: u64,
    pub num_program_headers: u16,
    // Add more fields as needed
}

/// A conceptual ELF loader.
pub struct ElfLoader {
    _private: (),
}

impl ElfLoader {
    /// Initializes the ELF loader.
    pub fn init() {
        kprintln!("[kernel] elf: Initializing ElfLoader (conceptual)...");
        // TODO: Any setup required for ELF parsing, e.g., memory regions for loading.
        kprintln!("[kernel] elf: ElfLoader initialized.");
    }

    /// Conceptually loads an ELF binary from the given path.
    /// It would read the file from AetherFS, parse its header, and load segments.
    pub fn load_elf(path: &str) -> Result<ElfHeader, String> {
        kprintln!("[kernel] elf: Conceptually loading ELF from: {}.", path);

        // Simulate reading the ELF binary from AetherFS.
        let elf_data = match aetherfs::read_file(path) {
            Ok(data) => data,
            Err(e) => return Err(format!("Failed to read ELF file '{}': {}", path, e)),
        };

        if elf_data.len() < core::mem::size_of::<ElfHeader>() { // Simplified check
            return Err("ELF file too small to contain header.".to_string());
        }

        // Simulate parsing the ELF header.
        let header = Self::parse_elf_header(&elf_data)?;
        kprintln!("[kernel] elf: Parsed ELF header: {:?}.", header);

        // TODO: In a real loader:
        // 1. Map program segments into virtual memory.
        // 2. Set up initial stack and arguments.
        // 3. Create a new task (V-Node) for the loaded ELF.

        Ok(header)
    }

    /// Conceptually parses an ELF header from a byte slice.
    fn parse_elf_header(elf_data: &[u8]) -> Result<ElfHeader, String> {
        kprintln!("[kernel] elf: Parsing conceptual ELF header...");
        // This parser intentionally validates only a small subset of ELF64 headers,
        // enough to sanity-check binaries before handoff to a fuller loader.

        if elf_data.len() < 64 {
            return Err("ELF header is smaller than expected ELF64 size.".to_string());
        }

        if &elf_data[0..4] != b"\x7FELF" {
            return Err("Invalid ELF magic bytes.".to_string());
        }

        // `2` => ELFCLASS64
        if elf_data[4] != 2 {
            return Err("Unsupported ELF class: expected 64-bit ELF.".to_string());
        }

        // `1` => little-endian
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
