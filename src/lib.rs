#![allow(dead_code, unused)]

use std::fmt::{Binary, Debug, Display};

pub use trait_mux_macros::trait_mux;

pub struct Match<'t, T>(pub &'t T);

//// TEST

pub trait BinaryDebug: Binary + Debug {}
impl<T: Binary + Debug> BinaryDebug for T {}

pub enum Test<'t> {
    BinaryDebug(&'t dyn BinaryDebug),
    Binary(&'t dyn Binary),
    Debug(&'t dyn Debug),
    Display(&'t dyn Display),
    None,
}

impl<'t> Test<'t> {
    pub fn try_as_binary(&self) -> Option<&dyn Binary> {
        match self {
            Test::BinaryDebug(binary_debug) => Some(binary_debug),
            Test::Binary(binary) => Some(binary),
            _ => None,
        }
    }

    pub fn try_as_debug(&self) -> Option<&dyn Debug> {
        match self {
            Test::BinaryDebug(bd) => Some(bd),
            Test::Debug(d) => Some(d),
            _ => None,
        }
    }
}

pub trait MatchBinaryDebug {
    fn wrap<'t>(&'t self) -> Test<'t>;
}
impl<'t, T: BinaryDebug> MatchBinaryDebug for &&&&Match<'t, T> {
    fn wrap(&self) -> Test<'t> {
        Test::BinaryDebug(self.0)
    }
}
pub trait MatchBinary {
    fn wrap<'t>(&'t self) -> Test<'t>;
}
impl<'t, T: Binary> MatchBinary for &&&Match<'t, T> {
    fn wrap(&self) -> Test<'t> {
        Test::Binary(self.0)
    }
}
pub trait MatchDebug {
    fn wrap<'t>(&'t self) -> Test<'t>;
}
impl<'t, T: Debug> MatchDebug for &&Match<'t, T> {
    fn wrap(&self) -> Test<'t> {
        Test::Debug(self.0)
    }
}

macro_rules! into_test {
    ($var:tt) => {
        (&&&&&$crate::Match(&$var)).wrap()
    };
}
