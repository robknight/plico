use std::backtrace::Backtrace;
pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum SolverError {
    #[error("{0}")]
    Custom(String),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Inner: {inner}\n{backtrace}")]
    Inner {
        inner: Box<SolverError>,
        backtrace: Box<Backtrace>,
    },
}

impl From<SolverError> for Error {
    fn from(inner: SolverError) -> Self {
        Error::Inner {
            inner: Box::new(inner),
            backtrace: Box::new(std::backtrace::Backtrace::capture()),
        }
    }
}
