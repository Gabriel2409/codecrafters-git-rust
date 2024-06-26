use derive_more::From;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, From)]
pub enum Error {
    InvalidBranchOrTag,
    InvalidHash(String),
    InvalidGitObject,
    NotATreeGitObject,
    TreeChildNotLoaded,
    ObjectBytesNotLoaded,
    Unreachable,
    InvalidSmartHttpRes,
    InvalidDiscoveryUrl(String),
    ContentTypeNotFound,
    ContentTypeInvalid,
    WrongContentType {
        expected: String,
        got: String,
    },
    WrongObjectSize {
        expected: usize,
        got: usize,
    },
    InvalidDiscoveryService {
        expected: String,
        got: String,
    },
    InvalidPackObjectType(usize),
    IncorrectPackObjectSize {
        expected: usize,
        got: usize,
    },
    ObjectNotFound(String),
    CantBuildFromRefDelta,
    InvalidPackFile,
    // -- Externals
    #[from]
    Io(std::io::Error),
    #[from]
    Reqwest(reqwest::Error),
}

// region:    --- Error Boilerplate
impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}
// endregion: --- Error Boilerplate
