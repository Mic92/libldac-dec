//! Decoder state: sound-frame / audio-block / audio-channel structures.
//!
//! Compared to the C version we collapse the pointer-heavy AB/AC graph
//! into owned, index-addressed Rust structs.  The parent-pointer from
//! `AC` back to `AB` becomes an index into `SfInfo::ab`.

use crate::consts::*;
use crate::tables::GAA_BLOCK_SETTING;

#[derive(Clone, Copy)]
pub struct Config {
    pub smplrate_id: usize,
    pub chconfig_id: usize,
    pub frame_length: usize,
    #[allow(dead_code)]
    pub frame_status: i32,
}

/// Per-channel spectral & time-domain buffers.
#[derive(Clone)]
pub struct AcSub {
    pub a_time: [Scalar; LDAC_MAXLSU * LDAC_NFRAME],
    pub a_spec: [Scalar; LDAC_MAXLSU],
}

impl Default for AcSub {
    fn default() -> Self {
        Self {
            a_time: [0.0; LDAC_MAXLSU * LDAC_NFRAME],
            a_spec: [0.0; LDAC_MAXLSU],
        }
    }
}

/// Audio-channel state.
#[derive(Clone)]
pub struct Ac {
    pub sfc_mode: i32,
    pub sfc_bitlen: i32,
    pub sfc_offset: i32,
    pub sfc_weight: usize,
    pub ext_size: i32,
    pub a_idsf: [i32; LDAC_MAXNQUS],
    pub a_idwl1: [i32; LDAC_MAXNQUS],
    pub a_idwl2: [i32; LDAC_MAXNQUS],
    pub a_addwl: [i32; LDAC_MAXNQUS],
    pub a_qspec: [i32; LDAC_MAXLSU],
    pub a_rspec: [i32; LDAC_MAXLSU],
    pub acsub: Box<AcSub>,
}

impl Ac {
    fn new() -> Self {
        Self {
            sfc_mode: 0,
            sfc_bitlen: 0,
            sfc_offset: 0,
            sfc_weight: 0,
            ext_size: 0,
            a_idsf: [0; LDAC_MAXNQUS],
            a_idwl1: [0; LDAC_MAXNQUS],
            a_idwl2: [0; LDAC_MAXNQUS],
            a_addwl: [0; LDAC_MAXNQUS],
            a_qspec: [0; LDAC_MAXLSU],
            a_rspec: [0; LDAC_MAXLSU],
            acsub: Box::default(),
        }
    }
}

/// Audio-block state.
#[derive(Clone)]
pub struct Ab {
    pub blk_nchs: usize,
    pub nbands: i32,
    pub nqus: usize,
    pub ext_flag: i32,
    pub ext_mode: i32,
    pub grad_mode: i32,
    pub grad_qu_l: i32,
    pub grad_qu_h: i32,
    pub grad_os_l: i32,
    pub grad_os_h: i32,
    pub a_grad: [i32; LDAC_MAXGRADQU],
    pub nadjqus: i32,
    /// Global indices into SfInfo::ac for each channel of this block.
    pub ac_idx: [usize; 2],
}

impl Ab {
    fn new(blk_type: i32) -> Self {
        let blk_nchs = match blk_type {
            LDAC_BLKID_MONO => 1,
            LDAC_BLKID_STEREO => 2,
            _ => 0,
        };
        Self {
            blk_nchs,
            nbands: 0,
            nqus: 0,
            ext_flag: 0,
            ext_mode: 0,
            grad_mode: 0,
            grad_qu_l: 0,
            grad_qu_h: 0,
            grad_os_l: 0,
            grad_os_h: 0,
            a_grad: [0; LDAC_MAXGRADQU],
            nadjqus: 0,
            ac_idx: [0; 2],
        }
    }
}

/// Sound-frame info: top-level decoder state.
pub struct SfInfo {
    pub cfg: Config,
    pub ab: Vec<Ab>,
    pub ac: Vec<Ac>,
}

impl SfInfo {
    pub fn new(
        smplrate_id: usize,
        chconfig_id: usize,
        frame_length: usize,
        frame_status: i32,
    ) -> Self {
        Self {
            cfg: Config {
                smplrate_id,
                chconfig_id,
                frame_length,
                frame_status,
            },
            ab: Vec::new(),
            ac: Vec::new(),
        }
    }

    /// Build the AB/AC topology from the current channel config.
    pub fn init_decode(&mut self) {
        let bs = &GAA_BLOCK_SETTING[self.cfg.chconfig_id];
        let nbks = bs[1] as usize;
        self.ab.clear();
        self.ac.clear();
        let mut ch_off = 0usize;
        for ibk in 0..nbks {
            let blk_type = bs[2 + ibk] as i32;
            let mut ab = Ab::new(blk_type);
            for ich in 0..ab.blk_nchs {
                ab.ac_idx[ich] = ch_off;
                self.ac.push(Ac::new());
                ch_off += 1;
            }
            self.ab.push(ab);
        }
    }
}
