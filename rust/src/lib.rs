//! Memory-safe Rust implementation of the LDAC Bluetooth audio decoder.
//!
//! LDAC is a lossy audio codec developed by Sony for high-resolution
//! audio streaming over Bluetooth (A2DP).
//!
//! This is a port of the C reference decoder to safe Rust; no `unsafe`
//! is used in the core decoding path.

#![forbid(unsafe_code)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::needless_range_loop)]

mod consts;
mod error;
mod tables;
mod tables_sigproc;

mod bitalloc;
mod bitreader;
mod decode;
mod dequant;
mod imdct;
mod setpcm;
mod sfinfo;
mod unpack;

pub use error::{LdacError, LdacResult};
pub use setpcm::SampleFormat;

use consts::*;
use sfinfo::SfInfo;

/// Channel mode as specified by the Bluetooth A2DP LDAC profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChannelMode {
    Mono = 0x04,
    DualChannel = 0x02,
    Stereo = 0x01,
}

impl ChannelMode {
    fn to_cci(self) -> usize {
        match self {
            ChannelMode::Stereo => LDAC_CHCONFIGID_ST,
            ChannelMode::DualChannel => LDAC_CHCONFIGID_DL,
            ChannelMode::Mono => LDAC_CHCONFIGID_MN,
        }
    }
}

/// LDAC decoder.
///
/// Create with [`LdacDecoder::new`], then feed transport frames to
/// [`LdacDecoder::decode`].  Each transport frame begins with the 3-byte
/// LDAC frame header followed by the raw data frame.
pub struct LdacDecoder {
    sfinfo: SfInfo,
    nlnn: usize,
    // Per-channel intermediate PCM buffers (planar).
    pcm_buf: [Vec<u8>; LDAC_MAXNCH],
    // Cached state extracted from the transport header so callers can
    // query it between frames.
    sample_rate: u32,
    channels: u8,
    frm_samples: u32,
    bitrate: u32,
}

impl LdacDecoder {
    /// Create a new decoder for the given channel mode and sampling rate.
    ///
    /// `sample_rate` must be one of 44 100, 48 000, 88 200 or 96 000 Hz.
    pub fn new(cm: ChannelMode, sample_rate: u32) -> LdacResult<Self> {
        let sfid = match sample_rate {
            44_100 => 0,
            48_000 => 1,
            88_200 => 2,
            96_000 => 3,
            _ => return Err(LdacError::IllSamplingFreq),
        };
        let cci = cm.to_cci();
        let nlnn = tables::GA_LN_FRAMESMPLS[sfid] as usize;
        let frm_samples = tables::GA_FRAMESMPLS[sfid] as u32;
        let channels = tables::GA_CH[cci];

        let mut sfinfo = SfInfo::new(sfid, cci, 165 * channels as usize - LDAC_FRMHDRBYTES, 0);
        sfinfo.init_decode();

        Ok(Self {
            sfinfo,
            nlnn,
            pcm_buf: [
                vec![0u8; LDACBT_MAX_LSU * LDACBT_PCM_WLEN_MAX],
                vec![0u8; LDACBT_MAX_LSU * LDACBT_PCM_WLEN_MAX],
            ],
            sample_rate,
            channels,
            frm_samples,
            bitrate: 0,
        })
    }

    /// Number of audio channels the decoder is configured for.
    pub fn channels(&self) -> u8 {
        self.channels
    }

    /// Configured sampling rate in Hz.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Number of PCM samples produced per frame per channel.
    pub fn frame_samples(&self) -> u32 {
        self.frm_samples
    }

    /// Bit-rate of the most recently decoded transport frame in kbit/s.
    pub fn bitrate(&self) -> u32 {
        self.bitrate
    }

    /// Decode one LDAC transport frame.
    ///
    /// * `stream` – input bitstream, starting with the 3-byte LDAC frame
    ///   header.  The caller should ensure a few bytes of slack past the
    ///   frame end (`frame_length + 2`) to allow for look-ahead reads.
    /// * `fmt` – desired output PCM sample format.
    /// * `pcm_out` – interleaved PCM output buffer.  Must hold at least
    ///   `frame_samples * channels * bytes_per_sample` bytes.
    ///
    /// Returns `(bytes_consumed, bytes_written)`.
    pub fn decode(
        &mut self,
        stream: &[u8],
        fmt: SampleFormat,
        pcm_out: &mut [u8],
    ) -> LdacResult<(usize, usize)> {
        if stream.len() <= 4 {
            return Err(LdacError::InputBufferSize);
        }
        // Parse transport frame header.
        let hdr = unpack::parse_frame_header(stream).ok_or(LdacError::IllSyncword)?;

        if hdr.smplrate_id > 3 {
            return Err(LdacError::AssertSupSamplingFreq);
        }
        if hdr.chconfig_id > 2 {
            return Err(LdacError::AssertChannelConfig);
        }

        // If the stream config diverges from the current decoder state we
        // re-initialise (matches C behaviour: DEC_CONFIG_UPDATED).
        if hdr.smplrate_id != self.sfinfo.cfg.smplrate_id
            || hdr.chconfig_id != self.sfinfo.cfg.chconfig_id
        {
            let sr = tables::GA_SMPLRATE[hdr.smplrate_id];
            let cm = match hdr.chconfig_id {
                LDAC_CHCONFIGID_ST => ChannelMode::Stereo,
                LDAC_CHCONFIGID_DL => ChannelMode::DualChannel,
                _ => ChannelMode::Mono,
            };
            *self = Self::new(cm, sr)?;
        }

        self.sfinfo.cfg.frame_length = hdr.frame_length;
        self.sfinfo.cfg.frame_status = hdr.frame_status;

        let raw = &stream[LDAC_FRMHDRBYTES..];
        if raw.len() < hdr.frame_length {
            return Err(LdacError::InputBufferSize);
        }

        let used = unpack::unpack_raw_data_frame(&mut self.sfinfo, raw)?;
        decode::decode(&mut self.sfinfo);
        imdct::proc_imdct(&mut self.sfinfo, self.nlnn);

        let nsmpl = 1usize << self.nlnn;
        setpcm::set_output_pcm(&self.sfinfo, &mut self.pcm_buf, fmt, nsmpl);

        let total_used = used + LDAC_FRMHDRBYTES;
        self.bitrate = (total_used as u32 * self.sample_rate / self.frm_samples) / (1000 / 8);

        let wrote = interleave_pcm(pcm_out, &self.pcm_buf, nsmpl, self.channels as usize, fmt);

        Ok((total_used, wrote))
    }
}

/// Interleave planar per-channel PCM buffers into a single interleaved
/// output buffer.  Returns number of bytes written.
fn interleave_pcm(
    out: &mut [u8],
    planar: &[Vec<u8>; LDAC_MAXNCH],
    nsmpl: usize,
    nch: usize,
    fmt: SampleFormat,
) -> usize {
    if nsmpl == 0 {
        return 0;
    }
    let wl = fmt.word_len();
    let total = nsmpl * nch * wl;
    assert!(out.len() >= total, "output PCM buffer too small");
    match nch {
        1 => {
            out[..total].copy_from_slice(&planar[0][..total]);
        }
        2 => {
            let l = &planar[0];
            let r = &planar[1];
            for i in 0..nsmpl {
                let so = i * wl;
                let do0 = (i * 2) * wl;
                let do1 = do0 + wl;
                out[do0..do0 + wl].copy_from_slice(&l[so..so + wl]);
                out[do1..do1 + wl].copy_from_slice(&r[so..so + wl]);
            }
        }
        _ => return 0,
    }
    total
}
