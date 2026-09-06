use std::{fmt, io};

/// Failure returned at the plugin-host seam.
/// 插件宿主接口返回的失败。
#[derive(Debug)]
pub enum HostError {
    InvalidArtifact(String),
    InvalidOperation(String),
    Abi(String),
    Limit(String),
    Process(String),
    Timeout,
    Health(String),
    Slot(String),
    Policy(String),
    Contract(String),
    State(String),
    Registry(String),
    Io(io::Error),
}

impl fmt::Display for HostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArtifact(message) => {
                write!(formatter, "invalid plugin artifact: {message}")
            }
            Self::InvalidOperation(message) => {
                write!(formatter, "invalid plugin operation: {message}")
            }
            Self::Abi(message) => write!(formatter, "plugin ABI error: {message}"),
            Self::Limit(message) => write!(formatter, "plugin limit exceeded: {message}"),
            Self::Process(message) => write!(formatter, "plugin process failed: {message}"),
            Self::Timeout => formatter.write_str("plugin call timed out"),
            Self::Health(message) => write!(formatter, "plugin health check failed: {message}"),
            Self::Slot(message) => write!(formatter, "plugin slot error: {message}"),
            Self::Policy(message) => {
                write!(formatter, "plugin policy rejected artifact: {message}")
            }
            Self::Contract(message) => write!(formatter, "plugin contract mismatch: {message}"),
            Self::State(message) => write!(formatter, "plugin host state failed: {message}"),
            Self::Registry(message) => write!(formatter, "registry graft failed: {message}"),
            Self::Io(error) => write!(formatter, "plugin I/O failed: {error}"),
        }
    }
}

impl std::error::Error for HostError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for HostError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
