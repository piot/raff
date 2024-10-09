use flood_rs::prelude::*;
use std::io;
use std::str::FromStr;
pub mod prelude;

#[derive(Debug, Eq, PartialEq)]
pub struct Tag(u16);

pub fn is_valid_tag_char(c: u8) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'_'
}

impl Tag {
    pub fn with_str(s: &str) -> io::Result<Self> {
        if s.len() != 2 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Tag must be exactly 2 characters long.",
            ));
        }

        let bytes = s.as_bytes();

        if !is_valid_tag_char(bytes[0]) || !is_valid_tag_char(bytes[1]) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid characters in tag.",
            ));
        }

        Ok(Tag((bytes[0] as u16) * 256 + bytes[1] as u16))
    }

    pub fn new(v: u16) -> io::Result<Self> {
        let first = (v % 0x100) as u8;
        let second = ((v / 0x100) % 0x100) as u8;

        if !is_valid_tag_char(first) || !is_valid_tag_char(second) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid characters in tag.",
            ));
        }
        Ok(Tag(v))
    }

    pub fn inner(&self) -> u16 {
        self.0
    }
}

impl FromStr for Tag {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Tag::with_str(s)
    }
}

impl From<&str> for Tag {
    fn from(value: &str) -> Self {
        Tag::from_str(value).unwrap()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct TagHeader {
    pub name: Tag,
}

impl TagHeader {
    pub const fn new(name: Tag) -> Self {
        Self { name }
    }
}

impl Serialize for TagHeader {
    fn serialize(&self, stream: &mut impl flood_rs::WriteOctetStream) -> std::io::Result<()> {
        stream.write_u16(self.name.0)
    }
}

impl Deserialize for TagHeader {
    fn deserialize(stream: &mut impl flood_rs::ReadOctetStream) -> std::io::Result<Self> {
        Ok(Self {
            name: Tag::new(stream.read_u16()?)?,
        })
    }
}

// Function to decode the size from a compact format
fn decode_size(stream: &mut impl flood_rs::ReadOctetStream) -> std::io::Result<u32> {
    let mut size = 0u32;
    let mut shift = 0;

    loop {
        let octet = stream.read_u8()?;

        size |= ((octet & 0x7F) as u32) << shift;
        shift += 7;

        if shift > 28 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Size exceeds u32 maximum value",
            ));
        }

        if octet & 0x80 == 0 {
            break;
        }
    }

    Ok(size)
}

// Function to encode the size in a compact format
fn encode_size(size: u32) -> Vec<u8> {
    let mut encoded = Vec::new();
    let mut current = size;

    loop {
        let octet = (current & 0x7F) as u8;
        current >>= 7;

        if current > 0 {
            encoded.push(octet | 0x80);
        } else {
            encoded.push(octet);
            break;
        }
    }

    encoded
}

#[derive(Debug, Eq, PartialEq)]
pub struct ChunkHeader {
    pub tag: TagHeader,
    pub size: u32,
}

impl ChunkHeader {
    pub fn new(tag: Tag, size: u32) -> Self {
        Self {
            tag: TagHeader::new(tag),
            size,
        }
    }
}

impl Serialize for ChunkHeader {
    fn serialize(&self, stream: &mut impl flood_rs::WriteOctetStream) -> std::io::Result<()> {
        self.tag.serialize(stream)?;
        stream.write(encode_size(self.size).as_slice())
    }
}

impl Deserialize for ChunkHeader {
    fn deserialize(stream: &mut impl flood_rs::ReadOctetStream) -> std::io::Result<Self> {
        Ok(Self {
            tag: TagHeader::deserialize(stream)?,
            size: decode_size(stream)?,
        })
    }
}

#[derive(Debug, Eq, PartialEq, Default)]
pub struct RaffHeader {
    pub major: u8,
    pub minor: u8,
}

pub const RAFF_TEXT: u32 = 0x52414646;
pub const RAFF_ICON: u32 = 0xF09FA68A;
pub const RAFF_MAJOR: u8 = 0x00;
pub const RAFF_MINOR: u8 = 0x01;

impl RaffHeader {
    pub const fn new() -> Self {
        Self {
            major: RAFF_MAJOR,
            minor: RAFF_MINOR,
        }
    }

    pub const fn with_version(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }
}

pub fn to_version(data: u8) -> u8 {
    if !(48..=57).contains(&data) {
        return 0;
    }
    data - 48
}

pub fn from_version(data: u8) -> u8 {
    if data > 9 {
        return 0;
    }
    data + 48
}

impl Deserialize for RaffHeader {
    fn deserialize(stream: &mut impl flood_rs::ReadOctetStream) -> std::io::Result<Self> {
        let icon = stream.read_u32()?;
        if icon != RAFF_ICON {
            // fox
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid icon",
            ));
        }

        let raff_text = stream.read_u32()?;
        if raff_text != RAFF_TEXT {
            // "RAFF"
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid RAFF text",
            ));
        }

        let mut version_buf = [0u8; 4]; // e.g. "0.1\n"
        stream.read(&mut version_buf)?;

        if version_buf[1] != b'.' || version_buf[3] != b'\n' {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid version",
            ));
        }

        Ok(Self {
            major: to_version(version_buf[0]),
            minor: to_version(version_buf[2]),
        })
    }
}

impl Serialize for RaffHeader {
    fn serialize(&self, stream: &mut impl flood_rs::WriteOctetStream) -> std::io::Result<()> {
        stream.write_u32(RAFF_ICON)?;
        stream.write_u32(RAFF_TEXT)?;
        stream.write_u8(from_version(self.major))?;
        stream.write_u8(b'.')?;
        stream.write_u8(from_version(self.minor))?;
        stream.write_u8(0x0A)?;
        Ok(())
    }
}

pub fn write_chunk(
    stream: &mut impl flood_rs::WriteOctetStream,
    tag: Tag,
    data: &[u8],
) -> io::Result<()> {
    let header = ChunkHeader::new(tag, data.len() as u32);
    header.serialize(stream)?;
    stream.write(data)
}

pub fn read_raff_header(stream: &mut impl flood_rs::ReadOctetStream) -> io::Result<RaffHeader> {
    let header = RaffHeader::deserialize(stream)?;
    if header.major != RAFF_MAJOR || header.minor != RAFF_MINOR {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid RAFF header",
        ));
    }
    Ok(header)
}

pub fn write_raff_header(stream: &mut impl flood_rs::WriteOctetStream) -> io::Result<()> {
    let header = RaffHeader::new();
    header.serialize(stream)
}

pub fn read_chunk_header(stream: &mut impl flood_rs::ReadOctetStream) -> io::Result<ChunkHeader> {
    ChunkHeader::deserialize(stream)
}
