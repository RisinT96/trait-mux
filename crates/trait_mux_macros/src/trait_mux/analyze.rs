//! Analysis module that transforms the parsed AST into a Model.
//! This model represents all the information necessary to generate the output code.
//! The main responsibility is to extract traits and generate all possible enum variants
//! that will be used in the final generated code.

use proc_macro2::{Ident, Span};
use syn::{Path, spanned::Spanned};

use crate::parse::Ast;

/// The core model structure that contains all processed information from the AST.
/// This model is used as input for code generation, representing enum variants and traits
/// in a format that's easy to work with.
pub struct Model<'t> {
    /// The identifier of the main enum, taken from the AST.
    pub enum_ident: &'t Ident,
    /// All possible variants of the enum based on trait combinations.
    pub enum_variants: Vec<EnumVariant<'t>>,
    /// The identifier for the wrapper structure that will encapsulate the enum.
    pub wrap_ident: Ident,
    /// All traits extracted from the AST.
    pub traits: Vec<Trait<'t>>,
}

/// Represents a trait with its identifier and path.
/// Used to track traits throughout the code generation process.
#[derive(Copy, Clone)]
pub struct Trait<'t> {
    /// The identifier of the trait (the name).
    pub ident: &'t Ident,
    /// The full path to the trait, including any module qualifiers.
    pub path: &'t Path,
}

/// Represents an enum variant, including its identifier, and the traits it implements.
/// Each variant corresponds to a specific combination of implemented traits.
pub struct EnumVariant<'t> {
    /// The enum ident, used both for the enum variant name, and (if needed) for the trait aggregate
    /// name.
    /// e.g. `TypeBinaryDebugDisplay`
    pub ident: Ident,
    /// The traits that this enum implements.
    /// This is a subset of all traits defined in the Model.
    pub implemented_traits: Vec<Trait<'t>>,
}

/// Analyzes the AST and constructs a Model containing all the necessary information
/// for code generation. This is the main entry point for the analysis phase.
///
/// # Arguments
///
/// * `ast` - The parsed AST containing trait paths and enum name
///
/// # Returns
///
/// A Model containing the processed information ready for code generation
pub fn analyze(ast: &Ast) -> Model {
    let traits = extract_traits(ast);
    let enum_variants = generate_enum_variants(ast, &traits);
    let wrap_ident = Ident::new(&format!("Wrap{}", ast.name), Span::call_site());

    Model {
        enum_ident: &ast.name,
        enum_variants,
        wrap_ident,
        traits,
    }
}

/// Extracts traits from the given AST and converts them to the Trait model.
/// Emits an error if a path is empty or malformed.
///
/// # Arguments
///
/// * `ast` - The AST to extract traits from
///
/// # Returns
///
/// A vector of Trait structs sorted alphabetically by their identifiers
fn extract_traits(ast: &Ast) -> Vec<Trait> {
    let mut traits = vec![];

    for path in &ast.paths {
        if path.segments.is_empty() {
            proc_macro_error::emit_error!(
                path.span(),
                "unexpected end of input, expected identifier"
            );
        }

        // Unwrap safety: checked that segments is not empty.
        traits.push(Trait {
            ident: &path.segments.last().unwrap().ident,
            path,
        });
    }

    // Sort traits alphabetically by their identifier.
    traits.sort_by_key(|t| t.ident.to_string());

    traits
}

