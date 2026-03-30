//! Inverse MDCT with overlap-add.

use crate::consts::*;
use crate::sfinfo::SfInfo;
use crate::tables_sigproc::{bwin, rev_perm, wcos, wsin};

fn imdct_core(spec: &mut [Scalar], time: &mut [Scalar], nlnn: usize) {
    let nsmpl = 1usize << nlnn;
    let half = nsmpl >> 1;
    let p_w = bwin(nlnn);
    let p_c = wcos(nlnn);
    let p_s = wsin(nlnn);
    let p_rp = rev_perm(nlnn);

    let mut work = [0.0 as Scalar; LDAC_MAXLSU];

    // Stage 0: bit-reverse permute & first butterfly.
    let cc = p_c[0];
    let cs = p_s[0];
    let q = nsmpl >> 2;
    for g in 0..q {
        let i = g * 4;
        let f1 = spec[p_rp[i] as usize];
        let f2 = spec[p_rp[i + 1] as usize];
        let f3 = spec[p_rp[i + 2] as usize];
        let f4 = spec[p_rp[i + 3] as usize];
        let a = f1 * cc + f2 * cs;
        let b = f1 * cs - f2 * cc;
        work[i] = a + f3;
        work[i + 1] = b + f4;
        work[i + 2] = f3 - a;
        work[i + 3] = f4 - b;
    }

    // Intermediate butterfly stages.
    let mut coef = 1usize;
    for stage in 1..(nlnn - 1) {
        let loop1 = nsmpl >> (stage + 2);
        let loop2 = 1usize << stage;
        let offset = 1usize << (stage + 2);
        let mut idx0 = 0usize;
        let mut idx1 = 1usize << (stage + 1);
        for _ in 0..loop2 {
            let cc = p_c[coef];
            let cs = p_s[coef];
            coef += 1;
            for _ in 0..loop1 {
                let a = work[idx0];
                let b = work[idx0 + 1];
                let c = work[idx1] * cc + work[idx1 + 1] * cs;
                let d = work[idx1] * cs - work[idx1 + 1] * cc;
                work[idx0] = a + c;
                work[idx0 + 1] = b + d;
                work[idx1] = a - c;
                work[idx1 + 1] = b - d;
                idx0 += offset;
                idx1 += offset;
            }
            idx0 = idx0.wrapping_add(2).wrapping_sub(nsmpl);
            idx1 = idx1.wrapping_add(2).wrapping_sub(nsmpl);
        }
    }

    // Post-rotate into spec.
    for i in 0..half {
        let s = p_s[coef + i];
        let c = p_c[coef + i];
        spec[2 * i] = s * work[2 * i + 1] + c * work[2 * i];
        spec[nsmpl - 2 * i - 1] = s * work[2 * i] - c * work[2 * i + 1];
    }

    // Overlap-add window.
    for i in 0..half {
        let y0 = spec[half + i];
        let y1 = spec[nsmpl - 1 - i];
        let y2 = spec[half - 1 - i];
        let y3 = spec[i];

        let t0 = y0 * p_w[i] - p_w[nsmpl - 1 - i] * time[nsmpl + i];
        let t1 = -p_w[half - 1 - i] * time[nsmpl + half + i] - y1 * p_w[half + i];

        time[i] = t0;
        time[half + i] = t1;
        time[nsmpl + i] = y2;
        time[nsmpl + half + i] = y3;
    }
}

pub fn proc_imdct(sf: &mut SfInfo, nlnn: usize) {
    for ac in sf.ac.iter_mut() {
        let sub = &mut *ac.acsub;
        imdct_core(&mut sub.a_spec, &mut sub.a_time, nlnn);
    }
}
