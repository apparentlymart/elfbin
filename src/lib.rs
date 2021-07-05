use binbin::endian::Endian;
use std::io::{Read, Result, Seek, Write};

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u8)]
pub enum Class {
    ELF32 = 1,
    ELF64 = 2,
}

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u8)]
pub enum Encoding {
    LSB = 1,
    MSB = 2,
}

pub struct Header {
    pub class: Class,
    pub encoding: Encoding,
    pub machine: u16,
    pub flags: u32,
}

pub struct Builder<W: Write + Seek> {
    w: W,
    class: Class,
    encoding: Encoding,
    headmap: HeaderMap,
    rodata_pos: u64,
    current_rodata_offset: u64,
    symbols: Vec<Symbol>,
    symbol_names: Vec<String>,
}

impl<W> Builder<W>
where
    W: Write + Seek,
{
    pub fn new(hdr: Header, mut target: W) -> Result<Self> {
        let mut headmap = HeaderMap {
            section_header_offset_field: 0,
        };
        match hdr.encoding {
            Encoding::LSB => binbin::write_le(&mut target, |mut w| match hdr.class {
                Class::ELF32 => {
                    headmap = write_hdr_32(&hdr, &mut w)?;
                    Ok(())
                }
                Class::ELF64 => {
                    headmap = write_hdr_64(&hdr, &mut w)?;
                    Ok(())
                }
            }),
            Encoding::MSB => binbin::write_be(&mut target, |mut w| match hdr.class {
                Class::ELF32 => {
                    headmap = write_hdr_32(&hdr, &mut w)?;
                    Ok(())
                }
                Class::ELF64 => {
                    headmap = write_hdr_64(&hdr, &mut w)?;
                    Ok(())
                }
            }),
        }?;

        let rodata_pos = target.stream_position()?;

        Ok(Self {
            w: target,
            class: hdr.class,
            encoding: hdr.encoding,
            headmap: headmap,
            rodata_pos: rodata_pos,
            current_rodata_offset: 0,
            symbols: Vec::new(),
            symbol_names: Vec::new(),
        })
    }

    pub fn add_symbol<S: Into<String>, R: Read>(&mut self, name: S, src: R) -> Result<Symbol> {
        let offset = self.current_rodata_offset;
        let length: u64;
        let stride: u64;

        let encoding = self.encoding;
        let class = self.class;
        let result = match encoding {
            Encoding::LSB => binbin::write_le(&mut self.w, |w| match class {
                Class::ELF32 => write_symbol_data(src, w, 4),
                Class::ELF64 => write_symbol_data(src, w, 8),
            }),
            Encoding::MSB => binbin::write_be(&mut self.w, |w| match class {
                Class::ELF32 => write_symbol_data(src, w, 4),
                Class::ELF64 => write_symbol_data(src, w, 8),
            }),
        }?;
        length = result.0;
        stride = result.1;

        let sym = Symbol {
            rodata_offset: offset,
            size: length,
            padded_size: stride,
        };
        self.symbols.push(sym);
        self.symbol_names.push(name.into());
        self.current_rodata_offset += stride;
        Ok(sym)
    }

    pub fn close(mut self) -> Result<W> {
        let encoding = self.encoding;
        let class = self.class;
        let sym_names = self.symbol_names;
        let syms = self.symbols;
        let rodata_pos = self.rodata_pos;

        let map = match encoding {
            Encoding::LSB => binbin::write_le(&mut self.w, |w| match class {
                Class::ELF32 => write_metadata_sections_32(rodata_pos, &sym_names, &syms, w),
                Class::ELF64 => write_metadata_sections_64(rodata_pos, &sym_names, &syms, w),
            }),
            Encoding::MSB => binbin::write_be(&mut self.w, |w| match class {
                Class::ELF32 => write_metadata_sections_32(rodata_pos, &sym_names, &syms, w),
                Class::ELF64 => write_metadata_sections_64(rodata_pos, &sym_names, &syms, w),
            }),
        }?;

        let final_pos = self.w.stream_position()?;
        self.w.seek(std::io::SeekFrom::Start(
            self.headmap.section_header_offset_field,
        ))?;
        let section_header_pos = map.section_header_offset;
        match encoding {
            Encoding::LSB => binbin::write_le(&mut self.w, |w| match class {
                Class::ELF32 => w.write(section_header_pos as u32).map(|_| ()),
                Class::ELF64 => w.write(section_header_pos as u64).map(|_| ()),
            }),
            Encoding::MSB => binbin::write_be(&mut self.w, |w| match class {
                Class::ELF32 => w.write(section_header_pos as u32).map(|_| ()),
                Class::ELF64 => w.write(section_header_pos as u64).map(|_| ()),
            }),
        }?;
        self.w.seek(std::io::SeekFrom::Start(final_pos))?;

        Ok(self.w)
    }
}

