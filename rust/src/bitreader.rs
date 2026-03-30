//! Bit-level reader over an immutable `&[u8]`.
//!
//! Matches the behaviour of `read_unpack_ldac` from the C code: reads up
//! to 16 bits at a time from a big-endian bitstream, returning zero once
//! past [`LDAC_MAXBITNUM`].  Reads use safe slice indexing; a two-byte
//! look-ahead past the stated frame length is expected (the caller must
//! pad the buffer accordingly, matching the C API contract).

use crate::consts::LDAC_MAXBITNUM;

pub struct BitReader<'a> {
    buf: &'a [u8],
    loc: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, loc: 0 }
    }

    /// Read `nbits` (1..=16) from the stream.
    #[inline]
    pub fn read(&mut self, nbits: u32) -> u32 {
        debug_assert!(nbits <= 16);
        let bpos = (self.loc & 7) as u32;
        let byte = self.loc >> 3;
        let value = if self.loc < LDAC_MAXBITNUM {
            // 3-byte big-endian window.  Missing bytes beyond the buffer
            // are treated as zero so the reader never panics on the
            // two-byte look-ahead documented in the C API.
            let b0 = *self.buf.get(byte).unwrap_or(&0) as u32;
            let b1 = *self.buf.get(byte + 1).unwrap_or(&0) as u32;
            let b2 = *self.buf.get(byte + 2).unwrap_or(&0) as u32;
            let tmp = (b0 << 16) | (b1 << 8) | b2;
            ((tmp << bpos) & 0x00FF_FFFF) >> (24 - nbits)
        } else {
            0
        };
        self.loc += nbits as usize;
        value
    }

    /// Undo `nbits` of advance (used after Huffman look-ahead).
    #[inline]
    pub fn rewind(&mut self, nbits: u32) {
        self.loc -= nbits as usize;
    }

    /// Current bit position.
    #[inline]
    pub fn pos(&self) -> usize {
        self.loc
    }
}
