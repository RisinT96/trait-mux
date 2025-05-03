use proc_macro2::TokenStream;
use quote::quote;

use crate::lower::Ir;

pub fn codegen(ir: Ir) -> TokenStream {
    let enum_ident = ir.enum_ident;

    let aggregate_traits = ir
        .enum_variants
        .iter()
        .map(|p| {
            let trait_ident = &p.ident;
            let sub_traits = p
                .implemented_traits
                .iter()
                .map(|s| &s.path)
                .collect::<Vec<_>>();

            if sub_traits.len() <= 1 {
                quote! {}
            } else {
                quote! {
                    pub trait #trait_ident: #(#sub_traits)+* {}
                    impl<T: #(#sub_traits)+*> #trait_ident for T {}
                }
            }
        })
        .collect::<Vec<_>>();

    let enum_fields = ir
        .enum_variants
        .iter()
        .map(|p| {
            let trait_ident = &p.ident;

            match p.implemented_traits.len() {
                0 => {
                    quote! {#trait_ident}
                }
                1 => {
                    let sub_trait = &p.implemented_traits.first().unwrap().path;
                    quote! {#trait_ident(&'t dyn #sub_trait)}
                }
                _ => {
                    quote! {#trait_ident(&'t dyn #trait_ident)}
                }
            }
        })
        .collect::<Vec<_>>();

    let enum_functions = ir
        .enum_impl_functions
        .iter()
        .map(|f| {
            let ident = &f.enum_variant;
            let result_path = &f.result_path;
            let variants = &f.variants;

            quote! {
                pub fn #ident(&self) -> core::option::Option<&dyn #result_path> {
                    match self {
                        #(#enum_ident::#variants(v) => Some(v),)*
                        _ => None,
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        #(#aggregate_traits)*

        pub enum #enum_ident<'t> {
            #(#enum_fields,)*
        }

        impl<'t> #enum_ident<'t> {
            #(#enum_functions)*
        }
    }
}
