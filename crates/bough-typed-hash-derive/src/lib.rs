use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// Derives `HashInto` by recursing over all fields (structs) or variants (enums).
///
/// Each field's `hash_into` is called in order. Enum variants are prefixed with
/// their discriminant index.
#[proc_macro_derive(HashInto)]
pub fn derive_hash_into(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let body = match &input.data {
        Data::Struct(data) => struct_hash_into_body(&data.fields),
        Data::Enum(data) => {
            let arms = data.variants.iter().enumerate().map(|(i, variant)| {
                let vname = &variant.ident;
                let disc = i as u32;
                match &variant.fields {
                    Fields::Unit => quote! {
                        Self::#vname => {
                            bough_typed_hash::HashInto::hash_into(&#disc, state)?;
                        }
                    },
                    Fields::Unnamed(fields) => {
                        let bindings: Vec<_> = (0..fields.unnamed.len())
                            .map(|i| format_ident!("f{i}"))
                            .collect();
                        let hash_fields = bindings.iter().map(|b| {
                            quote! { bough_typed_hash::HashInto::hash_into(#b, state)?; }
                        });
                        quote! {
                            Self::#vname(#(#bindings),*) => {
                                bough_typed_hash::HashInto::hash_into(&#disc, state)?;
                                #(#hash_fields)*
                            }
                        }
                    }
                    Fields::Named(fields) => {
                        let field_names: Vec<_> = fields.named.iter()
                            .map(|f| f.ident.as_ref().unwrap())
                            .collect();
                        let hash_fields = field_names.iter().map(|f| {
                            quote! { bough_typed_hash::HashInto::hash_into(#f, state)?; }
                        });
                        quote! {
                            Self::#vname { #(#field_names),* } => {
                                bough_typed_hash::HashInto::hash_into(&#disc, state)?;
                                #(#hash_fields)*
                            }
                        }
                    }
                }
            });
            quote! { match self { #(#arms)* } }
        }
        Data::Union(_) => panic!("HashInto cannot be derived for unions"),
    };

    let expanded = quote! {
        impl #impl_generics bough_typed_hash::HashInto for #name #ty_generics #where_clause {
            fn hash_into(&self, state: &mut bough_typed_hash::ShaState) -> ::std::result::Result<(), ::std::io::Error> {
                #body
                Ok(())
            }
        }
    };

    expanded.into()
}

/// Derives a full `TypedHash` implementation on a newtype wrapping `[u8; 32]`.
///
/// Generates: `TypedHash`, `HashInto`, `TypedHashable<Hash = Self>`, `Display`
/// (hex), `Serialize` (hex string), `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`.
///
/// Does **not** generate `FromStr`, `From<[u8; 32]>`, or `Deserialize`.
#[proc_macro_derive(TypedHash)]
pub fn derive_typed_hash(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let vis = &input.vis;

    let Data::Struct(ref data) = input.data else {
        panic!("TypedHash can only be derived for structs");
    };
    let Fields::Unnamed(ref fields) = data.fields else {
        panic!("TypedHash requires a tuple struct: struct Foo([u8; 32])");
    };
    assert!(
        fields.unnamed.len() == 1,
        "TypedHash requires exactly one field: [u8; 32]"
    );

    let _ = vis;

    let expanded = quote! {
        impl bough_typed_hash::TypedHash for #name {
            #[doc(hidden)]
            fn from_raw(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }

        impl bough_typed_hash::HashInto for #name {
            fn hash_into(&self, state: &mut bough_typed_hash::ShaState) -> ::std::result::Result<(), ::std::io::Error> {
                bough_typed_hash::sha2::Digest::update(state, self.0);
                Ok(())
            }
        }

        impl bough_typed_hash::TypedHashable for #name {
            type Hash = Self;
        }

        impl ::std::fmt::Display for #name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                for b in &self.0 {
                    write!(f, "{b:02x}")?;
                }
                Ok(())
            }
        }

        impl ::std::fmt::Debug for #name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}({})", stringify!(#name), self)
            }
        }

        impl ::std::clone::Clone for #name {
            fn clone(&self) -> Self {
                Self(self.0)
            }
        }

        impl ::std::cmp::PartialEq for #name {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }

        impl ::std::cmp::Eq for #name {}

        impl ::std::cmp::PartialOrd for #name {
            fn partial_cmp(&self, other: &Self) -> ::std::option::Option<::std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl ::std::cmp::Ord for #name {
            fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
                self.0.cmp(&other.0)
            }
        }

        impl ::std::marker::Copy for #name {}

        impl ::std::hash::Hash for #name {
            fn hash<H: ::std::hash::Hasher>(&self, hasher: &mut H) {
                self.0.hash(hasher);
            }
        }

        impl serde::Serialize for #name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.to_string())
            }
        }
    };

    expanded.into()
}

