use proc_macro::TokenStream;

#[proc_macro]
pub fn mikrotik_model(item: TokenStream) -> TokenStream {
    match mikrotik_model_generator::macros::mikrotik_model(item.into()) {
        Ok(value) => value.into(),
        Err(e) => e.write_errors().into(),
    }
}
