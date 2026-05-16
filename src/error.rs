//! Errors raised by [`networkframework`](crate).

use core::fmt;

/// Failure modes from the C shim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkError {
    InvalidArgument(String),
    ConnectFailed,
    SendFailed,
    ReceiveFailed,
    ListenFailed,
    Cancelled,
    Timeout,
    Unknown(i32),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument(m) => write!(f, "invalid argument: {m}"),
            Self::ConnectFailed => write!(f, "connect failed"),
            Self::SendFailed => write!(f, "send failed"),
            Self::ReceiveFailed => write!(f, "receive failed"),
            Self::ListenFailed => write!(f, "listen failed"),
            Self::Cancelled => write!(f, "operation cancelled"),
            Self::Timeout => write!(f, "operation timed out"),
            Self::Unknown(c) => write!(f, "unknown shim status {c}"),
        }
    }
}

impl std::error::Error for NetworkError {}

#[must_use]
pub(crate) fn from_status(code: i32) -> NetworkError {
    use crate::ffi::{
        NW_CANCELLED, NW_CONNECT_FAILED, NW_INVALID_ARG, NW_LISTEN_FAILED, NW_RECV_FAILED,
        NW_SEND_FAILED, NW_TIMEOUT,
    };
    match code {
        NW_INVALID_ARG => NetworkError::InvalidArgument("shim".into()),
        NW_CONNECT_FAILED => NetworkError::ConnectFailed,
        NW_SEND_FAILED => NetworkError::SendFailed,
        NW_RECV_FAILED => NetworkError::ReceiveFailed,
        NW_LISTEN_FAILED => NetworkError::ListenFailed,
        NW_CANCELLED => NetworkError::Cancelled,
        NW_TIMEOUT => NetworkError::Timeout,
        other => NetworkError::Unknown(other),
    }
}
