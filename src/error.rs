use std::error::Error;
use std::fmt;

use common::HashSpaceId;



#[derive(Debug)]
pub enum HashError {
    UnsupportedType,
    BadInputLength,
    UnknownCode,
    Other(Box<Error>),
}

impl fmt::Display for HashError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str( Error::description(self) )
    }
}

impl Error for HashError {
    fn description(&self) -> &str {
        match *self {
            HashError::UnsupportedType  => "This type is not supported yet",
            HashError::BadInputLength   => "Not matching input length",
            HashError::UnknownCode      => "Found unknown code",
            HashError::Other(ref e)     => e.description(),
        }
    }
}



#[derive(Debug)]
pub enum SerializerError {
    SerializationError(Box<Error>),
    DeserializationError(Box<Error>),
    Other(Box<Error>),
}

impl fmt::Display for SerializerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str( Error::description(self) )
    }
}

impl Error for SerializerError {
    fn description(&self) -> &str {
        match *self {
            SerializerError::SerializationError(ref e)      => e.description(),
            SerializerError::DeserializationError(ref e)    => e.description(),
            SerializerError::Other(ref e)                   => e.description(),
        }
    }
}



#[derive(Debug)]
pub enum StorageError {
    OutOfDiskSpace,
    InvalidKey,
    Other(Box<Error>),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str( Error::description(self) )
    }
}

impl Error for StorageError {
    fn description(&self) -> &str {
        match *self {
            StorageError::OutOfDiskSpace    => "Run out of disk space",
            StorageError::InvalidKey        => "The given key holds no value",
            StorageError::Other(ref e)      => e.description(),
        }
    }
}



#[derive(Debug)]
pub enum StringCoderError {
    Other(Box<Error>),
}

impl fmt::Display for StringCoderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str( Error::description(self) )
    }
}

impl Error for StringCoderError {
    fn description(&self) -> &str {
        match *self {
            StringCoderError::Other(ref e) => e.description(),
        }
    }
}



#[derive(Debug)]
pub enum HashSpaceError {
    SerializerError(SerializerError),
    HashError(HashError),
    StorageError(StorageError),
    StringCoderError(StringCoderError),
    UnknownHashSpace(HashSpaceId),
    UnsupportedHashSpace(HashSpaceId),
    Other(Box<Error>),
}

impl fmt::Display for HashSpaceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str( Error::description(self) )
    }
}

impl Error for HashSpaceError {
    fn description(&self) -> &str {
        match *self {
            HashSpaceError::SerializerError(ref e)  => e.description(),
            HashSpaceError::HashError(ref e)        => e.description(),
            HashSpaceError::StorageError(ref e)     => e.description(),
            HashSpaceError::StringCoderError(ref e) => e.description(),
            HashSpaceError::UnknownHashSpace(ref _s) => "Unknown hashspace identifier: {:?}",
            HashSpaceError::UnsupportedHashSpace(ref _s) => "Hashspace is not supporteD: {:?}",
            HashSpaceError::Other(ref e)            => e.description(),
        }
    }
}
