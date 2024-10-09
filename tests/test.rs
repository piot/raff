use flood_rs::prelude::*;
use raff::prelude::*;

#[test]
fn test_raff_header() {
    #[rustfmt::skip]
    let data = &[
        0xF0, 0x9F, 0xA6, 0x8A, // Icon
        0x52, 0x41, 0x46, 0x46, // RAFF
        0x30, 0x2E, 0x31, 0x0A, // Version 0.1
    ];

    let mut stream = OctetRefReader::new(data);

    let header = read_raff_header(&mut stream).unwrap();
    assert_eq!(header.major, 0);
    assert_eq!(header.minor, 1);
}

#[test]
fn test_write_raff_header() {
    let mut stream = OutOctetStream::new();

    write_raff_header(&mut stream).expect("should write");

    #[rustfmt::skip]
    let expected_output = &[
        0xF0, 0x9F, 0xA6, 0x8A, // Icon
        0x52, 0x41, 0x46, 0x46, // RAFF
        0x30, 0x2E, 0x31, 0x0A, // Version 0.1
    ];

    assert_eq!(stream.octets(), expected_output);

    let empty = &[0xff; 0x53];
    write_chunk(&mut stream, "xb".into(), empty).expect("Failed to write empty");

    let mut stream = InOctetStream::new(&stream.octets());
    RaffHeader::deserialize(&mut stream).expect("Failed to read header");
    let header = read_chunk_header(&mut stream).expect("Failed to read chunk header");
    assert_eq!(header.tag.name, "xb".into());
    assert_eq!(header.size, 0x53);
    let mut data = vec![0u8; header.size as usize];
    stream.read(&mut data).unwrap();
    assert_eq!(data, empty);
    assert!(stream.has_reached_end());
}

#[test]
fn is_valid_char() {
    assert!(is_valid_tag_char(b'a'));
    assert!(is_valid_tag_char(b'z'));
    assert!(is_valid_tag_char(b'A'));
    assert!(is_valid_tag_char(b'Z'));
    assert!(is_valid_tag_char(b'0'));
    assert!(is_valid_tag_char(b'9'));
    assert!(is_valid_tag_char(b'_'));
}
