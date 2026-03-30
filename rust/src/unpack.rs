//! Bitstream parsing.

use crate::bitalloc::{calc_add_word_length, reconst_gradient, reconst_word_length};
use crate::bitreader::BitReader;
use crate::consts::*;
use crate::error::{LdacError, LdacResult};
use crate::sfinfo::{Ac, SfInfo};
use crate::tables::*;

pub struct FrameHeader {
    pub smplrate_id: usize,
    pub chconfig_id: usize,
    pub frame_length: usize,
    pub frame_status: i32,
}

pub fn parse_frame_header(stream: &[u8]) -> Option<FrameHeader> {
    if stream.first().copied() != Some(LDAC_SYNCWORD) {
        return None;
    }
    let mut br = BitReader::new(stream);
    br.read(8); // syncword
    let smplrate_id = br.read(3) as usize;
    let chconfig_id = br.read(2) as usize;
    let frame_length = br.read(9) as usize + 1;
    let frame_status = br.read(2) as i32;
    Some(FrameHeader {
        smplrate_id,
        chconfig_id,
        frame_length,
        frame_status,
    })
}

fn unpack_scale_factor_0(ac: &mut Ac, nqus: usize, br: &mut BitReader) {
    ac.sfc_bitlen = br.read(LDAC_SFCBLENBITS) as i32 + LDAC_MINSFCBLEN_0;
    ac.sfc_offset = br.read(LDAC_IDSFBITS) as i32;
    ac.sfc_weight = br.read(LDAC_SFCWTBLBITS) as usize;
    ac.a_idsf[0] = br.read(ac.sfc_bitlen as u32) as i32;

    let h = &GA_HCDEC_SF0[(ac.sfc_bitlen - LDAC_MINSFCBLEN_0) as usize];
    let wtbl = &GAA_SFCWGT[ac.sfc_weight];

    for iqu in 1..nqus {
        let raw = br.read(h.maxlen as u32) as usize;
        let sym = h.dec[raw] as usize;
        br.rewind((h.maxlen - h.tbl[sym].len) as u32);
        ac.a_idsf[iqu] = (sym as i32 + ac.a_idsf[iqu - 1]) & h.mask as i32;
        ac.a_idsf[iqu - 1] += ac.sfc_offset - wtbl[iqu - 1] as i32;
    }
    ac.a_idsf[nqus - 1] += ac.sfc_offset - wtbl[nqus - 1] as i32;
}

fn sign_extend(v: u32, bits: u32) -> i32 {
    let shift = 32 - bits;
    ((v << shift) as i32) >> shift
}