/// Derives `HashInto` (field recursion), auto-generates a `{TypeName}Hash` struct
/// with `#[derive(TypedHash)]`, and implements `TypedHashable`.
///
/// Generic type parameters are joined into the hash struct name:
/// `Mutation<L>` → `MutationLHash`.
#[proc_macro_derive(TypedHashable)]
pub fn derive_typed_hashable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let vis = &input.vis;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let param_names: String = input.generics.type_params()
        .map(|p| p.ident.to_string())
        .collect();
    let hash_name = format_ident!("{}{param_names}Hash", name);

    let hash_into_body = match &input.data {
        Data::Struct(data) => struct_hash_into_body(&data.fields),
        Data::Enum(data) => {
            let arms = data.variants.iter().enumerate().map(|(i, variant)| {
                let vname = &variant.ident;
                let disc = i as u32;
                match &variant.fields {
                    Fields::Unit => quote! {
                        Self::#vname => {
                            bough_typed_hash::HashInto::hash_into(&#disc, state)?;
                        }
                    },
                    Fields::Unnamed(fields) => {
                        let bindings: Vec<_> = (0..fields.unnamed.len())
                            .map(|i| format_ident!("f{i}"))
                            .collect();
                        let hash_fields = bindings.iter().map(|b| {
                            quote! { bough_typed_hash::HashInto::hash_into(#b, state)?; }
                        });
                        quote! {
                            Self::#vname(#(#bindings),*) => {
                                bough_typed_hash::HashInto::hash_into(&#disc, state)?;
                                #(#hash_fields)*
                            }
                        }
                    }
                    Fields::Named(fields) => {
                        let field_names: Vec<_> = fields.named.iter()
                            .map(|f| f.ident.as_ref().unwrap())
                            .collect();
                        let hash_fields = field_names.iter().map(|f| {
                            quote! { bough_typed_hash::HashInto::hash_into(#f, state)?; }
                        });
                        quote! {
                            Self::#vname { #(#field_names),* } => {
                                bough_typed_hash::HashInto::hash_into(&#disc, state)?;
                                #(#hash_fields)*
                            }
                        }
                    }
                }
            });
            quote! { match self { #(#arms)* } }
        }
        Data::Union(_) => panic!("TypedHashable cannot be derived for unions"),
    };

    let expanded = quote! {
        #[derive(bough_typed_hash::TypedHash)]
        #vis struct #hash_name([u8; 32]);

        impl #impl_generics bough_typed_hash::HashInto for #name #ty_generics #where_clause {
            fn hash_into(&self, state: &mut bough_typed_hash::ShaState) -> ::std::result::Result<(), ::std::io::Error> {
                #hash_into_body
                Ok(())
            }
        }

        impl #impl_generics bough_typed_hash::TypedHashable for #name #ty_generics #where_clause {
            type Hash = #hash_name;
        }
    };

    expanded.into()
}

fn struct_hash_into_body(fields: &Fields) -> proc_macro2::TokenStream {
    match fields {
        Fields::Named(fields) => {
            let hash_fields = fields.named.iter().map(|f| {
                let name = f.ident.as_ref().unwrap();
                quote! { bough_typed_hash::HashInto::hash_into(&self.#name, state)?; }
            });
            quote! { #(#hash_fields)* }
        }
        Fields::Unnamed(fields) => {
            let hash_fields = (0..fields.unnamed.len()).map(|i| {
                let idx = syn::Index::from(i);
                quote! { bough_typed_hash::HashInto::hash_into(&self.#idx, state)?; }
            });
            quote! { #(#hash_fields)* }
        }
        Fields::Unit => quote! {},
    }
}
