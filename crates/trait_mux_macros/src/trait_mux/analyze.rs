use proc_macro2::{Ident, Span};
use syn::{Path, spanned::Spanned};

use crate::parse::Ast;

pub struct Model<'t> {
    pub enum_ident: &'t Ident,
    pub enum_variants: Vec<EnumVariant<'t>>,
    pub wrap_ident: Ident,
    pub traits: Vec<Trait<'t>>,
}

/// Represents a trait with its identifier and path.
#[derive(Copy, Clone)]
pub struct Trait<'t> {
    pub ident: &'t Ident,
    pub path: &'t Path,
}

/// Represents an enum variant, including its identifier, and the traits it implements.
pub struct EnumVariant<'t> {
    /// The enum ident, used both for the enum variant name, and (if needed) for the trait aggregate
    /// name.
    /// e.g. `BinaryDebugDisplay`
    pub ident: Ident,
    /// The traits that this enum implements.
    pub implemented_traits: Vec<Trait<'t>>,
}

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

/// Extracts traits from the given AST.
/// Emits an error if a path is empty or malformed.
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
