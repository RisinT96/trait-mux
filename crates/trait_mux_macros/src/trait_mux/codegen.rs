use proc_macro::Ident;
use proc_macro2::{Span, TokenStream};
use quote::quote;

use crate::lower::Ir;

use super::lower::{AutorefSpecializer, Constraint, EnumVariant, Function, TraitAggregate};

fn refs(n: usize) -> TokenStream {
    let mut refs = TokenStream::new();
    for _ in 0..n {
        refs.extend(quote![&]);
    }
    refs
}

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

    result.extend(quote! {
        macro_rules! #into {
            ($var:tt) => {
                (#refs #wrap(&$var)).#into_tag().#into(&$var)
            }
        }
    });

    result
}

fn generate_wrap(ir: &Ir) -> TokenStream {
    let wrap = ir.wrap_ident;

    quote! {
        pub struct #wrap<'t, T>(pub &'t T);
    }
}

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
