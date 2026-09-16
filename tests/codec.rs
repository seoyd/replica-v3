use replica_v3::{codec::*, event::*};
fn sample() -> Event {
    let mut e = Event::observation("s", "t", "u", Vec::new());
    e.id = 1;
    e
}
#[test]
fn literal_golden_and_exact_bytes() {
    // Literal schema fields and independently calculated bitwise CRC32C.
    const GOLDEN: &[u8] = &[
        82, 80, 86, 51, 1, 0, 0, 0, 13, 0, 0, 0, 13, 0, 0, 0, 46, 98, 208, 165, 1, 0, 0, 1, 117, 1,
        115, 1, 116, 0, 0, 0, 0,
    ];
    let e = sample();
    assert_eq!(decode(GOLDEN).unwrap(), e);
    assert_eq!(encode(&e, Compression::Raw).unwrap(), GOLDEN);
    for text in ["", "\0\n\"한글 한글 😀\n ", "안녕하세요"] {
        let mut e = sample();
        e.payload = text.as_bytes().to_vec();
        e.recorded_at = i64::MIN;
        e.observed_at = Some(i64::MAX);
        for c in [Compression::Raw, Compression::Auto] {
            let b = encode(&e, c).unwrap();
            assert_eq!(encode(&e, c).unwrap(), b);
            assert_eq!(decode(&b).unwrap(), e);
        }
    }
    let mut e = sample();
    e.payload = "오른쪽\n".repeat(4000).into_bytes();
    let raw = encode(&e, Compression::Raw).unwrap();
    let zstd = encode(&e, Compression::Auto).unwrap();
    assert_eq!(zstd[6], 1);
    assert!(zstd.len() < raw.len());
    assert_eq!(decode(&zstd).unwrap(), e);
}
#[test]
fn rejects_corruption_lengths_tags_and_varints() {
    for n in [0, 1, 127, 128, u32::MAX as u64, i64::MAX as u64, u64::MAX] {
        let mut b = Vec::new();
        put_varint(&mut b, n);
        assert_eq!(decode_varint(&b).unwrap(), (n, b.len()));
    }
    for b in [&[0x80][..], &[0x80, 0], &[0xff; 10], &[0x81, 0], &[]] {
        assert!(decode_varint(b).is_err());
    }
    let good = encode(&sample(), Compression::Raw).unwrap();
    for end in 0..good.len() {
        assert!(decode(&good[..end]).is_err());
    }
    for offset in [0, 4, 6, 7, 8, 12, 16] {
        let mut bad = good.clone();
        bad[offset] = 255;
        assert!(decode(&bad).is_err(), "offset {offset}");
    }
    let mut bad = good.clone();
    bad[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode(&bad).is_err());
    let mut bad = good.clone();
    bad[good.len() - 2] = 255;
    let checksum = crc32c::crc32c(&bad[HEADER..]);
    bad[16..20].copy_from_slice(&checksum.to_le_bytes());
    assert!(decode(&bad).is_err());
    let mut bad = good;
    bad.push(0);
    assert!(decode(&bad).is_err());
    let mut e = sample();
    e.payload = vec![b'a'; MAX_PAYLOAD];
    let compressed = encode(&e, Compression::Auto).unwrap();
    let mut bad = compressed.clone();
    bad[8..12].copy_from_slice(&1u32.to_le_bytes());
    assert!(decode(&bad).is_err());
    let mut extra = compressed.clone();
    extra.extend(zstd::bulk::compress(b"", 3).unwrap());
    let stored = (extra.len() - HEADER) as u32;
    extra[12..16].copy_from_slice(&stored.to_le_bytes());
    assert!(decode(&extra).is_err());
    let mut bad = compressed;
    bad[HEADER] = 0;
    assert!(decode(&bad).is_err());
}
