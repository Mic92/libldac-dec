//! Convert internal float samples to output PCM formats.

use crate::consts::*;
use crate::sfinfo::SfInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    S16,
    S24,
    S32,
    F32,
}

impl SampleFormat {
    pub fn word_len(self) -> usize {
        match self {
            SampleFormat::S16 => 2,
            SampleFormat::S24 => 3,
            SampleFormat::S32 | SampleFormat::F32 => 4,
        }
    }
}

pub fn set_output_pcm(
    sf: &SfInfo,
    bufs: &mut [Vec<u8>; LDAC_MAXNCH],
    fmt: SampleFormat,
    nsmpl: usize,
) {
    for (ich, ac) in sf.ac.iter().enumerate() {
        let time = &ac.acsub.a_time;
        let out = &mut bufs[ich];
        match fmt {
            SampleFormat::S16 => {
                for i in 0..nsmpl {
                    let v = (time[i] + 0.5).floor() as i32;
                    let v = v.clamp(-0x8000, 0x7FFF) as i16;
                    out[i * 2..i * 2 + 2].copy_from_slice(&v.to_le_bytes());
                }
            }
            SampleFormat::S24 => {
                for i in 0..nsmpl {
                    let v = (time[i] * 256.0 + 0.5).floor() as i32;
                    let v = v.clamp(-0x80_0000, 0x7F_FFFF);
                    let b = v.to_le_bytes();
                    out[i * 3] = b[0];
                    out[i * 3 + 1] = b[1];
                    out[i * 3 + 2] = b[2];
                }
            }
            SampleFormat::S32 => {
                for i in 0..nsmpl {
                    let v = (time[i] as f64 * 65536.0 + 0.5).floor() as i64;
                    let v = v.clamp(-0x8000_0000, 0x7FFF_FFFF) as i32;
                    out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
                }
            }
            SampleFormat::F32 => {
                for i in 0..nsmpl {
                    let v = (time[i] / 32768.0).clamp(-1.0, 1.0);
                    out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
                }
            }
        }
    }
}
