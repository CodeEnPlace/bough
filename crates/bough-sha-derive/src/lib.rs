use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

#[proc_macro_derive(ShaHashable)]
pub fn derive_sha_hashable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let body = match &input.data {
        Data::Struct(data) => struct_body(&data.fields),
        Data::Enum(data) => {
            let arms = data.variants.iter().enumerate().map(|(i, variant)| {
                let vname = &variant.ident;
                let disc = i as u32;
                match &variant.fields {
                    Fields::Unit => quote! {
                        Self::#vname => {
                            bough_sha::ShaHashable::sha_hash_into(&#disc, state);
                        }
                    },
                    Fields::Unnamed(fields) => {
                        let bindings: Vec<_> = (0..fields.unnamed.len())
                            .map(|i| syn::Ident::new(&format!("f{i}"), proc_macro2::Span::call_site()))
                            .collect();
                        let hash_fields = bindings.iter().map(|b| {
                            quote! { bough_sha::ShaHashable::sha_hash_into(#b, state); }
                        });
                        quote! {
                            Self::#vname(#(#bindings),*) => {
                                bough_sha::ShaHashable::sha_hash_into(&#disc, state);
                                #(#hash_fields)*
                            }
                        }
                    }
                    Fields::Named(fields) => {
                        let field_names: Vec<_> = fields.named.iter()
                            .map(|f| f.ident.as_ref().unwrap())
                            .collect();
                        let hash_fields = field_names.iter().map(|f| {
                            quote! { bough_sha::ShaHashable::sha_hash_into(#f, state); }
                        });
                        quote! {
                            Self::#vname { #(#field_names),* } => {
                                bough_sha::ShaHashable::sha_hash_into(&#disc, state);
                                #(#hash_fields)*
                            }
                        }
                    }
                }
            });
            quote! { match self { #(#arms)* } }
        }
        Data::Union(_) => panic!("ShaHashable cannot be derived for unions"),
    };

    let expanded = quote! {
        impl #impl_generics bough_sha::ShaHashable for #name #ty_generics #where_clause {
            fn sha_hash_into(&self, state: &mut bough_sha::ShaState) {
                #body
            }
        }
    };

    expanded.into()
}

fn struct_body(fields: &Fields) -> proc_macro2::TokenStream {
    match fields {
        Fields::Named(fields) => {
            let hash_fields = fields.named.iter().map(|f| {
                let name = f.ident.as_ref().unwrap();
                quote! { bough_sha::ShaHashable::sha_hash_into(&self.#name, state); }
            });
            quote! { #(#hash_fields)* }
        }
        Fields::Unnamed(fields) => {
            let hash_fields = (0..fields.unnamed.len()).map(|i| {
                let idx = syn::Index::from(i);
                quote! { bough_sha::ShaHashable::sha_hash_into(&self.#idx, state); }
            });
            quote! { #(#hash_fields)* }
        }
        Fields::Unit => quote! {},
    }
}
