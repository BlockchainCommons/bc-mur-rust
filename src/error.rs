use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("QR encoding failed: {0}")]
    QrEncode(String),

    #[error("Image encoding failed: {0}")]
    ImageEncode(String),

    #[error("SVG rendering failed: {0}")]
    SvgRender(String),

    #[error("Invalid color: {0}")]
    InvalidColor(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("GIF encoding failed: {0}")]
    GifEncode(String),

    #[error("ffmpeg not found on PATH — install ffmpeg for ProRes output")]
    FfmpegNotFound,

    #[error("ffmpeg failed: {0}")]
    FfmpegFailed(String),

    #[error("QR code too dense: {module_count} modules exceeds limit of \
             {max_modules} (reduce data size, lower error correction, \
             or increase --max-modules)")]
    QrCodeTooDense {
        module_count: usize,
        max_modules: usize,
    },

    #[error("insufficient frames: {requested} requested but message \
             requires at least {fragments} fragments")]
    InsufficientFrames {
        requested: usize,
        fragments: usize,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("UR error: {0}")]
    Ur(#[from] bc_ur::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
