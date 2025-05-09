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

    // Sort by the length of implemented traits, then alphabetically.
    variants.sort_by_key(|e| format!("{} {}", e.implemented_traits.len(), e.ident));
    variants.reverse();

    variants
}