fn write_hdr_32<'a, W: Write + Seek, E: Endian>(
    hdr: &Header,
    w: &mut binbin::Writer<'a, W, E>,
) -> Result<HeaderMap> {
    write_ident(hdr, w)?;
    w.write(ET_REL)?;
    w.write(hdr.machine)?;
    w.write(1 as u32)?; // header version
    w.write(0 as u32)?; // entry point (none)
    w.write(0 as u32)?; // no program headers
    let shoff_pos = w.position()?;
    w.write(0 as u32)?; // placeholder for section header offset
    w.write(hdr.flags)?;
    let header_size_dfr = w.write_deferred(0 as u16)?;
    w.write(0 as u16)?; // no program header entries
    w.write(0 as u16)?; // no program header entries
    w.write(40 as u16)?; // section header entry size
    w.write(5 as u16)?; // section header entry count
    w.write(1 as u16)?; // section names are in section 1

    let pos = w.position()? as u16;
    w.resolve(header_size_dfr, pos)?;

    w.align(4)?;

    Ok(HeaderMap {
        section_header_offset_field: shoff_pos,
    })
}

fn write_hdr_64<'a, W: Write + Seek, E: Endian>(
    hdr: &Header,
    w: &mut binbin::Writer<'a, W, E>,
) -> Result<HeaderMap> {
    write_ident(hdr, w)?;
    w.write(ET_REL)?;
    w.write(hdr.machine)?;
    w.write(1 as u32)?; // header version
    w.write(0 as u64)?; // entry point (none)
    w.write(0 as u64)?; // no program headers
    let shoff_pos = w.position()?;
    w.write(0 as u64)?; // placeholder for section header offset
    w.write(hdr.flags)?;
    let header_size_dfr = w.write_deferred(0 as u16)?;
    w.write(0 as u16)?; // no program header entries
    w.write(0 as u16)?; // no program header entries
    w.write(64 as u16)?; // section header entry size
    w.write(5 as u16)?; // section header entry count
    w.write(1 as u16)?; // section names are in section 1

    let pos = w.position()? as u16;
    w.resolve(header_size_dfr, pos)?;

    w.align(8)?;

    Ok(HeaderMap {
        section_header_offset_field: shoff_pos,
    })
}

fn write_symbol_data<'a, R: Read, W: Write + Seek, E: Endian>(
    mut src: R,
    w: &mut binbin::Writer<'a, W, E>,
    align: usize,
) -> Result<(u64, u64)> {
    let len = std::io::copy(&mut src, w)?;
    let extra = w.align(align)?;
    Ok((len, len + (extra as u64)))
}

