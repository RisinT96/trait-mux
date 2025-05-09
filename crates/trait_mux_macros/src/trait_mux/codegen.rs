//! This module is responsible for generating Rust code from the lowered intermediate
//! representation (IR) produced during the macro processing phase.

use proc_macro2::TokenStream;
use quote::quote;

use crate::lower::{AutorefSpecializer, Constraint, EnumVariant, Function, Ir, TraitAggregate};

/// Creates a TokenStream containing a sequence of `n` reference operators (`&`).
///
/// # Arguments
///
/// * `n` - The number of reference operators to generate
///
/// # Returns
///
/// A TokenStream containing `n` consecutive `&` operators
fn refs(n: usize) -> TokenStream {
    let mut refs = TokenStream::new();
    for _ in 0..n {
        refs.extend(quote![&]);
    }
    refs
}

/// Generates the complete Rust code from the intermediate representation.
///
/// This function orchestrates the code generation by combining all the different
/// code elements created by the specialized generator functions.
///
/// # Arguments
///
/// * `ir` - The intermediate representation to generate code from
///
/// # Returns
///
/// A TokenStream containing all the generated code
pub fn codegen(ir: Ir) -> TokenStream {
    let mut result = TokenStream::new();

    result.extend(generate_wrap(&ir));
    result.extend(generate_trait_aggregates(&ir));
    result.extend(generate_enum(&ir));
    result.extend(generate_enum_impl(&ir));
    result.extend(generate_autoref_specializers(&ir));

    let into = &ir.into;
    let into_tag = &ir.into_tag;
    let refs = refs(ir.wrap_derefs);
    let wrap = &ir.wrap_ident;

    // Generate a helper macro to convert values into the enum
    result.extend(quote! {
        macro_rules! #into {
            ($var:tt) => {
                (#refs #wrap(&$var)).#into_tag().#into(&$var)
            }
        }
    });

    result
}

/// Generates the wrapper struct that holds a reference to the original value.
/// The wrapper is necessary to support proper specialization for the original
/// type, and not its reference.
///
/// # Arguments
///
/// * `ir` - The intermediate representation containing the wrap identifier
///
/// # Returns
///
/// A TokenStream for the wrapper struct definition
fn generate_wrap(ir: &Ir) -> TokenStream {
    let wrap = ir.wrap_ident;

    quote! {
        pub struct #wrap<'t, T>(pub &'t T);
    }
}

/// Generates trait aggregates that combine multiple traits into a single trait.
///
/// # Arguments
///
/// * `ir` - The intermediate representation containing trait aggregates
///
/// # Returns
///
/// A TokenStream for all trait aggregate definitions and their implementations
fn generate_trait_aggregates(ir: &Ir) -> TokenStream {
    let mut trait_aggregates = TokenStream::new();

    ir.trait_aggregates
        .iter()
        .map(|TraitAggregate { name, traits }| {
            let traits: Vec<_> = traits.iter().map(|t| t.path).collect();

            trait_aggregates.extend(quote! {
                pub trait #name: #(#traits)+* {}
                impl<T: #(#traits)+*> #name for T {}
            });
        })
        .count();

    trait_aggregates
}

