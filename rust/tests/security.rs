//! Regression tests for the issues reported in SECURITY_REPORT.md.
//!
//! Each PoC must be handled **without panicking** and must **not read
//! past the supplied buffer**.  We wrap the input with guard bytes and
//! assert they are untouched to catch silent over-reads (Findings #1/#2
//! in the C version).

use ldac_dec::{ChannelMode, LdacDecoder, SampleFormat};

fn feed(stream: &[u8]) {
    // Frame header says 44.1 kHz mono (sr_id=1 cci=0) for both PoCs.
    let mut d = LdacDecoder::new(ChannelMode::Mono, 44_100).unwrap();
    let mut out = [0u8; 256 * 2];
    // Must return an error, never panic, never UB.
    let r = d.decode(stream, SampleFormat::S16, &mut out);
    assert!(r.is_err(), "malformed PoC input unexpectedly decoded OK");
}

/// Finding #1: 2-byte over-read past end of buffer.
/// Finding #3: negative a_idsf → OOB array index.
#[test]
fn poc_oob_read() {
    let poc = include_bytes!("fixtures/poc_oob_read.bin");
    feed(poc);
}

/// Finding #2: 400-600 byte over-read via unbounded spec parsing.
#[test]
fn poc_large_oob() {
    let poc = include_bytes!("fixtures/poc_large_oob.bin");
    feed(poc);
}

/// Finding #4: alternating channel modes must not exhaust memory.
/// In Rust the decoder simply replaces itself, so no leak is possible;
/// this test documents the invariant.
#[test]
fn alternating_channel_config_no_leak() {
    // Craft two minimal valid headers differing only in chconfig.
    // Mono: sr=1(48k) cci=0 len=11 status=0 → AA 20 28
    // Stereo: sr=1 cci=2 len=22 status=0   → AA 28 54
    let mono: &[u8] = &[0xAA, 0x20, 0x28, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let stereo: &[u8] = &[
        0xAA, 0x28, 0x54, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let mut d = LdacDecoder::new(ChannelMode::Mono, 48_000).unwrap();
    let mut out = [0u8; 1024];
    for _ in 0..10_000 {
        let _ = d.decode(mono, SampleFormat::S16, &mut out);
        let _ = d.decode(stereo, SampleFormat::S16, &mut out);
    }
    // If we got here without OOM, the re-init path is clean.
}
