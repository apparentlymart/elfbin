use binbin::endian::Endian;
use std::collections::HashMap;
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
    class: Class,
    encoding: Encoding,
    machine: u16,
    flags: u32,
}

pub struct Builder<W: Write + Seek> {
    w: W,
    class: Class,
    encoding: Encoding,
    headmap: HeaderMap,
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
        Ok(Self {
            w: target,
            class: hdr.class,
            encoding: hdr.encoding,
            headmap: headmap,
        })
    }

    pub fn close(mut self) -> Result<W> {
        // TODO: write out all of the other sections
        binbin::write_le(&mut self.w, |w| w.align(8).map(|_| ()))?;
        let section_header_pos = self.w.stream_position()?;
        // TODO: write out the section header

        let final_pos = self.w.stream_position()?;
        self.w.seek(std::io::SeekFrom::Start(
            self.headmap.section_header_offset_field,
        ))?;
        let encoding = self.encoding;
        let class = self.class;
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
    w.write(40 as u16)?; // section header entry size
    w.write(5 as u16)?; // section header entry count
    w.write(1 as u16)?; // section names are in section 1

    let pos = w.position()? as u16;
    w.resolve(header_size_dfr, pos)?;

    Ok(HeaderMap {
        section_header_offset_field: shoff_pos,
    })
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

const ET_REL: u16 = 0;

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
