use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    // Network errors
    #[error("TCP I/O error: {0}")]
    TcpIo(#[from] std::io::Error),

    #[error("failed to parse packet: {0}")]
    PacketParse(String),

    #[error("invalid UTF-8 in packet string: {0}")]
    PacketStringEncoding(#[from] std::str::Utf8Error),

    #[error("invalid byte conversion in packet: {0}")]
    PacketByteConversion(#[from] std::array::TryFromSliceError),

    // Timer errors
    #[error("timer send failed: channel disconnected")]
    TimerSend,

    #[error("timer mutex lock poisoned")]
    TimerLockPoisoned,

    // Compression errors
    #[error("compression error: {0}")]
    Compression(String),

    // State errors
    #[error("invalid state: {0}")]
    State(String),

    // Configuration errors
    #[error("configuration error: {0}")]
    Config(String),
}
