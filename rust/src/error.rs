//! Decoder error codes.

use std::fmt;

pub type LdacResult<T> = Result<T, LdacError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LdacError {
    // Syntax / block level
    SyntaxBand,
    SyntaxGradA,
    SyntaxGradB,
    SyntaxGradC,
    SyntaxGradD,
    SyntaxGradE,
    SyntaxIdsf,
    SyntaxSpec,
    UnpackBlockFailed,
    UnpackBlockAlign,
    UnpackFrameAlign,
    FrameLengthOver,

    // Handle / API level
    IllSyncword,
    IllSamplingFreq,
    AssertSupSamplingFreq,
    AssertChannelConfig,
    InputBufferSize,
}

impl fmt::Display for LdacError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use LdacError::*;
        let s = match self {
            SyntaxBand => "invalid band count",
            SyntaxGradA => "gradient qu_l out of range",
            SyntaxGradB => "gradient qu_h out of range",
            SyntaxGradC => "gradient qu_h < qu_l",
            SyntaxGradD => "gradient qu_l (mode!=0) out of range",
            SyntaxGradE => "nadjqus > nqus",
            SyntaxIdsf => "scale-factor index out of range",
            SyntaxSpec => "spectrum codeword out of range",
            UnpackBlockFailed => "block unpack failed",
            UnpackBlockAlign => "block alignment padding not zero",
            UnpackFrameAlign => "frame fill-code mismatch",
            FrameLengthOver => "bitstream overran frame length",
            IllSyncword => "bad syncword",
            IllSamplingFreq => "unsupported sampling frequency",
            AssertSupSamplingFreq => "unsupported sampling rate in header",
            AssertChannelConfig => "unsupported channel config in header",
            InputBufferSize => "input buffer too small",
        };
        f.write_str(s)
    }
}

impl std::error::Error for LdacError {}
