use trait_mux::trait_mux;
use std::fmt::Binary;

fn main() {
    trait_mux!();
    trait_mux!(std::fmt::Debug, Binary);
}
