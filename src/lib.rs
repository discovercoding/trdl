extern crate gl;

mod triangulation;
mod gl2d;

pub use gl2d::drawing::Window;
pub use gl2d::drawing::Drawing;
pub use gl2d::drawing::Path;

use std::io;
use std::error::Error;
use std::fmt;

// standard TRDL error
#[derive(Debug)]
pub enum TrdlError {
    ShaderIo(io::Error),
    NullString,
    CompileError(String),
    InvalidCompileError,
    LinkError(String),
    InvalidLinkError,
    NotEnoughVertices,
    NonSimplePolygon,
}

impl fmt::Display for TrdlError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TrdlError::ShaderIo(ref err) => err.fmt(f),
            TrdlError::NullString => write!(f, "{}", self.description()),
            TrdlError::CompileError(ref message) => write!(f, "{}", message),
            TrdlError::InvalidCompileError => write!(f, "{}", self.description()),
            TrdlError::LinkError(ref message) => write!(f, "{}", message),
            TrdlError::InvalidLinkError => write!(f, "{}", self.description()),
            TrdlError::NotEnoughVertices => write!(f, "{}", self.description()),
            TrdlError::NonSimplePolygon => write!(f, "{}", self.description()),
        }
    }
}

impl std::error::Error for TrdlError {
    fn description(&self) -> &str {
        match *self {
            TrdlError::ShaderIo(ref err) => err.description(),
            TrdlError::NullString => "Shader string was null",
            TrdlError::CompileError(ref message) => message,
            TrdlError::InvalidCompileError => "An error occurred during shader compile",
            TrdlError::LinkError(ref message) => message,
            TrdlError::InvalidLinkError => "An error occurred during shader program link",
            TrdlError::NotEnoughVertices => "A polygon must have 3 or more points",
            TrdlError::NonSimplePolygon => "Error triangulating polygon, is it non-simple?"
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            TrdlError::ShaderIo(ref err) => Some(err),
            TrdlError::NullString => None,
            TrdlError::CompileError(_) => None,
            TrdlError::InvalidCompileError =>  None,
            TrdlError::LinkError(_) => None,
            TrdlError::InvalidLinkError => None,
            TrdlError::NotEnoughVertices => None,
            TrdlError::NonSimplePolygon => None
        }
    }
}

impl From<io::Error> for TrdlError {
    fn from(err: io::Error) -> TrdlError {
        TrdlError::ShaderIo(err)
    }
}

