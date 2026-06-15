use std::path::PathBuf;

#[derive(Debug)]
pub enum GeneratorError {
    Io(std::io::Error),
    Xml(roxmltree::Error),
    Format(String),
    MissingFile(PathBuf),
}

impl std::fmt::Display for GeneratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Xml(e) => write!(f, "xml error: {e}"),
            Self::Format(msg) => write!(f, "{msg}"),
            Self::MissingFile(path) => write!(f, "file does not exist: {}", path.display()),
        }
    }
}

impl std::error::Error for GeneratorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Xml(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for GeneratorError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<roxmltree::Error> for GeneratorError {
    fn from(value: roxmltree::Error) -> Self {
        Self::Xml(value)
    }
}

pub type Result<T> = std::result::Result<T, GeneratorError>;