/// Generates the enum definition based on the intermediate representation.
///
/// # Arguments
///
/// * `ir` - The intermediate representation containing the enum definition
///
/// # Returns
///
/// A TokenStream for the enum definition
fn generate_enum(ir: &Ir) -> TokenStream {
    let enum_name = ir.r#enum.name;

    let mut enum_fields = TokenStream::new();

    for EnumVariant { ident, constraint } in &ir.r#enum.variants {
        let constraint = match constraint {
            Constraint::None => quote! {},
            Constraint::Path(path) => quote! {(&'t dyn #path)},
            Constraint::Ident(ident) => quote! {(&'t dyn #ident)},
        };

        enum_fields.extend(quote! {
            #ident #constraint,
        });
    }

    quote! {
        pub enum #enum_name<'t> {
            #enum_fields
        }
    }
}

/// Generates the implementation of the enum, including methods for accessing
/// the enum variants.
///
/// # Arguments
///
/// * `ir` - The intermediate representation containing the enum implementation
///
/// # Returns
///
/// A TokenStream for the enum implementation
fn generate_enum_impl(ir: &Ir) -> TokenStream {
    let enum_name = ir.r#enum.name;

    let mut fns = TokenStream::new();

    for Function {
        name,
        result_path,
        matching_variants,
    } in &ir.enum_impl.functions
    {
        fns.extend(quote! {
            pub fn #name(&self) -> ::core::option::Option<&dyn #result_path> {
                match self {
                    #(#enum_name::#matching_variants (v) => Some(v),)*
                    _ => None,
                }
            }
        });
    }

    quote! {
        impl<'t> #enum_name<'t> {
            #fns
        }
    }
}

/// Generates the autoref specializers, which are responsible for automatically
/// referencing values and converting them into the appropriate enum variants.
///
/// # Arguments
///
/// * `ir` - The intermediate representation containing the autoref specializers
///
/// # Returns
///
/// A TokenStream for all autoref specializer definitions and their implementations
fn generate_autoref_specializers(ir: &Ir) -> TokenStream {
    let mut autoref_specializers = TokenStream::new();

    let enum_name = ir.r#enum.name;
    let wrap = ir.wrap_ident;
    let into = &ir.into;
    let into_tag = &ir.into_tag;

    ir.autoref_specializers
        .iter()
        .map(
            |AutorefSpecializer {
                 tag,
                 r#match,
                 deref_count,
                 variant,
                 constraint,
             }| {
                let refs = refs(*deref_count);

                let t_constraint = match constraint {
                    Constraint::None => quote! {},
                    Constraint::Path(path) => quote! {: #path},
                    Constraint::Ident(ident) => quote! {: #ident},
                };

                let param = match constraint {
                    Constraint::None => quote! {},
                    Constraint::Path(_) | Constraint::Ident(_) => quote! {(v)},
                };

                autoref_specializers.extend(quote! {
                    pub struct #tag;
                    impl #tag {
                        pub fn #into<T #t_constraint>(self, v: &T) -> #enum_name {
                            #enum_name::#variant #param
                        }
                    }

                    pub trait #r#match<T> {
                        fn #into_tag(&self) -> #tag;
                    }
                    impl<'t, T #t_constraint> #r#match<T> for #refs #wrap<'t,T> {
                        fn #into_tag(&self) -> #tag {
                            #tag
                        }
                    }
                });
            },
        )
        .count();

    autoref_specializers
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::trait_mux::analyze::Trait;

    use super::*;
    use proc_macro2::Span;
    use syn::{Ident, Path, parse_quote};

    fn create_idents() -> HashMap<&'static str, Ident> {
        let mut res = HashMap::new();

        res.insert("Wrap", Ident::new("Wrap", Span::call_site()));
        res.insert("into", Ident::new("into", Span::call_site()));
        res.insert("Combined", Ident::new("Combined", Span::call_site()));
        res.insert("Dispatcher", Ident::new("Dispatcher", Span::call_site()));
        res.insert("Debug", Ident::new("Debug", Span::call_site()));
        res.insert("Display", Ident::new("Display", Span::call_site()));
        res.insert(
            "DebugDisplay",
            Ident::new("DebugDisplay", Span::call_site()),
        );

        res
    }

    fn create_paths() -> HashMap<&'static str, Path> {
        let mut res = HashMap::new();

        res.insert("std::fmt::Debug", parse_quote!(std::fmt::Debug));
        res.insert("std::fmt::Display", parse_quote!(std::fmt::Display));

        res
    }

    fn create_traits<'t>(
        idents: &'t HashMap<&'static str, Path>,
    ) -> HashMap<&'static str, Trait<'t>> {
        idents
            .iter()
            .map(|(&k, v)| {
                (
                    k,
                    Trait {
                        ident: &v.segments.last().unwrap().ident,
                        path: v,
                    },
                )
            })
            .collect()
    }

    // Helper function to create a simple IR for testing
    fn create_test_ir<'t>(
        idents: &'t HashMap<&str, Ident>,
        paths: &'t HashMap<&str, Path>,
        traits: &'t HashMap<&str, Trait<'t>>,
    ) -> Ir<'t> {
        Ir {
            wrap_ident: &idents["Wrap"],
            wrap_derefs: 1,
            into: Ident::new("into", Span::call_site()),
            into_tag: Ident::new("into_tag", Span::call_site()),
            trait_aggregates: vec![TraitAggregate {
                name: &idents["Combined"],
                traits: vec![&traits["std::fmt::Debug"], &traits["std::fmt::Display"]],
            }],
            r#enum: crate::lower::Enum {
                name: &idents["Dispatcher"],
                variants: vec![
                    EnumVariant {
                        ident: &idents["Debug"],
                        constraint: Constraint::Path(&paths["std::fmt::Debug"]),
                    },
                    EnumVariant {
                        ident: &idents["Display"],
                        constraint: Constraint::Path(&paths["std::fmt::Display"]),
                    },
                    EnumVariant {
                        ident: &idents["DebugDisplay"],
                        constraint: Constraint::Ident(&idents["DebugDisplay"]),
                    },
                ],
            },
            enum_impl: crate::lower::EnumImpl {
                functions: vec![
                    Function {
                        name: Ident::new("as_debug", Span::call_site()),
                        result_path: &paths["std::fmt::Debug"],
                        matching_variants: vec![&idents["Debug"], &idents["DebugDisplay"]],
                    },
                    Function {
                        name: Ident::new("as_display", Span::call_site()),
                        result_path: &paths["std::fmt::Display"],
                        matching_variants: vec![&idents["Display"], &idents["DebugDisplay"]],
                    },
                ],
            },
            autoref_specializers: vec![
                AutorefSpecializer {
                    tag: Ident::new("DebugDisplayTag", Span::call_site()),
                    r#match: Ident::new("DebugDisplayMatch", Span::call_site()),
                    deref_count: 2,
                    variant: &idents["DebugDisplay"],
                    constraint: Constraint::Ident(&idents["DebugDisplay"]),
                },
                AutorefSpecializer {
                    tag: Ident::new("DebugTag", Span::call_site()),
                    r#match: Ident::new("DebugMatch", Span::call_site()),
                    deref_count: 1,
                    variant: &idents["Debug"],
                    constraint: Constraint::Path(&paths["std::fmt::Debug"]),
                },
                AutorefSpecializer {
                    tag: Ident::new("DisplayTag", Span::call_site()),
                    r#match: Ident::new("DisplayMatch", Span::call_site()),
                    deref_count: 1,
                    variant: &idents["Display"],
                    constraint: Constraint::Path(&paths["std::fmt::Display"]),
                },
            ],
        }
    }

    #[test]
    fn test_refs() {
        let result = refs(3);
        let expected = quote!(& & &);
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn test_generate_wrap() {
        let idents = create_idents();
        let paths = create_paths();
        let traits = create_traits(&paths);
        let ir = create_test_ir(&idents, &paths, &traits);

        let result = generate_wrap(&ir);
        let expected = quote! {
            pub struct Wrap<'t, T>(pub &'t T);
        };
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn test_generate_trait_aggregates() {
        let idents = create_idents();
        let paths = create_paths();
        let traits = create_traits(&paths);
        let ir = create_test_ir(&idents, &paths, &traits);

        let result = generate_trait_aggregates(&ir);
        let expected = quote! {
            pub trait Combined: std::fmt::Debug + std::fmt::Display {}
            impl<T: std::fmt::Debug + std::fmt::Display> Combined for T {}
        };
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn test_generate_enum() {
        let idents = create_idents();
        let paths = create_paths();
        let traits = create_traits(&paths);
        let ir = create_test_ir(&idents, &paths, &traits);

        let result = generate_enum(&ir);
        let expected = quote! {
            pub enum Dispatcher<'t> {
                Debug (&'t dyn std::fmt::Debug),
                Display (&'t dyn std::fmt::Display),
                DebugDisplay (&'t dyn DebugDisplay),
            }
        };
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn test_generate_enum_impl() {
        let idents = create_idents();
        let paths = create_paths();
        let traits = create_traits(&paths);
        let ir = create_test_ir(&idents, &paths, &traits);

        let result = generate_enum_impl(&ir);
        let expected = quote! {
            impl<'t> Dispatcher<'t> {
                pub fn as_debug(&self) -> ::core::option::Option<&dyn std::fmt::Debug> {
                    match self {
                        Dispatcher::Debug(v) => Some(v),
                        Dispatcher::DebugDisplay(v) => Some(v),
                        _ => None,
                    }
                }
                pub fn as_display(&self) -> ::core::option::Option<&dyn std::fmt::Display> {
                    match self {
                        Dispatcher::Display(v) => Some(v),
                        Dispatcher::DebugDisplay(v) => Some(v),
                        _ => None,
                    }
                }
            }
        };
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn test_generate_autoref_specializers() {
        let idents = create_idents();
        let paths = create_paths();
        let traits = create_traits(&paths);
        let ir = create_test_ir(&idents, &paths, &traits);

        let result = generate_autoref_specializers(&ir);

        let expected_structs = vec![
            quote! {
                pub struct DebugDisplayTag;
            },
            quote! {
                pub struct DebugTag;
            },
            quote! {
                pub struct DisplayTag;
            },
        ];

        let expected_struct_impls = vec![
            quote! {
                impl DebugDisplayTag {
                    pub fn into<T: DebugDisplay>(self, v: &T) -> Dispatcher {
                        Dispatcher::DebugDisplay(v)
                    }
                }
            },
            quote! {
                impl DebugTag {
                    pub fn into<T: std::fmt::Debug>(self, v: &T) -> Dispatcher {
                        Dispatcher::Debug(v)
                    }
                }
            },
            quote! {
                impl DisplayTag {
                    pub fn into<T: std::fmt::Display>(self, v: &T) -> Dispatcher {
                        Dispatcher::Display(v)
                    }
                }
            },
        ];

        let expected_traits = vec![
            quote! {
                pub trait DebugDisplayMatch<T> {
                    fn into_tag(&self) -> DebugDisplayTag;
                }
            },
            quote! {
                pub trait DebugMatch<T> {
                    fn into_tag(&self) -> DebugTag;
                }
            },
            quote! {
                pub trait DisplayMatch<T> {
                    fn into_tag(&self) -> DisplayTag;
                }
            },
        ];

        let expected_trait_impls: Vec<TokenStream> = vec![
            quote! {
                impl<'t, T: DebugDisplay> DebugDisplayMatch<T> for & & Wrap<'t,T> {
                    fn into_tag(&self) -> DebugDisplayTag {
                        DebugDisplayTag
                    }
                }
            },
            quote! {
                impl<'t, T: std::fmt::Debug> DebugMatch<T> for & Wrap<'t,T> {
                    fn into_tag(&self) -> DebugTag {
                        DebugTag
                    }
                }
            },
            quote! {
                impl<'t, T: std::fmt::Display> DisplayMatch<T> for & Wrap<'t,T> {
                    fn into_tag(&self) -> DisplayTag {
                        DisplayTag
                    }
                }
            },
        ];

        // Check for expected substrings to make test less brittle
        let result_str = result.to_string();

        for expected in expected_structs {
            assert!(result_str.contains(&expected.to_string()));
        }

        for expected in expected_struct_impls {
            assert!(result_str.contains(&expected.to_string()));
        }

        for expected in expected_traits {
            assert!(result_str.contains(&expected.to_string()));
        }

        for expected in expected_trait_impls {
            assert!(result_str.contains(&expected.to_string()));
        }
    }

    #[test]
    fn test_codegen() {
        let idents = create_idents();
        let paths = create_paths();
        let traits = create_traits(&paths);
        let ir = create_test_ir(&idents, &paths, &traits);

        let result = codegen(ir);

        // Just verify that the output contains expected important elements
        let result_str = result.to_string();

        assert!(result_str.contains(&quote! {pub struct Wrap}.to_string()));
        assert!(result_str.contains(&quote! {pub trait Combined}.to_string()));
        assert!(result_str.contains(&quote! {pub enum Dispatcher}.to_string()));
        assert!(result_str.contains(&quote! {impl<'t> Dispatcher<'t>}.to_string()));
        assert!(result_str.contains(&quote! {macro_rules! into}.to_string()));
    }
}