fn write_metadata_sections_32<'a, W: Write + Seek, E: Endian>(
    rodata_pos: u64,
    sym_names: &Vec<String>,
    syms: &Vec<Symbol>,
    w: &mut binbin::Writer<'a, W, E>,
) -> Result<TrailerMap> {
    // At the point we're called, our position is at the end of the
    // .rodata section body and we've not produced any other sections
    // yet. We'll first produce all of the other section bodies and
    // then finally write out the section header containing offsets
    // back to these body positions.
    const ALIGN: usize = 4;

    // .shstrtab is a hard-coded string table of the four section names
    // we always generate. This must be the first entry in the section
    // header table below, because our ELF header points to it there.
    w.align(ALIGN)?;
    let shstrtab_start = w.position()?;
    w.write(SHSTRTAB)?;
    let shstrtab_len = w.position()? - shstrtab_start;

    // .strtab is the table of our symbol names.
    w.align(ALIGN)?;
    let strtab_start = w.position()?;
    w.write(0 as u8)?; // string tables always start with a null
    let mut symbol_name_idx: Vec<u32> = Vec::with_capacity(syms.len());
    {
        let mut idx: usize = 1;

        for name in sym_names.iter() {
            symbol_name_idx.push(idx as u32);
            w.write(name.as_bytes())?;
            w.write(0 as u8)?; // null terminator
            idx += name.len() + 1;
        }
    }
    let strtab_len = w.position()? - strtab_start;

    // .symtab is the table of the symbols themselves
    w.align(ALIGN)?;
    let symtab_start = w.position()?;
    let mut rodata_size: u64 = 0;
    if syms.len() > 0 {
        // Symbol zero is a null symbol required by the ELF format
        write_symbol_32(
            w,
            Symbol32 {
                name_idx: 0,
                value: 0,
                size: 0,
                info: 0,
                other: 0,
                section_idx: 0,
            },
        )?;
        for (i, sym) in syms.iter().enumerate() {
            write_symbol_32(
                w,
                Symbol32 {
                    name_idx: symbol_name_idx[i],
                    value: sym.rodata_offset as u32,
                    size: sym.size as u32,
                    info: (1 << 4) | 1 as u8, // (STB_GLOBAL, STT_OBJECT)
                    other: 0,
                    section_idx: 2, // .rodata
                },
            )?;
            rodata_size += sym.padded_size;
        }
    }
    let symtab_len = w.position()? - symtab_start;

    // Now we'll write out the section headers. .shstrtab must be index 1
    // and .rodata must be index 2 due to references we've left elsewhere
    // in the file to those indices.
    w.align(ALIGN)?;
    let section_header_pos = w.position()?;
    {
        // Unused header index zero, as required by the ELF standard
        write_section_header_32(
            w,
            SectionHeader32 {
                name_idx: 0,
                typ: SHT_NULL,
                flags: 0,
                addr: 0,
                offset: 0,
                size: 0,
                link: 0,
                info: 0,
                addralign: 0,
                entsize: 0,
            },
        )?;
    }
    {
        // .shstrtab (section header names table)
        write_section_header_32(
            w,
            SectionHeader32 {
                name_idx: SHSTRTAB_SHSTRTAB,
                typ: SHT_STRTAB,
                flags: SHF_STRINGS,
                addr: 0,
                offset: shstrtab_start as u32,
                size: shstrtab_len as u32,
                link: 0,
                info: 0,
                addralign: 0,
                entsize: 1, // one byte per character
            },
        )?;
    }
    {
        // .rodata (the actual symbol contents)
        write_section_header_32(
            w,
            SectionHeader32 {
                name_idx: SHSTRTAB_RODATA,
                typ: SHT_PROGBITS,
                flags: SHF_ALLOC,
                addr: 0, // linker will decide final addr
                offset: rodata_pos as u32,
                size: rodata_size as u32,
                link: 0,
                info: 0,
                addralign: ALIGN as u32,
                entsize: 0,
            },
        )?;
    }
    {
        // .strtab (the symbol names table)
        write_section_header_32(
            w,
            SectionHeader32 {
                name_idx: SHSTRTAB_STRTAB,
                typ: SHT_STRTAB,
                flags: SHF_STRINGS,
                addr: 0,
                offset: strtab_start as u32,
                size: strtab_len as u32,
                link: 0,
                info: 0,
                addralign: 0,
                entsize: 1, // one byte per character
            },
        )?;
    }
    {
        // .symtab (the symbol table itself)
        write_section_header_32(
            w,
            SectionHeader32 {
                name_idx: SHSTRTAB_SYMTAB,
                typ: SHT_SYMTAB,
                flags: 0,
                addr: 0,
                offset: symtab_start as u32,
                size: symtab_len as u32,
                link: 3,      // symbol names are in section 3 (.strtab)
                info: 1,      // symbol 1 is the first global symbol
                addralign: 0, // no alignment requirements
                entsize: 16,
            },
        )?;
    }

    Ok(TrailerMap {
        section_header_offset: section_header_pos,
    })
}

