use super::*;
use std::io::{Cursor, Result};

#[test]
fn no_symbols_le32() -> Result<()> {
    let buf: Vec<u8> = Vec::new();
    let cursor = Cursor::new(buf);
    let builder = Builder::new(
        Header {
            class: Class::ELF32,
            encoding: Encoding::LSB,
            machine: 0x28,     // ARM instruction set
            flags: 0x05000000, // ARM ABI version 5
        },
        cursor,
    )?;
    let mut cursor = builder.close()?;
    cursor.seek(std::io::SeekFrom::Start(0))?;

    let ef = elf::File::open_stream(&mut cursor).unwrap();
    assert_eq!(
        ef.ehdr,
        elf::types::FileHeader {
            class: elf::types::ELFCLASS32,
            data: elf::types::ELFDATA2LSB,
            version: elf::types::Version(1),
            osabi: elf::types::ELFOSABI_NONE,
            abiversion: 0,
            elftype: elf::types::ET_REL,
            machine: elf::types::EM_ARM,
            entry: 0,
        }
    );
    assert_eq!(ef.phdrs.len(), 0, "no program headers");
    assert_eq!(ef.sections.len(), 5, "five section headers");
    let symtab = ef.get_section(".symtab").unwrap();
    let syms = ef.get_symbols(symtab).unwrap();
    assert_eq!(syms.len(), 0, "no symbols");

    Ok(())
}

#[test]
fn no_symbols_be32() -> Result<()> {
    let buf: Vec<u8> = Vec::new();
    let cursor = Cursor::new(buf);
    let builder = Builder::new(
        Header {
            class: Class::ELF32,
            encoding: Encoding::MSB,
            machine: 0x28,     // ARM instruction set
            flags: 0x05000000, // ARM ABI version 5
        },
        cursor,
    )?;
    let mut cursor = builder.close()?;
    cursor.seek(std::io::SeekFrom::Start(0))?;

    let ef = elf::File::open_stream(&mut cursor).unwrap();
    assert_eq!(
        ef.ehdr,
        elf::types::FileHeader {
            class: elf::types::ELFCLASS32,
            data: elf::types::ELFDATA2MSB,
            version: elf::types::Version(1),
            osabi: elf::types::ELFOSABI_NONE,
            abiversion: 0,
            elftype: elf::types::ET_REL,
            machine: elf::types::EM_ARM,
            entry: 0,
        }
    );
    assert_eq!(ef.phdrs.len(), 0, "no program headers");
    assert_eq!(ef.sections.len(), 5, "five section headers");
    let symtab = ef.get_section(".symtab").unwrap();
    let syms = ef.get_symbols(symtab).unwrap();
    assert_eq!(syms.len(), 0, "no symbols");

    Ok(())
}

#[test]
fn three_symbols_le32() -> Result<()> {
    let buf: Vec<u8> = Vec::new();
    let cursor = Cursor::new(buf);
    let mut builder = Builder::new(
        Header {
            class: Class::ELF32,
            encoding: Encoding::LSB,
            machine: 0x28,     // ARM instruction set
            flags: 0x05000000, // ARM ABI version 5
        },
        cursor,
    )?;
    let sym_a = builder.add_symbol("A", &b"ay"[..]).unwrap();
    let sym_b = builder.add_symbol("B", &b"bee"[..]).unwrap();
    let sym_c = builder.add_symbol("C", &b"see"[..]).unwrap();
    assert_eq!(
        sym_a,
        Symbol {
            rodata_offset: 0,
            size: 2,
            padded_size: 4,
        }
    );
    assert_eq!(
        sym_b,
        Symbol {
            rodata_offset: 4,
            size: 3,
            padded_size: 4,
        }
    );
    assert_eq!(
        sym_c,
        Symbol {
            rodata_offset: 8,
            size: 3,
            padded_size: 4,
        }
    );

    let mut cursor = builder.close()?;
    cursor.seek(std::io::SeekFrom::Start(0))?;

    let ef = elf::File::open_stream(&mut cursor).unwrap();
    assert_eq!(
        ef.ehdr,
        elf::types::FileHeader {
            class: elf::types::ELFCLASS32,
            data: elf::types::ELFDATA2LSB,
            version: elf::types::Version(1),
            osabi: elf::types::ELFOSABI_NONE,
            abiversion: 0,
            elftype: elf::types::ET_REL,
            machine: elf::types::EM_ARM,
            entry: 0,
        }
    );
    assert_eq!(ef.phdrs.len(), 0, "no program headers");
    assert_eq!(ef.sections.len(), 5, "five section headers");
    let rodata = ef.get_section(".rodata").unwrap();
    let symtab = ef.get_section(".symtab").unwrap();
    let syms = ef.get_symbols(symtab).unwrap();
    assert_eq!(
        syms.len(),
        4,
        "three symbols in addition to the zero placeholder"
    );

    {
        // Placeholder symbol zero
        assert_eq!(syms[0].name, "");
        assert_eq!(syms[0].value, 0);
        assert_eq!(syms[0].size, 0);
    }
    {
        // Real symbol 1: A
        assert_eq!(syms[1].name, "A");
        assert_eq!(syms[1].value, 0);
        assert_eq!(syms[1].size, 2);
        let start_offset = syms[1].value as usize;
        let end_offset = start_offset + syms[1].size as usize;
        assert_eq!(&rodata.data[start_offset..end_offset], &b"ay"[..]);
    }
    {
        // Real symbol 2: B
        assert_eq!(syms[2].name, "B");
        assert_eq!(syms[2].value, 4);
        assert_eq!(syms[2].size, 3);
        let start_offset = syms[2].value as usize;
        let end_offset = start_offset + syms[2].size as usize;
        assert_eq!(&rodata.data[start_offset..end_offset], &b"bee"[..]);
    }
    {
        // Real symbol 3: C
        assert_eq!(syms[3].name, "C");
        assert_eq!(syms[3].value, 8);
        assert_eq!(syms[3].size, 3);
        let start_offset = syms[3].value as usize;
        let end_offset = start_offset + syms[3].size as usize;
        assert_eq!(&rodata.data[start_offset..end_offset], &b"see"[..]);
    }

    Ok(())
}

