use convert_case::{Case, Casing};
use std::{fmt::format, ops::Deref, path};

use proc_macro2::{Ident, Span};
use syn::{Path, spanned::Spanned};

use crate::parse::Ast;

pub struct Ir<'t> {
    pub ident: &'t Ident,
    pub traits: Vec<Trait<'t>>,
    pub permutations: Vec<Permutation<'t>>,
    pub functions: Vec<Function<'t>>,
}

#[derive(Copy, Clone)]
pub struct Trait<'t> {
    pub ident: &'t Ident,
    pub path: &'t Path,
}

impl<'t> std::fmt::Debug for Trait<'t> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Trait").field("ident", &self.ident).finish()
    }
}

pub struct Permutation<'t> {
    pub ident: Ident,
    pub match_ident: Ident,
    pub implemented_traits: Vec<Trait<'t>>,
}

impl<'t> std::fmt::Debug for Permutation<'t> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Permutation")
            .field("ident", &self.ident)
            .field("match_ident", &self.match_ident)
            .field("implemented_traits", &self.implemented_traits)
            .finish()
    }
}

pub struct Function<'t> {
    pub ident: Ident,
    pub result_path: &'t Path,
    pub variants: Vec<Ident>,
}

fn extract_traits<'t>(ast: &'t Ast) -> Vec<Trait<'t>> {
    let mut traits = vec![];

    for path in &ast.paths {
        if path.segments.is_empty() {
            proc_macro_error::emit_error!(
                path.span(),
                "unexpected end of input, expected identifier"
            );
        }

        // unwrap safety: checked that segments is not empty.
        traits.push(Trait {
            ident: &path.segments.last().unwrap().ident,
            path,
        });
    }

    traits.sort_by_key(|t| t.ident.to_string());
    traits
}

fn generate_permutations<'t>(traits: &Vec<Trait<'t>>) -> Vec<Permutation<'t>> {
    let mut permutations = Vec::new();
    let n = traits.len();

    // Create all possible permutations of the trait names.
    // We have 2^n possible permutations
    // Since trait_names is sorted, output should also be sorted.
    for i in 0..(1 << n) {
        let mut permutation = vec![];

        for (j, r#trait) in traits.iter().enumerate() {
            if (i & (1 << j)) != 0 {
                permutation.push(r#trait.clone());
            }
        }
        permutations.push(permutation);
    }

    let mut permutations = permutations
        .iter()
        .map(|p| {
            let permutation_name = if p.is_empty() {
                "None".to_string()
            } else {
                p.iter().map(|t| t.ident.to_string()).collect::<String>()
            };
            let permutation_match_name = format!("Match{}", permutation_name);

            Permutation {
                ident: Ident::new(&permutation_name, Span::call_site()),
                match_ident: Ident::new(&permutation_match_name, Span::call_site()),
                implemented_traits: p.to_vec(),
            }
        })
        .collect::<Vec<_>>();

    // We want the list to be sorted descending by amount of traits, this is important as we'll be
    // using autoref-specialization, where the order is very important, as the first match (even not
    // full match) will be the one that dictates the output.
    // We'll sort by the length, then alphabetically
    permutations.sort_by_key(|e| format!("{} {}", e.implemented_traits.len(), e.ident.to_string()));
    permutations.reverse();

    permutations
}

pub fn generate_functions<'t, 'p>(
    traits: &'p [Trait<'t>],
    permutations: &'p [Permutation<'t>],
) -> Vec<Function<'t>> {
    traits
        .iter()
        .map(|t| {
            let name = format!("try_as_{}", t.ident.to_string().to_case(Case::Snake));

            // We want to find all enum variants that implement the trait `t`.
            let variants = permutations
                .iter()
                .filter(|p| {
                    p.implemented_traits
                        .iter()
                        .find(|it| core::ptr::eq(it.path, t.path))
                        .is_some()
                })
                .map(|p| p.ident.clone())
                .collect();

            Function {
                ident: Ident::new(&name, Span::call_site()),
                result_path: t.path,
                variants,
            }
        })
        .collect()
}

pub fn lower<'t>(ast: &'t Ast) -> Ir<'t> {
    let traits = extract_traits(&ast);
    let permutations = generate_permutations(&traits);
    let functions = generate_functions(&traits, &permutations);

    Ir {
        ident: &ast.name,
        traits,
        permutations,
        functions,
    }
}
