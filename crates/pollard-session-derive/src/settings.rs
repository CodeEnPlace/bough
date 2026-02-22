use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Field, Fields, GenericArgument, Lit, PathArguments, Type,
    parse2,
};

struct FieldInfo {
    ident: syn::Ident,
    ty: Type,
    is_option: bool,
    is_vec: bool,
    is_bool: bool,
    skip: bool,
    cli_only: bool,
    flatten: bool,
    default_expr: Option<TokenStream>,
    env: Option<String>,
    long: Option<String>,
}

fn extract_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty {
        let seg = type_path.path.segments.last()?;
        if seg.ident == "Option" || seg.ident == "Vec" {
            if let PathArguments::AngleBracketed(args) = &seg.arguments {
                if let Some(GenericArgument::Type(inner)) = args.args.first() {
                    return Some(inner);
                }
            }
        }
    }
    None
}

fn is_bare_type(ty: &Type, name: &str) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(seg) = type_path.path.segments.last() {
            return seg.ident == name && seg.arguments.is_none();
        }
    }
    false
}

fn is_type_wrapper(ty: &Type, name: &str) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(seg) = type_path.path.segments.last() {
            return seg.ident == name;
        }
    }
    false
}

fn parse_field(field: &Field) -> syn::Result<FieldInfo> {
    let ident = field.ident.clone().expect("named fields only");
    let ty = field.ty.clone();
    let is_option = is_type_wrapper(&ty, "Option");
    let is_vec = is_type_wrapper(&ty, "Vec");
    let is_bool = is_bare_type(&ty, "bool");

    let mut skip = false;
    let mut cli_only = false;
    let mut flatten = false;
    let mut default_expr = None;
    let mut env = None;
    let mut long = None;

    for attr in &field.attrs {
        if !attr.path().is_ident("setting") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                skip = true;
            } else if meta.path.is_ident("cli_only") {
                cli_only = true;
            } else if meta.path.is_ident("flatten") {
                flatten = true;
            } else if meta.path.is_ident("default") {
                let value = meta.value()?;
                let lit: Lit = value.parse()?;
                if let Lit::Str(s) = lit {
                    let expr: TokenStream = s.parse()?;
                    default_expr = Some(expr);
                }
            } else if meta.path.is_ident("env") {
                let value = meta.value()?;
                let lit: Lit = value.parse()?;
                if let Lit::Str(s) = lit {
                    env = Some(s.value());
                }
            } else if meta.path.is_ident("long") {
                let value = meta.value()?;
                let lit: Lit = value.parse()?;
                if let Lit::Str(s) = lit {
                    long = Some(s.value());
                }
            }
            Ok(())
        })?;
    }

    Ok(FieldInfo {
        ident,
        ty,
        is_option,
        is_vec,
        is_bool,
        skip,
        cli_only,
        flatten,
        default_expr,
        env,
        long,
    })
}

fn partial_ident(original: &syn::Ident) -> syn::Ident {
    format_ident!("Partial{}", original)
}

pub fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = parse2(input)?;
    let vis = &input.vis;
    let name = &input.ident;
    let partial_name = partial_ident(name);

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => &named.named,
            _ => return Err(syn::Error::new_spanned(&input, "expected named fields")),
        },
        _ => return Err(syn::Error::new_spanned(&input, "expected struct")),
    };

    let parsed: Vec<FieldInfo> = fields.iter().map(parse_field).collect::<syn::Result<_>>()?;

    let partial_fields = gen_partial_fields(&parsed);
    let merge_fields = gen_merge_fields(&parsed);
    let resolve_fields = gen_resolve_fields(&parsed, name);
    let skipped_fields = gen_skipped_struct(&parsed, name, vis);

    Ok(quote! {
        #[derive(Default, Clone, Debug, clap::Args, serde::Deserialize)]
        #[serde(default)]
        #vis struct #partial_name {
            #(#partial_fields)*
        }

        #skipped_fields

        impl #partial_name {
            #vis fn merge(self, fallback: Self) -> Self {
                Self {
                    #(#merge_fields)*
                }
            }

            #resolve_fields
        }
    })
}