#[test]
fn no_symbols_le64() -> Result<()> {
    let buf: Vec<u8> = Vec::new();
    let cursor = Cursor::new(buf);
    let builder = Builder::new(
        Header {
            class: Class::ELF64,
            encoding: Encoding::LSB,
            machine: 0x28,     // ARM instruction set
            flags: 0x05000000, // ARM ABI version 5
        },
        cursor,
    )?;
    let mut cursor = builder.close()?;
    cursor.seek(std::io::SeekFrom::Start(0))?;

    let ef = elf::File::open_stream(&mut cursor).unwrap();
    assert_eq!(
        ef.ehdr,
        elf::types::FileHeader {
            class: elf::types::ELFCLASS64,
            data: elf::types::ELFDATA2LSB,
            version: elf::types::Version(1),
            osabi: elf::types::ELFOSABI_NONE,
            abiversion: 0,
            elftype: elf::types::ET_REL,
            machine: elf::types::EM_ARM,
            entry: 0,
        }
    );
    assert_eq!(ef.phdrs.len(), 0, "no program headers");
    assert_eq!(ef.sections.len(), 5, "five section headers");
    let symtab = ef.get_section(".symtab").unwrap();
    let syms = ef.get_symbols(symtab).unwrap();
    assert_eq!(syms.len(), 0, "no symbols");

    Ok(())
}

#[test]
fn three_symbols_le64() -> Result<()> {
    let buf: Vec<u8> = Vec::new();
    let cursor = Cursor::new(buf);
    let mut builder = Builder::new(
        Header {
            class: Class::ELF64,
            encoding: Encoding::LSB,
            machine: 0x28,     // ARM instruction set
            flags: 0x05000000, // ARM ABI version 5
        },
        cursor,
    )?;
    let sym_a = builder.add_symbol("A", &b"ay"[..]).unwrap();
    let sym_b = builder.add_symbol("B", &b"bee"[..]).unwrap();
    let sym_c = builder.add_symbol("C", &b"see"[..]).unwrap();
    assert_eq!(
        sym_a,
        Symbol {
            rodata_offset: 0,
            size: 2,
            padded_size: 8,
        }
    );
    assert_eq!(
        sym_b,
        Symbol {
            rodata_offset: 8,
            size: 3,
            padded_size: 8,
        }
    );
    assert_eq!(
        sym_c,
        Symbol {
            rodata_offset: 16,
            size: 3,
            padded_size: 8,
        }
    );

    let mut cursor = builder.close()?;
    cursor.seek(std::io::SeekFrom::Start(0))?;

    let ef = elf::File::open_stream(&mut cursor).unwrap();
    assert_eq!(
        ef.ehdr,
        elf::types::FileHeader {
            class: elf::types::ELFCLASS64,
            data: elf::types::ELFDATA2LSB,
            version: elf::types::Version(1),
            osabi: elf::types::ELFOSABI_NONE,
            abiversion: 0,
            elftype: elf::types::ET_REL,
            machine: elf::types::EM_ARM,
            entry: 0,
        }
    );
    assert_eq!(ef.phdrs.len(), 0, "no program headers");
    assert_eq!(ef.sections.len(), 5, "five section headers");
    let rodata = ef.get_section(".rodata").unwrap();
    let symtab = ef.get_section(".symtab").unwrap();
    let syms = ef.get_symbols(symtab).unwrap();
    assert_eq!(
        syms.len(),
        4,
        "three symbols in addition to the zero placeholder"
    );
    {
        // Placeholder symbol zero
        assert_eq!(syms[0].name, "");
        assert_eq!(syms[0].value, 0);
        assert_eq!(syms[0].size, 0);
    }
    {
        // Real symbol 1: A
        assert_eq!(syms[1].name, "A");
        assert_eq!(syms[1].value, 0);
        assert_eq!(syms[1].size, 2);
        let start_offset = syms[1].value as usize;
        let end_offset = start_offset + syms[1].size as usize;
        assert_eq!(&rodata.data[start_offset..end_offset], &b"ay"[..]);
    }
    {
        // Real symbol 2: B
        assert_eq!(syms[2].name, "B");
        assert_eq!(syms[2].value, 8);
        assert_eq!(syms[2].size, 3);
        let start_offset = syms[2].value as usize;
        let end_offset = start_offset + syms[2].size as usize;
        assert_eq!(&rodata.data[start_offset..end_offset], &b"bee"[..]);
    }
    {
        // Real symbol 3: C
        assert_eq!(syms[3].name, "C");
        assert_eq!(syms[3].value, 16);
        assert_eq!(syms[3].size, 3);
        let start_offset = syms[3].value as usize;
        let end_offset = start_offset + syms[3].size as usize;
        assert_eq!(&rodata.data[start_offset..end_offset], &b"see"[..]);
    }

    Ok(())
}
