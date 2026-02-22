use proc_macro::TokenStream;

mod settings;

#[proc_macro_derive(Settings, attributes(setting))]
pub fn derive_settings(input: TokenStream) -> TokenStream {
    settings::derive(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
