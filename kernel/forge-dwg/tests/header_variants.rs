//! Header sections as other writers lay them out, read as the variables they hold. LibreCAD's
//! DWG writer gives an R2010+ header's high size word whatever its maintenance number (0), counted
//! in the section's size; made here from the ODA converter's own R2013 header (tests/data).
use forge_dwg::file;
use forge_dwg::header::read_header;

#[test]
fn a_high_size_word_given_at_maintenance_0_is_read_past() {
    let buf = std::fs::read(format!("{}/tests/data/mix_2013.dwg", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let s = file::sections(&buf).unwrap();
    assert!(s.maint > 3, "a file of a release that gives the word");
    let want = read_header(&s.header, s.ver, s.maint).unwrap();
    let sec = &s.header;
    let size = u32::from_le_bytes(sec[16..20].try_into().unwrap()) as usize;
    assert_eq!(&sec[20..24], &[0, 0, 0, 0], "the high word");
    // the same bytes, the size now counting the word, the CRC over them again
    let mut other = sec[..24 + size].to_vec();
    other[16..20].copy_from_slice(&((size + 4) as u32).to_le_bytes());
    let crc = forge_dwg::bits::crc16(0xC0C1, &other[16..]);
    other.extend_from_slice(&crc.to_le_bytes());
    other.extend_from_slice(&sec[24 + size + 2..]);
    let got = read_header(&other, s.ver, 0).unwrap();
    assert_eq!(got, want);
}