/// Generates all possible enum variants from the given traits.
/// The variants are sorted by descending length and then alphabetically.
/// The order is very important for later stages, as we want to generate code
/// with the most specific trait constraints first, and relax the constraints as
/// we go down, if the order was incorrect, autoref specialization won't work
/// properly.
///
/// # Arguments
///
/// * `ast` - The AST containing the enum name
/// * `traits` - A vector of Trait structs to generate permutations from
///
/// # Returns
///
/// A vector of EnumVariant structs representing all possible trait combinations
fn generate_enum_variants<'t>(ast: &Ast, traits: &Vec<Trait<'t>>) -> Vec<EnumVariant<'t>> {
    let mut permutations = Vec::new();
    let n = traits.len();

    // Create all possible permutations of the trait names.
    // We have 2^n possible permutations.
    for i in 0..(1 << n) {
        let mut permutation = vec![];

        for (j, r#trait) in traits.iter().enumerate() {
            if (i & (1 << j)) != 0 {
                permutation.push(*r#trait);
            }
        }
        permutations.push(permutation);
    }

    let mut variants = permutations
        .iter()
        .map(|variant| {
            let variant_name = if variant.is_empty() {
                "None".to_string()
            } else {
                variant
                    .iter()
                    .map(|t| t.ident.to_string())
                    .collect::<String>()
            };

            let variant_name = format!("{}{}", ast.name, variant_name);

            EnumVariant {
                ident: Ident::new(&variant_name, Span::call_site()),
                implemented_traits: variant.to_vec(),
            }
        })
        .collect::<Vec<_>>();

    // Calculate how many characters (digits) are needed to represent the number of traits (n)
    // Needed in case we have to zero pad for proper sorting.
    // Not the most efficient but good enough, as n is not expected to be massive.
    let n_chars = n.to_string().len();

    // Sort by the length of implemented traits (descending), then alphabetically (ascending).
    variants.sort_by_key(|e| {
        format!(
            "{:0width$} {}",
            n - e.implemented_traits.len(),
            e.ident,
            width = n_chars
        )
    });

    variants
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, punctuated::Punctuated};

    #[test]
    fn test_generate_enum_variants_empty() {
        // Test with no traits
        let ast = Ast {
            name: Ident::new("Test", Span::call_site()),
            paths: Punctuated::new(),
        };

        let traits = extract_traits(&ast);
        let variants = generate_enum_variants(&ast, &traits);

        assert_eq!(variants.len(), 1);

        assert_eq!(variants[0].ident.to_string(), "TestNone");
        assert!(variants[0].implemented_traits.is_empty());
    }

    #[test]
    fn test_generate_enum_variants_single_trait() {
        // Test with a single trait
        let ast = Ast {
            name: Ident::new("Test", Span::call_site()),
            paths: parse_quote!(Debug),
        };

        let traits = extract_traits(&ast);
        let variants = generate_enum_variants(&ast, &traits);

        assert_eq!(variants.len(), 2);

        assert_eq!(variants[0].ident.to_string(), "TestDebug");
        assert_eq!(variants[0].implemented_traits.len(), 1);

        assert_eq!(variants[1].ident.to_string(), "TestNone");
        assert!(variants[1].implemented_traits.is_empty());
    }

    #[test]
    fn test_generate_enum_variants_multiple_traits() {
        // Test with multiple traits
        let ast = Ast {
            name: Ident::new("Type", Span::call_site()),
            paths: parse_quote!(Debug, Display, Clone),
        };

        let traits = extract_traits(&ast);
        let variants = generate_enum_variants(&ast, &traits);

        // Should have 2^3 = 8 variants
        assert_eq!(variants.len(), 8);

        // Check that sorting is done correctly - by trait count then alphabetically

        let two_trait_variants = &variants[1..4];
        let one_trait_variants = &variants[4..7];

        // Check that variants with more traits come first
        assert_eq!(variants[0].implemented_traits.len(), 3);
        assert_eq!(variants[0].ident.to_string(), "TypeCloneDebugDisplay");

        for v in two_trait_variants {
            assert_eq!(v.implemented_traits.len(), 2);
        }

        for v in one_trait_variants {
            assert_eq!(v.implemented_traits.len(), 1);
        }

        // Check that the empty variant comes last
        assert_eq!(variants[7].ident.to_string(), "TypeNone");
        assert!(variants[7].implemented_traits.is_empty());

        // Check alphabetical order among same-length trait combinations
        assert!(two_trait_variants[0].ident.to_string() <= two_trait_variants[1].ident.to_string());
        assert!(two_trait_variants[1].ident.to_string() <= two_trait_variants[2].ident.to_string());

        assert!(one_trait_variants[0].ident.to_string() <= one_trait_variants[1].ident.to_string());
        assert!(one_trait_variants[1].ident.to_string() <= one_trait_variants[2].ident.to_string());
    }

    #[test]
    fn test_extract_traits_sorting() {
        // Test that traits are sorted alphabetically
        let ast = Ast {
            name: Ident::new("Test", Span::call_site()),
            paths: parse_quote!(Zzz, Aaa, Mmm),
        };

        let traits = extract_traits(&ast);

        assert_eq!(traits.len(), 3);
        assert_eq!(traits[0].ident.to_string(), "Aaa");
        assert_eq!(traits[1].ident.to_string(), "Mmm");
        assert_eq!(traits[2].ident.to_string(), "Zzz");
    }
}
