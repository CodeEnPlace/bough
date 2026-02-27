use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse::Parse, parse_macro_input, Expr, LitStr, Token};

struct CmdArgs {
    dir: Expr,
    cmd: LitStr,
    patterns: Vec<LitStr>,
}

impl Parse for CmdArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let dir = input.parse()?;
        input.parse::<Token![,]>()?;
        let cmd = input.parse()?;
        let mut patterns = Vec::new();
        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if !input.is_empty() {
                patterns.push(input.parse()?);
            }
        }
        Ok(Self { dir, cmd, patterns })
    }
}

enum Segment {
    Literal(String),
    Capture(String),
    BackRef(String),
}

fn parse_pattern(pattern: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut literal = String::new();
    let bytes = pattern.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if i + 2 < bytes.len()
            && bytes[i] == b'{'
            && (bytes[i + 1] == b'!' || bytes[i + 1] == b'?')
        {
            if !literal.is_empty() {
                segments.push(Segment::Literal(std::mem::take(&mut literal)));
            }
            let mode = bytes[i + 1];
            let rest = &pattern[i + 2..];
            let close = rest.find('}').expect("unclosed capture in pattern");
            let name = pattern[i + 2..i + 2 + close].to_string();
            if mode == b'!' {
                segments.push(Segment::Capture(name));
            } else {
                segments.push(Segment::BackRef(name));
            }
            i += 2 + close + 1;
        } else {
            literal.push(bytes[i] as char);
            i += 1;
        }
    }

    if !literal.is_empty() {
        segments.push(Segment::Literal(literal));
    }

    segments
}

