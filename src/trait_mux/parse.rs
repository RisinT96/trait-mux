//! This module provides functionality to parse a named list of traits or paths from a `TokenStream`.
//! It supports both simple trait names (e.g., `Display`) and full paths (e.g., `std::fmt::Display`).
//! The parsed traits are stored as `Path` objects in the `Ast` struct, along with the name of the implementation.

use proc_macro2::TokenStream;
use proc_macro_error::abort;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{parse2, Ident, Path, Result, Token};

/// Represents the parsed Abstract Syntax Tree (AST) for a named list of traits or paths.
///
/// The syntax format is `SomeName{Display, std::fmt::Debug}`, where:
/// - `SomeName` is the name of the implementation.
/// - `{Display, std::fmt::Debug}` is a comma-separated list of traits or paths.
pub struct Ast {
    /// The name of the implementation (e.g., `SomeName`).
    pub name: Ident,
    /// A punctuated list of parsed paths representing traits or modules.
    /// Each path can be a simple identifier (e.g., `Display`) or a full path (e.g., `std::fmt::Display`).
    pub paths: Punctuated<Path, Comma>,
}

impl Parse for Ast {
    /// Parses a syntax like `SomeName{Display, std::fmt::Debug}`.
    ///
    /// # Arguments
    /// * `input` - The input stream to parse.
    ///
    /// # Returns
    /// * `Result<Self>` - The parsed `Ast` containing the identifier name and list of paths.
    ///
    /// # Errors
    /// Returns an error if the input does not match the expected syntax.
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse::<Ident>()?;

        let content;
        syn::braced!(content in input);

        let paths = Punctuated::<Path, Token![,]>::parse_terminated(&content)?;

        Ok(Ast { name, paths })
    }
}

/// Parses a `TokenStream` into an `Ast` containing a named list of paths.
///
/// The input must follow the syntax `SomeName{Display, std::fmt::Debug}`.
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

    /// Tests parsing with the new syntax format: Name{traits...}.
    ///
    /// Verifies that the parser correctly extracts the name and paths.
    #[test]
    fn valid_named_syntax() {
        let ast = parse(quote!(SomeName{Display, std::fmt::Debug}));

        assert_eq!(ast.name.to_string(), "SomeName");
        assert_eq!(ast.paths.len(), 2);
        assert_eq!(ast.paths[0].get_ident().unwrap().to_string(), "Display");

        let debug = &ast.paths[1].segments;
        assert_eq!(debug.len(), 3);
        assert_eq!(debug[0].ident.to_string(), "std");
        assert_eq!(debug[1].ident.to_string(), "fmt");
        assert_eq!(debug[2].ident.to_string(), "Debug");
    }

    /// Tests parsing with mixed path formats.
    ///
    /// Verifies that the parser handles a mix of full paths and simple trait names.
    #[test]
    fn valid_syntax_mixed_paths() {
        let ast = parse(quote!(MyImpl{std::fmt::Display, ::fmt::Debug, Clone}));

        assert_eq!(ast.name.to_string(), "MyImpl");
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

    /// Tests parsing an empty list of traits with a name.
    ///
    /// Verifies that the parser correctly handles an empty list of traits.
    #[test]
    fn empty_named_trait_list() {
        let ast = parse(quote!(EmptyImpl {}));
        assert_eq!(ast.name.to_string(), "EmptyImpl");
        assert_eq!(ast.paths.len(), 0);
    }

    /// Tests parsing invalid input where a number is used instead of a valid path.
    ///
    /// Verifies that the parser fails when encountering invalid paths.
    #[test]
    #[should_panic]
    fn invalid_trait_input_not_path() {
        // Using a number instead of an identifier, which should cause the parser to fail
        parse(quote!(Invalid{Display, 123, Debug}));
        // The test should panic due to the abort! macro being called
    }

    /// Tests parsing invalid input with missing braces.
    ///
    /// Verifies that the parser fails when braces are missing.
    #[test]
    #[should_panic]
    fn invalid_trait_input_missing_braces() {
        parse(quote!(NoImpl));
    }

    /// Tests parsing invalid input with empty input.
    ///
    /// Verifies that the parser fails when the input is empty.
    #[test]
    #[should_panic]
    fn invalid_empty_input() {
        parse(quote!());
    }
}
