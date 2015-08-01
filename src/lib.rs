//! Interface to [SQLite][1].
//!
//! ## Example
//!
//! ```
//! let connection = sqlite::open(":memory:").unwrap();
//!
//! connection.execute("
//!     CREATE TABLE users (id INTEGER, name VARCHAR(255));
//!     INSERT INTO users (id, name) VALUES (42, 'Alice');
//! ").unwrap();
//!
//! connection.process("SELECT * FROM users", |pairs| {
//!     for &(column, value) in pairs.iter() {
//!         println!("{} = {}", column, value.unwrap());
//!     }
//!     true
//! }).unwrap();
//! ```
//!
//! The same example using prepared statements:
//!
//! ```
//! use sqlite::State;
//!
//! let connection = sqlite::open(":memory:").unwrap();
//!
//! connection.execute("
//!     CREATE TABLE users (id INTEGER, name VARCHAR(255))
//! ");
//!
//! let mut statement = connection.prepare("
//!     INSERT INTO users (id, name) VALUES (?, ?)
//! ").unwrap();
//! statement.bind(1, 42).unwrap();
//! statement.bind(2, "Alice").unwrap();
//! assert_eq!(statement.step().unwrap(), State::Done);
//!
//! let mut statement = connection.prepare("SELECT * FROM users").unwrap();
//! while let State::Row = statement.step().unwrap() {
//!     println!("id = {}", statement.read::<i64>(0).unwrap());
//!     println!("name = {}", statement.read::<String>(1).unwrap());
//! }
//! ```
//!
//! [1]: https://www.sqlite.org

extern crate libc;
extern crate sqlite3_sys as ffi;

use std::{error, fmt};

macro_rules! raise(
    ($message:expr) => (return Err(::Error { code: None, message: Some($message.to_string()) }));
);

macro_rules! error(
    ($connection:expr, $code:expr) => (match ::last_error($connection) {
        Some(error) => return Err(error),
        _ => return Err(::Error { code: Some($code as isize), message: None }),
    });
);

macro_rules! ok(
    ($connection:expr, $result:expr) => (match $result {
        ::ffi::SQLITE_OK => {},
        code => error!($connection, code),
    });
    ($result:expr) => (match $result {
        ::ffi::SQLITE_OK => {},
        code => return Err(::Error { code: Some(code as isize), message: None }),
    });
);

macro_rules! c_str_to_str(
    ($string:expr) => (::std::str::from_utf8(::std::ffi::CStr::from_ptr($string).to_bytes()));
);

macro_rules! c_str_to_string(
    ($string:expr) => (
        String::from_utf8_lossy(::std::ffi::CStr::from_ptr($string as *const _).to_bytes())
               .into_owned()
    );
);

macro_rules! path_to_cstr(
    ($path:expr) => (match $path.to_str() {
        Some(path) => match ::std::ffi::CString::new(path) {
            Ok(string) => string,
            _ => raise!("failed to process a path"),
        },
        _ => raise!("failed to process a path"),
    });
);

macro_rules! str_to_cstr(
    ($string:expr) => (match ::std::ffi::CString::new($string) {
        Ok(string) => string,
        _ => raise!("failed to process a string"),
    });
);

/// An error.
#[derive(Debug)]
pub struct Error {
    /// The error code.
    pub code: Option<isize>,
    /// The error message.
    pub message: Option<String>,
}

/// A result.
pub type Result<T> = std::result::Result<T, Error>;

/// A data type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Type {
    /// Binary data.
    Blob,
    /// A 64-bit IEEE floating-point number.
    Float,
    /// A 64-bit signed integer.
    Integer,
    /// An absence of a value.
    Null,
    /// A string.
    String,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match (self.code, &self.message) {
            (Some(code), &Some(ref message)) => write!(formatter, "{} (code {})", message, code),
            (Some(code), _) => write!(formatter, "an SQLite error (code {})", code),
            (_, &Some(ref message)) => message.fmt(formatter),
            _ => write!(formatter, "an SQLite error"),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self.message {
            Some(ref message) => message,
            _ => "an SQLite error",
        }
    }
}

mod connection;
mod statement;

pub use connection::Connection;
pub use statement::{Statement, State, Parameter, Value};

/// Open a connection to a new or existing database.
#[inline]
pub fn open<T: AsRef<std::path::Path>>(path: T) -> Result<Connection> {
    Connection::open(path)
}

fn last_error(raw: *mut ffi::sqlite3) -> Option<Error> {
    unsafe {
        let code = ffi::sqlite3_errcode(raw);
        if code == ffi::SQLITE_OK {
            return None;
        }
        let message = ffi::sqlite3_errmsg(raw);
        if message.is_null() {
            return None;
        }
        Some(Error { code: Some(code as isize), message: Some(c_str_to_string!(message)) })
    }
}
