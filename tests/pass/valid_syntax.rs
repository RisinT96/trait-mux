use std::fmt::Binary;
use trait_mux::trait_mux;

fn main() {
    trait_mux!(MyEnum {});
    trait_mux!(MyEnum2{std::fmt::Debug, Binary});
}
