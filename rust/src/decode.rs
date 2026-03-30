//! Top-level per-frame decode dispatch.

use crate::dequant::{clear_spectrum, dequant_residual, dequant_spectrum};
use crate::sfinfo::SfInfo;

pub fn decode(sf: &mut SfInfo) {
    // We need shared access to AB while mutating AC.  Since `unpack`
    // already populated the AB state, iterate by index and clone the
    // tiny immutable pieces we need.
    for ibk in 0..sf.ab.len() {
        let nchs = sf.ab[ibk].blk_nchs;
        for ich in 0..nchs {
            let gidx = sf.ab[ibk].ac_idx[ich];
            // Snapshot the AB fields dequant needs (cheap copy).
            let ab = sf.ab[ibk].clone();
            let ac = &mut sf.ac[gidx];
            clear_spectrum(ac);
            dequant_spectrum(ac, &ab);
            dequant_residual(ac, &ab);
        }
    }
}
