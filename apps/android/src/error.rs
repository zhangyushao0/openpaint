//! `HostError`: why a JNI call could not be served. Logged by the JNI layer; never crosses into
//! Java as an exception.

use std::fmt;

use openpaint_runtime::RuntimeError;

use crate::motion::DecodeError;

#[derive(Debug)]
pub(crate) enum HostError {
    Jni(jni::errors::Error),
    Runtime(RuntimeError),
    Decode(DecodeError),
    /// The Java `Surface` had no native window (already released).
    NoWindow,
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Jni(error) => write!(f, "JNI error: {error}"),
            Self::Runtime(error) => write!(f, "{error}"),
            Self::Decode(error) => write!(f, "{error}"),
            Self::NoWindow => f.write_str("the surface has no native window"),
        }
    }
}

impl std::error::Error for HostError {}

impl From<jni::errors::Error> for HostError {
    fn from(error: jni::errors::Error) -> Self {
        Self::Jni(error)
    }
}

impl From<RuntimeError> for HostError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<DecodeError> for HostError {
    fn from(error: DecodeError) -> Self {
        Self::Decode(error)
    }
}
