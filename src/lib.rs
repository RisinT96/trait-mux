use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

mod trait_mux;
use crate::trait_mux::analyze;
use crate::trait_mux::codegen;
use crate::trait_mux::lower;
use crate::trait_mux::parse;

#[proc_macro]
#[proc_macro_error]
pub fn trait_mux(ts: TokenStream) -> TokenStream {
    let ast = parse::parse(ts.clone().into());
    let model = analyze::analyze(ast);
    let ir = lower::lower(model);
    let _ = codegen::codegen(ir);
    TokenStream::new()
}
