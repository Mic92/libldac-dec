//! CLI driver mirroring `ldac_dec_test.c`.
//!
//! Reads `<name>_enc_{HQ,SQ,MQ}.bin`, decodes it using parameters from
//! `<name>.wav`, and writes `<name>_dec_{HQ,SQ,MQ}.wav`.

use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

use ldac_dec::{ChannelMode, LdacDecoder, SampleFormat};

fn read_le_u16(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}
fn read_le_u32(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

struct WavInfo {
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    format_tag: u16,
}

fn parse_wav_header(path: &Path) -> io::Result<WavInfo> {
    let mut f = fs::File::open(path)?;
    let mut hdr = [0u8; 44];
    f.read_exact(&mut hdr)?;
    Ok(WavInfo {
        format_tag: read_le_u16(&hdr[20..]),
        channels: read_le_u16(&hdr[22..]),
        sample_rate: read_le_u32(&hdr[24..]),
        bits_per_sample: read_le_u16(&hdr[34..]),
    })
}

fn write_wav(path: &Path, info: &WavInfo, pcm: &[u8]) -> io::Result<()> {
    let mut f = fs::File::create(path)?;
    let bps = info.bits_per_sample as u32 / 8;
    let byte_rate = info.sample_rate * bps * info.channels as u32;
    let block_align = (bps * info.channels as u32) as u16;
    f.write_all(b"RIFF")?;
    f.write_all(&((pcm.len() as u32 + 36).to_le_bytes()))?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&info.format_tag.to_le_bytes())?;
    f.write_all(&info.channels.to_le_bytes())?;
    f.write_all(&info.sample_rate.to_le_bytes())?;
    f.write_all(&byte_rate.to_le_bytes())?;
    f.write_all(&block_align.to_le_bytes())?;
    f.write_all(&info.bits_per_sample.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&(pcm.len() as u32).to_le_bytes())?;
    f.write_all(pcm)?;
    Ok(())
}

fn main() -> io::Result<()> {
    let name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "512kMeasSweep_0_to_24000_-12_dBV_48k_PCM16_LR".into());
    let base = Path::new("..");

    let wav_path = base.join(format!("{name}.wav"));
    let info = parse_wav_header(&wav_path)?;

    let fmt = match (info.format_tag, info.bits_per_sample) {
        (3, _) => SampleFormat::F32,
        (_, 16) => SampleFormat::S16,
        (_, 24) => SampleFormat::S24,
        (_, 32) => SampleFormat::S32,
        _ => panic!("unsupported sample format"),
    };
    let cm = if info.channels == 2 {
        ChannelMode::Stereo
    } else {
        ChannelMode::Mono
    };

    for (suffix, _eqmid) in [("HQ", 0), ("SQ", 1), ("MQ", 2)] {
        let enc = base.join(format!("{name}_enc_{suffix}.bin"));
        let dec = base.join(format!("{name}_dec_{suffix}.wav"));
        if !enc.exists() || dec.exists() {
            continue;
        }

        let stream = fs::read(&enc)?;
        let mut decoder =
            LdacDecoder::new(cm, info.sample_rate).map_err(|e| io::Error::other(e.to_string()))?;

        let frame_bytes =
            decoder.frame_samples() as usize * info.channels as usize * fmt.word_len();
        let mut pcm = Vec::<u8>::new();
        let mut scratch = vec![0u8; frame_bytes];

        let mut off = 0usize;
        let mut frames = 0u64;
        let t0 = std::time::Instant::now();
        while off + 5 < stream.len() {
            match decoder.decode(&stream[off..], fmt, &mut scratch) {
                Ok((used, wrote)) => {
                    pcm.extend_from_slice(&scratch[..wrote]);
                    off += used;
                    frames += 1;
                }
                Err(e) => {
                    eprintln!("decode error at offset {off}: {e}");
                    break;
                }
            }
        }
        let dt = t0.elapsed();
        eprintln!(
            "{suffix}: decoded {frames} frames in {:?} ({:.2} us/frame)",
            dt,
            dt.as_secs_f64() * 1e6 / frames.max(1) as f64
        );

        write_wav(&dec, &info, &pcm)?;
        break; // match the C driver: process one quality mode per run
    }

    Ok(())
}
