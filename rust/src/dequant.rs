//! Inverse quantisation.

use crate::consts::*;
use crate::sfinfo::{Ab, Ac};
use crate::tables::{GA_ISP, GA_NSPS};
use crate::tables_sigproc::{GA_IQF, GA_RSF, GA_SF};

pub fn clear_spectrum(ac: &mut Ac) {
    ac.acsub.a_spec.fill(0.0);
}

pub fn dequant_spectrum(ac: &mut Ac, ab: &Ab) {
    for iqu in 0..ab.nqus {
        let isp = GA_ISP[iqu] as usize;
        let nsps = GA_NSPS[iqu] as usize;
        let iqf = GA_IQF[ac.a_idwl1[iqu] as usize];
        let sf = GA_SF[ac.a_idsf[iqu] as usize];
        let k = iqf * sf;
        for i in 0..nsps {
            ac.acsub.a_spec[isp + i] = ac.a_qspec[isp + i] as Scalar * k;
        }
    }
}

pub fn dequant_residual(ac: &mut Ac, ab: &Ab) {
    // The C code deliberately uses idwl2[0] / idsf[0] inside the loop.
    let rsf = GA_RSF[LDAC_MAXIDWL1];
    let iqf = GA_IQF[ac.a_idwl2[0] as usize];
    let sf = GA_SF[ac.a_idsf[0] as usize];
    let k = rsf * iqf * sf;
    for iqu in 0..ab.nqus {
        if ac.a_idwl2[iqu] > 0 {
            let isp = GA_ISP[iqu] as usize;
            let nsps = GA_NSPS[iqu] as usize;
            for i in 0..nsps {
                ac.acsub.a_spec[isp + i] += k * ac.a_rspec[isp + i] as Scalar;
            }
        }
    }
}