fn build_cmd_expr(cmd_lit: &LitStr) -> proc_macro2::TokenStream {
    let cmd_str = cmd_lit.value();
    if !cmd_str.contains("{?") {
        return quote! { #cmd_lit };
    }

    let parts = parse_pattern(&cmd_str);
    let pieces: Vec<proc_macro2::TokenStream> = parts
        .iter()
        .map(|seg| match seg {
            Segment::Literal(s) => quote! { #s },
            Segment::BackRef(name) => {
                let ident = format_ident!("{}", name);
                quote! { #ident.as_str() }
            }
            Segment::Capture(_) => panic!("{{!capture}} not allowed in command string"),
        })
        .collect();

    quote! { &[#(#pieces),*].concat() }
}

/// Collect needles (literal/backref parts) and capture names from segments.
/// Needles alternate with captures: needle[0], capture[0], needle[1], ...
/// So needles.len() == captures.len() + 1.
fn collect_needles_and_captures(
    segments: &[Segment],
) -> (Vec<Vec<proc_macro2::TokenStream>>, Vec<String>) {
    let mut needles: Vec<Vec<proc_macro2::TokenStream>> = vec![vec![]];
    let mut captures = Vec::new();

    for seg in segments {
        match seg {
            Segment::Literal(s) => {
                needles.last_mut().unwrap().push(quote! { #s });
            }
            Segment::BackRef(name) => {
                let ident = format_ident!("{}", name);
                needles
                    .last_mut()
                    .unwrap()
                    .push(quote! { #ident.as_str() });
            }
            Segment::Capture(name) => {
                captures.push(name.clone());
                needles.push(vec![]);
            }
        }
    }

    (needles, captures)
}

fn build_pattern_stmts(patterns: &[LitStr]) -> Vec<proc_macro2::TokenStream> {
    let mut stmts = Vec::new();

    for (pi, pat_lit) in patterns.iter().enumerate() {
        let pat_str = pat_lit.value();
        let pat_trimmed = pat_str.trim();

        if pat_trimmed.is_empty() {
            continue;
        }

        let segments = parse_pattern(pat_trimmed);
        let (needles, captures) = collect_needles_and_captures(&segments);

        let needle_exprs: Vec<proc_macro2::TokenStream> = needles
            .iter()
            .map(|parts| {
                if parts.is_empty() {
                    quote! { String::new() }
                } else if parts.len() == 1 {
                    let p = &parts[0];
                    quote! { #p.to_string() }
                } else {
                    quote! { [#(#parts),*].concat() }
                }
            })
            .collect();

        let n_needles = needle_exprs.len();
        let needles_ident = format_ident!("__needles_{}", pi);
        let refs_ident = format_ident!("__refs_{}", pi);
        let result_ident = format_ident!("__result_{}", pi);

        stmts.push(quote! {
            let #needles_ident: [String; #n_needles] = [#(#needle_exprs),*];
            let #refs_ident: Vec<&str> = #needles_ident.iter().map(|s| s.as_str()).collect();
            let #result_ident = ::bough_cli_test::find_unmatched_line(&__lines, &__matched, &#refs_ident)
                .unwrap_or_else(|| panic!(
                    "no output line matched pattern: {:?}\nunmatched output:\n{}",
                    #pat_trimmed,
                    __lines.iter().enumerate()
                        .filter(|(i, _)| !__matched[*i])
                        .map(|(_, l)| *l)
                        .collect::<Vec<_>>()
                        .join("\n"),
                ));
            __matched[#result_ident.0] = true;
        });

        for (ci, name) in captures.iter().enumerate() {
            let ident = format_ident!("{}", name);
            let ri = result_ident.clone();
            stmts.push(quote! {
                let #ident = #ri.1[#ci].clone();
            });
        }
    }

    stmts
}

fn build_cmd(args: CmdArgs, success: bool) -> TokenStream {
    let dir = &args.dir;
    let cmd_lit = &args.cmd;
    let cmd_expr = build_cmd_expr(cmd_lit);
    let pattern_stmts = build_pattern_stmts(&args.patterns);

    let (assert_check, output_field) = if success {
        (
            quote! {
                assert!(
                    __output.status.success(),
                    "expected success for: {}\nstderr: {}",
                    #cmd_lit,
                    String::from_utf8_lossy(&__output.stderr),
                );
            },
            quote! { __output.stdout },
        )
    } else {
        (
            quote! {
                assert!(
                    !__output.status.success(),
                    "expected failure for: {}\nstdout: {}",
                    #cmd_lit,
                    String::from_utf8_lossy(&__output.stdout),
                );
            },
            quote! { __output.stderr },
        )
    };

    quote! {
        let __output = ::bough_cli_test::exec_cmd(#dir.as_ref(), #cmd_expr);
        #assert_check
        let __text = String::from_utf8(#output_field).unwrap();
        let __lines: Vec<&str> = __text.lines().collect();
        let mut __matched: Vec<bool> = vec![false; __lines.len()];
        #(#pattern_stmts)*
    }
    .into()
}

#[proc_macro]
pub fn cmd(input: TokenStream) -> TokenStream {
    build_cmd(parse_macro_input!(input as CmdArgs), true)
}

#[proc_macro]
pub fn cmd_err(input: TokenStream) -> TokenStream {
    build_cmd(parse_macro_input!(input as CmdArgs), false)
}

struct AssertFileArgs {
    dir: Expr,
    path: LitStr,
    patterns: Vec<LitStr>,
}

impl Parse for AssertFileArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let dir = input.parse()?;
        input.parse::<Token![,]>()?;
        let path = input.parse()?;
        let mut patterns = Vec::new();
        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if !input.is_empty() {
                patterns.push(input.parse()?);
            }
        }
        Ok(Self { dir, path, patterns })
    }
}

#[proc_macro]
pub fn assert_file(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as AssertFileArgs);
    let dir = &args.dir;
    let path_lit = &args.path;
    let path_expr = build_cmd_expr(path_lit);
    let pattern_stmts = build_pattern_stmts(&args.patterns);

    quote! {
        let __file_path = ::std::path::Path::new(#dir.as_ref()).join(#path_expr);
        let __text = ::std::fs::read_to_string(&__file_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", __file_path.display()));
        let __lines: Vec<&str> = __text.lines().collect();
        let mut __matched: Vec<bool> = vec![false; __lines.len()];
        #(#pattern_stmts)*
    }
    .into()
}