fn gen_partial_fields(fields: &[FieldInfo]) -> Vec<TokenStream> {
    fields
        .iter()
        .filter(|f| !f.skip)
        .map(|f| {
            let ident = &f.ident;
            let long_name = f.long.clone().unwrap_or_else(|| {
                ident.to_string().replace('_', "-")
            });

            if f.flatten {
                let inner = extract_inner_type(&f.ty).unwrap_or(&f.ty);
                let partial_ty = if let Type::Path(tp) = inner {
                    let seg = tp.path.segments.last().unwrap();
                    let partial = format_ident!("Partial{}", seg.ident);
                    quote! { #partial }
                } else {
                    quote! { #inner }
                };

                quote! {
                    #[command(flatten)]
                    #[serde(default)]
                    pub #ident: #partial_ty,
                }
            } else if f.is_vec {
                let inner = extract_inner_type(&f.ty).unwrap_or(&f.ty);
                let serde_skip = if f.cli_only {
                    quote! { #[serde(skip)] }
                } else {
                    quote! { #[serde(default)] }
                };
                let env_attr = f.env.as_ref().map(|e| quote! { env = #e, });
                quote! {
                    #[arg(long = #long_name, global = true, #env_attr)]
                    #serde_skip
                    pub #ident: Vec<#inner>,
                }
            } else if f.is_bool {
                let serde_skip = if f.cli_only {
                    quote! { #[serde(skip)] }
                } else {
                    quote! { #[serde(default)] }
                };
                let env_attr = f.env.as_ref().map(|e| quote! { env = #e, });
                let hide_env = f.env.as_ref().map(|_| quote! { hide_env = true, });
                quote! {
                    #[arg(long = #long_name, global = true, action = clap::ArgAction::SetTrue, #env_attr #hide_env)]
                    #serde_skip
                    pub #ident: bool,
                }
            } else {
                let ty = if f.is_option {
                    let inner = extract_inner_type(&f.ty).unwrap();
                    quote! { Option<#inner> }
                } else {
                    let ty = &f.ty;
                    quote! { Option<#ty> }
                };

                let serde_skip = if f.cli_only {
                    quote! { #[serde(skip)] }
                } else {
                    quote! { #[serde(default)] }
                };

                let env_attr = f.env.as_ref().map(|e| quote! { env = #e, });
                let hide_env = f.env.as_ref().map(|_| quote! { hide_env = true, });

                quote! {
                    #[arg(long = #long_name, global = true, #env_attr #hide_env)]
                    #serde_skip
                    pub #ident: #ty,
                }
            }
        })
        .collect()
}

fn gen_merge_fields(fields: &[FieldInfo]) -> Vec<TokenStream> {
    fields
        .iter()
        .filter(|f| !f.skip)
        .map(|f| {
            let ident = &f.ident;
            if f.flatten {
                quote! { #ident: self.#ident.merge(fallback.#ident), }
            } else if f.is_vec {
                quote! {
                    #ident: if self.#ident.is_empty() {
                        fallback.#ident
                    } else {
                        self.#ident
                    },
                }
            } else if f.is_bool {
                quote! { #ident: self.#ident || fallback.#ident, }
            } else {
                quote! { #ident: self.#ident.or(fallback.#ident), }
            }
        })
        .collect()
}

fn gen_resolve_fields(fields: &[FieldInfo], struct_name: &syn::Ident) -> TokenStream {
    let skipped_name = format_ident!("{}Skipped", struct_name);
    let has_skip_fields = fields.iter().any(|f| f.skip);

    // Phase 1: check required fields, collect missing
    let check_stmts: Vec<TokenStream> = fields
        .iter()
        .filter(|f| !f.skip && !f.flatten && !f.is_vec && !f.is_option && !f.is_bool && f.default_expr.is_none())
        .map(|f| {
            let field_name = f.ident.to_string();
            let ident = &f.ident;
            quote! {
                if self.#ident.is_none() {
                    missing.push(#field_name);
                }
            }
        })
        .collect();

    // Phase 2: build the struct (only reached if no missing fields)
    let resolve_assignments: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;

            if f.skip {
                return quote! { #ident: skipped.#ident, };
            }

            if f.flatten {
                let inner_has_skip = false; // sub-structs don't have skip fields in our usage
                if inner_has_skip {
                    return quote! { #ident: self.#ident.resolve(/* skipped */)?, };
                }
                return quote! { #ident: self.#ident.resolve_no_skip()?, };
            }

            if f.is_bool {
                return quote! { #ident: self.#ident, };
            }

            if f.is_vec {
                return quote! { #ident: self.#ident, };
            }

            if f.is_option {
                return quote! { #ident: self.#ident, };
            }

            match &f.default_expr {
                Some(default) => quote! {
                    #ident: self.#ident.unwrap_or_else(|| #default),
                },
                None => quote! {
                    #ident: self.#ident.unwrap(),
                },
            }
        })
        .collect();

    let resolve_with_skip = if has_skip_fields {
        quote! {
            pub fn resolve(self, skipped: #skipped_name) -> Result<#struct_name, String> {
                let mut missing: Vec<&str> = Vec::new();
                #(#check_stmts)*
                if !missing.is_empty() {
                    return Err(format!(
                        "missing required settings:\n  - {}",
                        missing.join("\n  - ")
                    ));
                }
                Ok(#struct_name {
                    #(#resolve_assignments)*
                })
            }
        }
    } else {
        // No skip fields — generate resolve_no_skip that takes no args
        quote! {
            pub fn resolve_no_skip(self) -> Result<#struct_name, String> {
                let mut missing: Vec<&str> = Vec::new();
                #(#check_stmts)*
                if !missing.is_empty() {
                    return Err(format!(
                        "missing required settings:\n  - {}",
                        missing.join("\n  - ")
                    ));
                }
                // skipped is unused but needed for struct construction
                let skipped = #skipped_name {};
                Ok(#struct_name {
                    #(#resolve_assignments)*
                })
            }
        }
    };

    resolve_with_skip
}

fn gen_skipped_struct(
    fields: &[FieldInfo],
    struct_name: &syn::Ident,
    vis: &syn::Visibility,
) -> TokenStream {
    let skipped_name = format_ident!("{}Skipped", struct_name);

    let skipped_fields: Vec<TokenStream> = fields
        .iter()
        .filter(|f| f.skip)
        .map(|f| {
            let ident = &f.ident;
            let ty = &f.ty;
            quote! { pub #ident: #ty, }
        })
        .collect();

    quote! {
        #vis struct #skipped_name {
            #(#skipped_fields)*
        }
    }
}