pub fn unpack_raw_data_frame(sf: &mut SfInfo, stream: &[u8]) -> LdacResult<usize> {
    let nbytes_frame = sf.cfg.frame_length;
    let smplrate_id = sf.cfg.smplrate_id;
    let mut br = BitReader::new(stream);

    // Borrow the AB and AC vectors disjointly for the whole frame.
    let SfInfo {
        ab: abs, ac: acs, ..
    } = sf;

    for ab in abs.iter_mut() {
        ab.nbands = br.read(LDAC_NBANDBITS) as i32 + LDAC_BAND_OFFSET;
        if ab.nbands as u8 > GA_MAX_NBANDS[smplrate_id] {
            return Err(LdacError::SyntaxBand);
        }
        ab.nqus = GA_NQUS[ab.nbands as usize] as usize;

        ab.ext_flag = br.read(LDAC_FLAGBITS) as i32;
        if ab.ext_flag != 0 {
            ab.ext_mode = br.read(2) as i32;
            if ab.ext_mode == 3 {
                for ich in 0..ab.blk_nchs {
                    let ac = &mut acs[ab.ac_idx[ich]];
                    ac.ext_size = br.read(7) as i32;
                    let mut ext = ac.ext_size;
                    while ext > 0 {
                        br.read(16);
                        ext -= 16;
                    }
                }
            }
        }

        ab.grad_mode = br.read(LDAC_GRADMODEBITS) as i32;
        if ab.grad_mode == LDAC_MODE_0 {
            ab.grad_qu_l = br.read(LDAC_GRADQU0BITS) as i32;
            if ab.grad_qu_l >= LDAC_MAXGRADQU as i32 {
                return Err(LdacError::SyntaxGradA);
            }
            ab.grad_qu_h = br.read(LDAC_GRADQU0BITS) as i32 + 1;
            if ab.grad_qu_h >= LDAC_MAXGRADQU as i32 {
                return Err(LdacError::SyntaxGradB);
            }
            if ab.grad_qu_h < ab.grad_qu_l {
                return Err(LdacError::SyntaxGradC);
            }
            ab.grad_os_l = br.read(LDAC_GRADOSBITS) as i32;
            ab.grad_os_h = br.read(LDAC_GRADOSBITS) as i32;
        } else {
            ab.grad_qu_l = br.read(LDAC_GRADQU1BITS) as i32;
            if ab.grad_qu_l > LDAC_DEFGRADQUH {
                return Err(LdacError::SyntaxGradD);
            }
            ab.grad_os_l = br.read(LDAC_GRADOSBITS) as i32;
            ab.grad_qu_h = LDAC_DEFGRADQUH;
            ab.grad_os_h = LDAC_DEFGRADOSH;
        }
        ab.nadjqus = br.read(LDAC_NADJQUBITS) as i32;
        if ab.nqus < ab.nadjqus as usize {
            return Err(LdacError::SyntaxGradE);
        }

        let nqus = ab.nqus;
        reconst_gradient(ab, nqus);

        for ich in 0..ab.blk_nchs {
            let gidx = ab.ac_idx[ich];
            // Split to obtain a mutable ref to current channel while
            // keeping shared access to channel 0 (needed by sfc_mode==1).
            let (lo, hi) = acs.split_at_mut(gidx);
            let ac = &mut hi[0];
            let ch0: Option<&Ac> = if ich > 0 {
                Some(&lo[ab.ac_idx[0]])
            } else {
                None
            };

            ac.a_idsf.fill(0);
            ac.sfc_mode = br.read(LDAC_SFCMODEBITS) as i32;
            let mode = ac.sfc_mode;

            if ich > 0 && mode != 0 {
                // Differential against channel 0.
                ac.sfc_bitlen = br.read(LDAC_SFCBLENBITS) as i32;
                let h = &GA_HCDEC_SF1[ac.sfc_bitlen as usize];
                ac.sfc_bitlen += 2;
                let blen = ac.sfc_bitlen as u32;
                let ch0 = ch0.unwrap();
                for iqu in 0..nqus {
                    let raw = br.read(h.maxlen as u32) as usize;
                    let sym = h.dec[raw] as u32;
                    br.rewind((h.maxlen - h.tbl[sym as usize].len) as u32);
                    let dif = sign_extend(sym, blen);
                    ac.a_idsf[iqu] = (ch0.a_idsf[iqu] + dif) & 0x1F;
                }
            } else if ich == 0 && mode != 0 {
                ac.sfc_bitlen = br.read(LDAC_SFCBLENBITS) as i32 + 2;
                if ac.sfc_bitlen > 4 {
                    for iqu in 0..nqus {
                        ac.a_idsf[iqu] = br.read(ac.sfc_bitlen as u32) as i32;
                    }
                } else {
                    ac.sfc_offset = br.read(LDAC_IDSFBITS) as i32;
                    ac.sfc_weight = br.read(LDAC_SFCWTBLBITS) as usize;
                    let w = &GAA_SFCWGT[ac.sfc_weight];
                    for iqu in 0..nqus {
                        ac.a_idsf[iqu] =
                            br.read(ac.sfc_bitlen as u32) as i32 + ac.sfc_offset - w[iqu] as i32;
                    }
                }
            } else {
                unpack_scale_factor_0(ac, nqus, &mut br);
            }

            if ac.a_idsf.iter().any(|&v| v > 31) {
                return Err(LdacError::SyntaxIdsf);
            }

            calc_add_word_length(ac, ab);
            reconst_word_length(ac, ab);

            // Quantised spectrum
            ac.a_qspec.fill(0);
            for iqu in 0..nqus {
                let idwl1 = ac.a_idwl1[iqu] as usize;
                let lsp = GA_ISP[iqu] as usize;
                let hsp = GA_ISP[iqu + 1] as usize;
                if idwl1 == 1 {
                    let nsps = GA_NSPS[iqu] as usize;
                    if nsps == 2 {
                        let idx = br.read(LDAC_2DIMSPECBITS) as usize;
                        ac.a_qspec[lsp] = GAA_2DIMDEC_SPEC[idx][0] as i32;
                        ac.a_qspec[lsp + 1] = GAA_2DIMDEC_SPEC[idx][1] as i32;
                    } else {
                        let groups = nsps >> 2;
                        for g in 0..groups {
                            let idx = br.read(LDAC_4DIMSPECBITS) as usize;
                            if idx > 80 {
                                return Err(LdacError::SyntaxSpec);
                            }
                            let b = lsp + g * 4;
                            for k in 0..4 {
                                ac.a_qspec[b + k] = GAA_4DIMDEC_SPEC[idx][k] as i32;
                            }
                        }
                    }
                } else {
                    let wl = GA_WL[idwl1] as u32;
                    for isp in lsp..hsp {
                        ac.a_qspec[isp] = sign_extend(br.read(wl), wl);
                    }
                }
            }

            // Residual spectrum
            ac.a_rspec.fill(0);
            for iqu in 0..nqus {
                let idwl2 = ac.a_idwl2[iqu] as usize;
                if idwl2 > 0 {
                    let lsp = GA_ISP[iqu] as usize;
                    let hsp = GA_ISP[iqu + 1] as usize;
                    let wl = GA_WL[idwl2] as u32;
                    for isp in lsp..hsp {
                        ac.a_rspec[isp] = sign_extend(br.read(wl), wl);
                    }
                }
            }
        }

        // Byte-align padding
        if br.pos() > LDAC_BYTESIZE * nbytes_frame {
            return Err(LdacError::FrameLengthOver);
        }
        let pad = (LDAC_BYTESIZE - (br.pos() % LDAC_BYTESIZE)) % LDAC_BYTESIZE;
        if pad > 0 && br.read(pad as u32) != 0 {
            return Err(LdacError::UnpackBlockAlign);
        }
    }

    // Fill code
    let filled = nbytes_frame - br.pos() / LDAC_BYTESIZE;
    for _ in 0..filled {
        if br.read(LDAC_BYTESIZE as u32) != LDAC_FILLCODE {
            return Err(LdacError::UnpackFrameAlign);
        }
    }

    Ok(br.pos() / LDAC_BYTESIZE)
}
