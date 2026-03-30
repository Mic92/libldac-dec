//! Compile-time constants lifted from `ldac.h`.

#![allow(dead_code)]

pub type Scalar = f32;

pub const LDAC_SYNCWORD: u8 = 0xAA;
pub const LDAC_FRMHDRBYTES: usize = 3;

pub const LDAC_MAXNCH: usize = 2;
pub const LDAC_NSUPSMPLRATEID: usize = 4;
pub const LDAC_NCHCONFIGID: usize = 8;

pub const LDAC_CHCONFIGID_MN: usize = 0;
pub const LDAC_CHCONFIGID_DL: usize = 1;
pub const LDAC_CHCONFIGID_ST: usize = 2;

pub const LDAC_NFRAME: usize = 2;
pub const LDAC_MAXLNN: usize = 8;
pub const LDAC_1FSLNN: usize = 7;
pub const LDAC_2FSLNN: usize = 8;
pub const LDAC_MAXLSU: usize = 1 << LDAC_MAXLNN; // 256
pub const LDAC_1FSLSU: usize = 1 << LDAC_1FSLNN; // 128
pub const LDAC_2FSLSU: usize = 1 << LDAC_2FSLNN; // 256
pub const LDAC_NUMLNN: usize = 2;

pub const LDAC_MAXNBANDS: usize = 16;
pub const LDAC_MAXGRADQU: usize = 50;
pub const LDAC_MAXNQUS: usize = 34;

pub const LDAC_NSFCWTBL: usize = 8;
pub const LDAC_NIDSF: usize = 32;
pub const LDAC_NIDWL: usize = 16;

pub const LDAC_MAXIDWL1: usize = 15;
pub const LDAC_MAXIDWL2: usize = 15;

pub const LDAC_N2DIMSPECDECTBL: usize = 8;
pub const LDAC_N4DIMSPECDECTBL: usize = 81;

// Stream syntax
pub const LDAC_FILLCODE: u32 = 0x01;
pub const LDAC_BYTESIZE: usize = 8;
pub const LDAC_MAXBITNUM: usize = 8192;

pub const LDAC_NBANDBITS: u32 = 4;
pub const LDAC_BAND_OFFSET: i32 = 2;
pub const LDAC_FLAGBITS: u32 = 1;
pub const LDAC_GRADMODEBITS: u32 = 2;
pub const LDAC_GRADOSBITS: u32 = 5;
pub const LDAC_GRADQU0BITS: u32 = 6;
pub const LDAC_GRADQU1BITS: u32 = 5;
pub const LDAC_DEFGRADQUH: i32 = 26;
pub const LDAC_DEFGRADOSH: i32 = 31;
pub const LDAC_NADJQUBITS: u32 = 5;
pub const LDAC_SFCMODEBITS: u32 = 1;
pub const LDAC_SFCBLENBITS: u32 = 2;
pub const LDAC_SFCWTBLBITS: u32 = 3;
pub const LDAC_IDSFBITS: u32 = 5;
pub const LDAC_MINSFCBLEN_0: i32 = 3;
pub const LDAC_2DIMSPECBITS: u32 = 3;
pub const LDAC_4DIMSPECBITS: u32 = 7;

pub const LDAC_MODE_0: i32 = 0;
pub const LDAC_MODE_1: i32 = 1;
pub const LDAC_MODE_2: i32 = 2;
pub const LDAC_MODE_3: i32 = 3;

pub const LDAC_BLKID_MONO: i32 = 0;
pub const LDAC_BLKID_STEREO: i32 = 1;

// Bluetooth transport
pub const LDACBT_MAX_LSU: usize = 512;
pub const LDACBT_PCM_WLEN_MAX: usize = 4;
