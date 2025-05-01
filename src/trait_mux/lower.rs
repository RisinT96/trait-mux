use std::collections::HashSet;

use syn::spanned::Spanned;

use crate::parse::Ast;

pub struct Ir {
    pub enum_name: String,
    pub trait_names: Vec<String>,
    pub permutations: Vec<HashSet<String>>,
}

fn extract_trait_names(ast: &Ast) -> Vec<String> {
    let mut trait_names = vec![];

    for path in &ast.paths {
        if path.segments.is_empty() {
            proc_macro_error::emit_error!(
                path.span(),
                "unexpected end of input, expected identifier"
            );
        }

        // unwrap safety: checked that segments is not empty.
        trait_names.push(path.segments.last().unwrap().ident.to_string());
    }

    trait_names.sort();
    trait_names
}

fn generate_permutations(trait_names: &Vec<String>) -> Vec<HashSet<String>> {
    let mut permutations = Vec::new();
    let n = trait_names.len();

    // Create all possible permutations of the trait names.
    // We have 2^n possible permutations
    for i in 0..(1 << n) {
        let mut permutation = HashSet::new();

        for (j, name) in trait_names.iter().enumerate() {
            if (i & (1 << j)) != 0 {
                permutation.insert(name.clone());
            }
        }
        permutations.push(permutation);
    }

    // We want the list to be sorted descending by amount of traits, this is important as we'll be
    // using `spez` to generate specialization, where the order is very important, as the first
    // match (even not full match) will be the one that dictates the output.
    permutations.sort_by_key(|e| e.len());
    permutations.reverse();

    permutations
}

pub fn lower(ast: Ast) -> Ir {
    let trait_names = extract_trait_names(&ast);
    let permutations = generate_permutations(&trait_names);

    Ir {
        enum_name: ast.name.to_string(),
        trait_names,
        permutations,
    }
}
