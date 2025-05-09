#![allow(unused_macros)]

use std::fmt;
use std::fmt::Binary;
use trait_mux::trait_mux;

trait_mux!(MyEnum{std::fmt::Debug, Binary, fmt::Display});

fn main() {}
