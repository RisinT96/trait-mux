//! This module provides functionality to parse a list of traits or paths from a `TokenStream`.
//! It supports both simple trait names (e.g., `Display`) and full paths (e.g., `std::fmt::Display`).
//! The parsed traits are stored as `Path` objects in the `Ast` struct.

use proc_macro2::TokenStream;
use proc_macro_error::abort;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse2, Path, Result, Token};

/// Represents the parsed Abstract Syntax Tree (AST) for a list of traits or paths.
pub struct Ast {
    /// A vector of parsed paths representing traits or modules.
    pub paths: Vec<Path>,
}

impl Parse for Ast {
    /// Parses a comma-separated list of paths from the input stream.
    ///
    /// # Arguments
    /// * `input` - The input stream to parse.
    ///
    /// # Returns
    /// * `Result<Self>` - The parsed `Ast` containing the list of paths.
    ///
    /// # Errors
    /// Returns an error if the input does not match the expected syntax.
    fn parse(input: ParseStream) -> Result<Self> {
        let traits = Punctuated::<Path, Token![,]>::parse_terminated(input)?;
        Ok(Ast {
            paths: traits.into_iter().collect(),
        })
    }
}

/// Parses a `TokenStream` into an `Ast` containing a list of paths.
///
/// # Arguments
/// * `ts` - The `TokenStream` to parse.
///
/// # Returns
/// * `Ast` - The parsed AST.
///
/// # Panics
/// Panics if the input cannot be parsed, using the `abort!` macro to provide an error message.
pub fn parse(ts: TokenStream) -> Ast {
    match parse2::<Ast>(ts) {
        Ok(ast) => ast,
        Err(e) => {
            abort!(e.span(), e)
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for the parsing functionality.
    //!
    //! These tests verify that the parser correctly handles valid and invalid inputs,
    //! including simple trait names, full paths, and edge cases.

    use super::*;
    use quote::quote;

    /// Tests parsing a list of simple trait names.
    #[test]
    fn valid_syntax() {
        let ast = parse(quote!(Display, Debug, Clone));

        assert_eq!(ast.paths.len(), 3);
        assert_eq!(ast.paths[0].get_ident().unwrap().to_string(), "Display");
        assert_eq!(ast.paths[1].get_ident().unwrap().to_string(), "Debug");
        assert_eq!(ast.paths[2].get_ident().unwrap().to_string(), "Clone");
    }

    /// Tests parsing a mix of full paths and simple trait names.
    #[test]
    fn valid_syntax_mixed_paths() {
        let ast = parse(quote!(std::fmt::Display, ::fmt::Debug, Clone));

        assert_eq!(ast.paths.len(), 3);
        // Check the segments of the path for the first trait
        let display = &ast.paths[0].segments;
        assert_eq!(display.len(), 3);
        assert_eq!(display[0].ident.to_string(), "std");
        assert_eq!(display[1].ident.to_string(), "fmt");
        assert_eq!(display[2].ident.to_string(), "Display");

        let debug = &ast.paths[1].segments;
        assert_eq!(debug.len(), 2);
        assert_eq!(debug[0].ident.to_string(), "fmt");
        assert_eq!(debug[1].ident.to_string(), "Debug");

        let clone = &ast.paths[2].segments;
        assert_eq!(clone.len(), 1);
        assert_eq!(clone[0].ident.to_string(), "Clone");
    }

    /// Tests parsing an empty list of traits.
    #[test]
    fn empty_trait_list() {
        let ast = parse(quote!());
        assert_eq!(ast.paths.len(), 0);
    }

    /// Tests parsing invalid input where a number is used instead of a valid path.
    #[test]
    #[should_panic]
    fn invalid_trait_input_not_path() {
        // Using a number instead of an identifier, which should cause the parser to fail
        parse(quote!(Display, 123, Debug));
        // The test should panic due to the abort! macro being called
    }

    /// Tests parsing invalid input with unsupported syntax.
    #[test]
    #[should_panic]
    fn invalid_trait_input() {
        parse(quote!({}));
    }
}
