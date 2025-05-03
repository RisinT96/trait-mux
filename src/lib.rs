#![allow(dead_code, unused)]

use std::fmt::{Binary, Debug, Display};

pub use trait_mux_macros::trait_mux;

struct Match<'t, T>(&'t T);