fn write_metadata_sections_64<'a, W: Write + Seek, E: Endian>(
    rodata_pos: u64,
    sym_names: &Vec<String>,
    syms: &Vec<Symbol>,
    w: &mut binbin::Writer<'a, W, E>,
) -> Result<TrailerMap> {
    // At the point we're called, our position is at the end of the
    // .rodata section body and we've not produced any other sections
    // yet. We'll first produce all of the other section bodies and
    // then finally write out the section header containing offsets
    // back to these body positions.
    const ALIGN: usize = 8;

    // .shstrtab is a hard-coded string table of the four section names
    // we always generate. This must be the first entry in the section
    // header table below, because our ELF header points to it there.
    w.align(ALIGN)?;
    let shstrtab_start = w.position()?;
    w.write(SHSTRTAB)?;
    let shstrtab_len = w.position()? - shstrtab_start;

    // .strtab is the table of our symbol names.
    w.align(ALIGN)?;
    let strtab_start = w.position()?;
    w.write(0 as u8)?; // string tables always start with a null
    let mut symbol_name_idx: Vec<u32> = Vec::with_capacity(syms.len());
    {
        let mut idx: usize = 1;

        for name in sym_names.iter() {
            symbol_name_idx.push(idx as u32);
            w.write(name.as_bytes())?;
            w.write(0 as u8)?; // null terminator
            idx += name.len() + 1;
        }
    }
    let strtab_len = w.position()? - strtab_start;

    // .symtab is the table of the symbols themselves
    w.align(ALIGN)?;
    let symtab_start = w.position()?;
    let mut rodata_size: u64 = 0;
    if syms.len() > 0 {
        // Symbol zero is a null symbol required by the ELF format
        write_symbol_64(
            w,
            Symbol64 {
                name_idx: 0,
                value: 0,
                size: 0,
                info: 0,
                other: 0,
                section_idx: 0,
            },
        )?;
        for (i, v) in syms.iter().enumerate() {
            write_symbol_64(
                w,
                Symbol64 {
                    name_idx: symbol_name_idx[i],
                    value: v.rodata_offset,
                    size: v.size,
                    info: (1 << 4) | 1 as u8, // (STB_GLOBAL, STT_OBJECT)
                    other: 0,
                    section_idx: 2, // .rodata
                },
            )?;
            rodata_size += v.padded_size;
        }
    }
    let symtab_len = w.position()? - symtab_start;

    // Now we'll write out the section headers. .shstrtab must be index 1
    // and .rodata must be index 2 due to references we've left elsewhere
    // in the file to those indices.
    w.align(ALIGN)?;
    let section_header_pos = w.position()?;
    {
        // Unused header index zero, as required by the ELF standard
        write_section_header_64(
            w,
            SectionHeader64 {
                name_idx: 0,
                typ: SHT_NULL,
                flags: 0,
                addr: 0,
                offset: 0,
                size: 0,
                link: 0,
                info: 0,
                addralign: 0,
                entsize: 0,
            },
        )?;
    }
    {
        // .shstrtab (section header names table)
        write_section_header_64(
            w,
            SectionHeader64 {
                name_idx: SHSTRTAB_SHSTRTAB,
                typ: SHT_STRTAB,
                flags: SHF_STRINGS as u64,
                addr: 0,
                offset: shstrtab_start,
                size: shstrtab_len,
                link: 0,
                info: 0,
                addralign: 0,
                entsize: 1, // one byte per character
            },
        )?;
    }
    {
        // .rodata (the actual symbol contents)
        write_section_header_64(
            w,
            SectionHeader64 {
                name_idx: SHSTRTAB_RODATA,
                typ: SHT_PROGBITS,
                flags: SHF_ALLOC as u64,
                addr: 0, // linker will decide final addr
                offset: rodata_pos,
                size: rodata_size,
                link: 0,
                info: 0,
                addralign: ALIGN as u64,
                entsize: 0,
            },
        )?;
    }
    {
        // .strtab (the symbol names table)
        write_section_header_64(
            w,
            SectionHeader64 {
                name_idx: SHSTRTAB_STRTAB,
                typ: SHT_STRTAB,
                flags: SHF_STRINGS as u64,
                addr: 0,
                offset: strtab_start,
                size: strtab_len,
                link: 0,
                info: 0,
                addralign: 0,
                entsize: 1, // one byte per character
            },
        )?;
    }
    {
        // .symtab (the symbol table itself)
        write_section_header_64(
            w,
            SectionHeader64 {
                name_idx: SHSTRTAB_SYMTAB,
                typ: SHT_SYMTAB,
                flags: 0,
                addr: 0,
                offset: symtab_start,
                size: symtab_len,
                link: 3,      // symbol names are in section 3 (.strtab)
                info: 1,      // symbol 1 is the first global symbol
                addralign: 0, // no alignment requirements
                entsize: 24,
            },
        )?;
    }

    Ok(TrailerMap {
        section_header_offset: section_header_pos,
    })
}

