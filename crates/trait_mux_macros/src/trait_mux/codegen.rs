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
