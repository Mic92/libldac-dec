use ldac_dec::{ChannelMode, LdacDecoder, SampleFormat};

#[test]
fn decoder_construction() {
    for sr in [44_100, 48_000, 88_200, 96_000] {
        let d = LdacDecoder::new(ChannelMode::Stereo, sr).unwrap();
        assert_eq!(d.sample_rate(), sr);
        assert_eq!(d.channels(), 2);
        let expected_samples = if sr <= 48_000 { 128 } else { 256 };
        assert_eq!(d.frame_samples(), expected_samples);
    }
    assert!(LdacDecoder::new(ChannelMode::Stereo, 22_050).is_err());
}

#[test]
fn bad_syncword_rejected() {
    let mut d = LdacDecoder::new(ChannelMode::Stereo, 48_000).unwrap();
    let mut out = [0u8; 1024];
    let r = d.decode(&[0x00; 16], SampleFormat::S16, &mut out);
    assert!(r.is_err());
}

#[test]
fn short_input_rejected() {
    let mut d = LdacDecoder::new(ChannelMode::Stereo, 48_000).unwrap();
    let mut out = [0u8; 1024];
    assert!(d
        .decode(&[0xAA, 0, 0], SampleFormat::S16, &mut out)
        .is_err());
}
