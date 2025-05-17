# trait-mux

Proc macro library for generating enums that can multiplex different trait objects.

[![Crates.io](https://img.shields.io/crates/v/trait_mux.svg)](https://crates.io/crates/trait_mux)
[![Documentation](https://docs.rs/trait_mux/badge.svg)](https://docs.rs/trait_mux)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

## Overview

`trait-mux` provides a macro solution for multiplexing different trait objects within a single enum. This is useful when you need to handle multiple trait implementations through a common interface.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
trait_mux = "0.2.0"
```

## Usage

### Basic Example

```rust
use trait_mux::trait_mux;

// Define some traits
trait Greet {
    fn greet(&self) -> &'static str;
}

trait Calculate {
    fn add(&self, a: i32, b: i32) -> i32;
}

// Generate a multiplexer enum for these traits
trait_mux!(MyMux { Greet, Calculate });

// Implement traits for concrete types
struct Greeter;
impl Greet for Greeter {
    fn greet(&self) -> &'static str {
        "Hi, I'm a greeter!"
    }
}

struct Calculator;
impl Calculate for Calculator {
    fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}

struct CalculatorGreeter;
impl Greet for CalculatorGreeter {
    fn greet(&self) -> &'static str {
        "Hi, I'm a calculator greeter!"
    }
}
impl Calculate for CalculatorGreeter {
    fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}

struct Nothing;

fn main() {
    // Use the generated enum to store different trait objects
    let mut objects: Vec<MyMux> = Vec::new();

    let greeter = Greeter;
    let calculator = Calculator;
    let calculator_greeter = CalculatorGreeter;
    let nothing = Nothing;

    // Add trait objects to the vector
    objects.push(into_my_mux!(greeter));
    objects.push(into_my_mux!(calculator));
    objects.push(into_my_mux!(calculator_greeter));
    objects.push(into_my_mux!(nothing));

    // Use the trait methods through the enum
    for obj in &objects {
        if let Some(greeter) = obj.try_as_greet() {
            println!("{}", greeter.greet());
        } else {
            println!("I don't implement Greet!");
        }

        if let Some(calc) = obj.try_as_calculate() {
            println!("5 + 3 = {}", calc.add(5, 3));
        } else {
            println!("I don't implement Calculate!");
        }
    }
}
```

## Features

- Generate enums that wrap multiple trait objects
- Automatic conversion from implementors to the generated enum
- Type-safe downcasting back to specific trait objects
- Support for generic traits

## How It Works

The `#[trait_mux]` attribute macro generates an enum with variants for each possible combination of
the specified traits. It also implements conversion methods, allowing you to:

1. Convert trait implementors to the enum
2. Downcast from the enum back to trait objects
3. Access trait methods in a type-safe manner

## License

This project is licensed under the [MIT License](LICENSE).
