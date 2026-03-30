//! Bit-allocation reconstruction.

use crate::consts::*;
use crate::sfinfo::{Ab, Ac};
use crate::tables::GAA_RESAMP_GRAD;

pub fn calc_add_word_length(ac: &mut Ac, ab: &Ab) {
    let nqus = ab.nqus;
    ac.a_addwl.fill(0);
    if ab.grad_mode == LDAC_MODE_0 {
        return;
    }
    for iqu in 1..nqus {
        let dif = ac.a_idsf[iqu] - ac.a_idsf[iqu - 1];
        if dif > 1 {
            ac.a_addwl[iqu] += (dif - 1).min(5);
        } else if dif < -1 {
            ac.a_addwl[iqu - 1] += (-dif - 1).min(5);
        }
    }
}

pub fn reconst_gradient(ab: &mut Ab, hqu: usize) {
    let gqu_l = ab.grad_qu_l as usize;
    let gqu_h = ab.grad_qu_h as usize;
    let gos_l = ab.grad_os_l;
    let gos_h = ab.grad_os_h;
    let p = &mut ab.a_grad;
    for i in 0..gqu_h {
        p[i] = -gos_l;
    }
    for i in gqu_h..hqu {
        p[i] = -gos_h;
    }
    let span = gqu_h as i32 - gqu_l as i32;
    if span > 0 {
        let t = &GAA_RESAMP_GRAD[span as usize - 1];
        let diff = gos_h - gos_l;
        if diff > 0 {
            let d = diff - 1;
            for (k, iqu) in (gqu_l..gqu_h).enumerate() {
                p[iqu] -= ((t[k] as i32 * d) >> 8) + 1;
            }
        } else if diff < 0 {
            let d = -diff - 1;
            for (k, iqu) in (gqu_l..gqu_h).enumerate() {
                p[iqu] += ((t[k] as i32 * d) >> 8) + 1;
            }
        }
    }
}

pub fn reconst_word_length(ac: &mut Ac, ab: &Ab) {
    let hqu = ab.nqus;
    let grad = &ab.a_grad;
    match ab.grad_mode {
        LDAC_MODE_0 => {
            for i in 0..hqu {
                ac.a_idwl1[i] = (ac.a_idsf[i] + grad[i]).max(1);
            }
        }
        LDAC_MODE_1 => {
            for i in 0..hqu {
                let mut v = ac.a_idsf[i] + grad[i] + ac.a_addwl[i];
                if v > 0 {
                    v >>= 1;
                }
                ac.a_idwl1[i] = v.max(1);
            }
        }
        LDAC_MODE_2 => {
            for i in 0..hqu {
                let mut v = ac.a_idsf[i] + grad[i] + ac.a_addwl[i];
                if v > 0 {
                    v = (v * 3) >> 3;
                }
                ac.a_idwl1[i] = v.max(1);
            }
        }
        LDAC_MODE_3 => {
            for i in 0..hqu {
                let mut v = ac.a_idsf[i] + grad[i] + ac.a_addwl[i];
                if v > 0 {
                    v >>= 2;
                }
                ac.a_idwl1[i] = v.max(1);
            }
        }
        _ => {}
    }
    for i in 0..ab.nadjqus as usize {
        ac.a_idwl1[i] += 1;
    }
    for i in 0..hqu {
        let w1 = ac.a_idwl1[i];
        if w1 > LDAC_MAXIDWL1 as i32 {
            ac.a_idwl2[i] = (w1 - LDAC_MAXIDWL1 as i32).min(LDAC_MAXIDWL2 as i32);
            ac.a_idwl1[i] = LDAC_MAXIDWL1 as i32;
        } else {
            ac.a_idwl2[i] = 0;
        }
    }
}
