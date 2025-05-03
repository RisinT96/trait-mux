use convert_case::{Case, Casing};

use proc_macro2::{Ident, Span};
use syn::{Path, spanned::Spanned};

use crate::parse::Ast;

/// Intermediate representation (IR) of the parsed AST.
/// Contains the identifier, enum variants, and functions derived from the AST.
pub struct Ir<'t> {
    pub enum_ident: &'t Ident,
    pub enum_variants: Vec<EnumVariant<'t>>,
    pub enum_impl_functions: Vec<Function<'t>>,
}

/// Represents a trait with its identifier and path.
#[derive(Copy, Clone)]
pub struct Trait<'t> {
    pub ident: &'t Ident,
    pub path: &'t Path,
}

/// Represents an enum variant, including its identifier,
/// match identifier, and the traits it implements.
pub struct EnumVariant<'t> {
    pub ident: Ident,
    pub implemented_traits: Vec<Trait<'t>>,
}

/// Represents a function derived from a trait, including its identifier,
/// result path, and the variants it applies to.
pub struct Function<'t> {
    pub enum_variant: Ident,
    pub result_path: &'t Path,
    pub variants: Vec<Ident>,
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
fn generate_variants<'t>(traits: &Vec<Trait<'t>>) -> Vec<EnumVariant<'t>> {
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

/// Generates functions for each trait, mapping them to the enum variants
/// that implement the trait.
pub fn generate_enum_impl_functions<'t, 'p>(
    traits: &'p [Trait<'t>],
    variants: &'p [EnumVariant<'t>],
) -> Vec<Function<'t>> {
    traits
        .iter()
        .map(|t| {
            let name = format!("try_as_{}", t.ident.to_string().to_case(Case::Snake));

            // Find all enum variants that implement the trait `t`.
            let variants = variants
                .iter()
                .filter(|p| {
                    p.implemented_traits
                        .iter()
                        .any(|it| core::ptr::eq(it.path, t.path))
                })
                .map(|p| p.ident.clone())
                .collect();

            Function {
                enum_variant: Ident::new(&name, Span::call_site()),
                result_path: t.path,
                variants,
            }
        })
        .collect()
}

/// Converts the given AST into its intermediate representation (IR).
/// This includes extracting traits, generating enum variants, and creating functions.
pub fn lower(ast: &Ast) -> Ir {
    let traits = extract_traits(ast);
    let variants = generate_variants(&traits);
    let enum_impl_functions = generate_enum_impl_functions(&traits, &variants);

    Ir {
        enum_ident: &ast.name,
        enum_variants: variants,
        enum_impl_functions,
    }
}