fn write_section_header_32<'a, W: Write + Seek, E: Endian>(
    w: &mut binbin::Writer<'a, W, E>,
    hdr: SectionHeader32,
) -> Result<()> {
    w.write(hdr.name_idx)?; // index into .shstrtab
    w.write(hdr.typ)?; // type
    w.write(hdr.flags)?; // no flags
    w.write(hdr.addr)?; // no addr
    w.write(hdr.offset)?; // offset
    w.write(hdr.size)?; // size
    w.write(hdr.link)?; // symbol names are in section 3 (.strtab)
    w.write(hdr.info)?; // symbol 1 is the first global symbol
    w.write(hdr.addralign)?; // no alignment
    w.write(hdr.entsize)?; // no alignment
    Ok(())
}

fn write_section_header_64<'a, W: Write + Seek, E: Endian>(
    w: &mut binbin::Writer<'a, W, E>,
    hdr: SectionHeader64,
) -> Result<()> {
    w.write(hdr.name_idx)?; // index into .shstrtab
    w.write(hdr.typ)?; // type
    w.write(hdr.flags)?; // no flags
    w.write(hdr.addr)?; // no addr
    w.write(hdr.offset)?; // offset
    w.write(hdr.size)?; // size
    w.write(hdr.link)?; // symbol names are in section 3 (.strtab)
    w.write(hdr.info)?; // symbol 1 is the first global symbol
    w.write(hdr.addralign)?; // no alignment
    w.write(hdr.entsize)?; // no alignment
    Ok(())
}

fn write_symbol_32<'a, W: Write + Seek, E: Endian>(
    w: &mut binbin::Writer<'a, W, E>,
    sym: Symbol32,
) -> Result<()> {
    w.write(sym.name_idx)?; // index into .strtab
    w.write(sym.value)?;
    w.write(sym.size)?;
    w.write(sym.info)?;
    w.write(sym.other)?;
    w.write(sym.section_idx)?;
    Ok(())
}

fn write_symbol_64<'a, W: Write + Seek, E: Endian>(
    w: &mut binbin::Writer<'a, W, E>,
    sym: Symbol64,
) -> Result<()> {
    w.write(sym.name_idx)?; // index into .strtab
    w.write(sym.info)?;
    w.write(sym.other)?;
    w.write(sym.section_idx)?;
    w.write(sym.value)?;
    w.write(sym.size)?;
    Ok(())
}

fn write_ident<'a, W: Write + Seek, E: Endian>(
    hdr: &Header,
    w: &mut binbin::Writer<'a, W, E>,
) -> Result<()> {
    // e_ident bytes
    w.write(&b"\x7fELF"[..])?;
    w.write(hdr.class as u8)?;
    w.write(hdr.encoding as u8)?;
    w.write(1 as u8)?; // file version 1
    w.write(0 as u8)?; // no particular ABI
    w.skip(8)?; // unused ident bytes
    Ok(())
}

struct HeaderMap {
    section_header_offset_field: u64,
}

struct TrailerMap {
    section_header_offset: u64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Symbol {
    rodata_offset: u64,
    size: u64,
    padded_size: u64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct SectionHeader32 {
    name_idx: u32,
    typ: u32,
    flags: u32,
    addr: u32,
    offset: u32,
    size: u32,
    link: u32,
    info: u32,
    addralign: u32,
    entsize: u32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct SectionHeader64 {
    name_idx: u32,
    typ: u32,
    flags: u64,
    addr: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    addralign: u64,
    entsize: u64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct Symbol32 {
    name_idx: u32,
    value: u32,
    size: u32,
    info: u8,
    other: u8,
    section_idx: u16,
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct Symbol64 {
    name_idx: u32,
    info: u8,
    other: u8,
    section_idx: u16,
    value: u64,
    size: u64,
}

const ET_REL: u16 = 1;

const SHT_NULL: u32 = 0;
const SHT_PROGBITS: u32 = 1;
const SHT_SYMTAB: u32 = 2;
const SHT_STRTAB: u32 = 3;
const SHF_ALLOC: u32 = 0x2;
const SHF_STRINGS: u32 = 0x20;

const SHSTRTAB: &'static [u8] = b"\x00.shstrtab\x00.strtab\x00.symtab\x00.rodata\x00";
const SHSTRTAB_SHSTRTAB: u32 = 1;
const SHSTRTAB_STRTAB: u32 = 11;
const SHSTRTAB_SYMTAB: u32 = 19;
const SHSTRTAB_RODATA: u32 = 27;

#[cfg(test)]
mod tests;
