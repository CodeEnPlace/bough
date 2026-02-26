use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse::Parse, parse_macro_input, Expr, LitStr, Token};

struct CmdArgs {
    dir: Expr,
    cmd: LitStr,
    pattern: LitStr,
    stderr: Option<LitStr>,
}

impl Parse for CmdArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let dir = input.parse()?;
        input.parse::<Token![,]>()?;
        let cmd = input.parse()?;
        input.parse::<Token![,]>()?;
        let pattern = input.parse()?;
        let stderr = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Self { dir, cmd, pattern, stderr })
    }
}

fn extract_captures(pattern: &str) -> Vec<String> {
    let mut names = Vec::new();
    let bytes = pattern.as_bytes();
    let mut i = 0;
    while i + 2 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'!' {
            let rest = &pattern[i + 2..];
            if let Some(close) = rest.find('}') {
                names.push(pattern[i + 2..i + 2 + close].to_string());
                i += 2 + close + 1;
                continue;
            }
        }
        i += 1;
    }
    names
}

#[proc_macro]
pub fn cmd(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as CmdArgs);
    let dir = &args.dir;
    let cmd_str = &args.cmd;
    let pattern = &args.pattern;

    let captures = extract_captures(&pattern.value());
    let bindings = captures.iter().map(|name| {
        let ident = format_ident!("{}", name);
        let name_str = name.as_str();
        quote! { let #ident = #dir.take(#name_str); }
    });

    let call = if let Some(stderr) = &args.stderr {
        quote! { #dir.run_failure(#cmd_str, #stderr); }
    } else {
        quote! { #dir.run_success(#cmd_str, #pattern); }
    };

    let expanded = quote! {
        #call
        #(#bindings)*
    };

    expanded.into()
}
