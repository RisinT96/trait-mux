use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

mod trait_mux;
use crate::trait_mux::codegen;
use crate::trait_mux::lower;
use crate::trait_mux::parse;

#[proc_macro]
#[proc_macro_error]
pub fn trait_mux(ts: TokenStream) -> TokenStream {
    let ast = parse::parse(ts.clone().into());
    let ir = lower::lower(&ast);
    let ts = codegen::codegen(ir);
    ts.into()
}
