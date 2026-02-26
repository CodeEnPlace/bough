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

/// For a single pattern line's segments, collect needles (literal/backref parts)
/// and capture names. Needles alternate with captures:
///   needle[0], capture[0], needle[1], capture[1], ..., needle[N]
/// So needles.len() == captures.len() + 1. Empty needles mean the pattern
/// starts/ends with a capture or has adjacent captures.
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

fn build_match_code(
    output: &proc_macro2::TokenStream,
    pattern_str: &str,
) -> proc_macro2::TokenStream {
    let pattern_lines: Vec<&str> = pattern_str
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if pattern_lines.is_empty() {
        return quote! {};
    }

    let mut stmts = Vec::new();

    for (li, pat_line) in pattern_lines.iter().enumerate() {
        let segments = parse_pattern(pat_line);
        let (needles, captures) = collect_needles_and_captures(&segments);

        let needle_exprs: Vec<proc_macro2::TokenStream> = needles.iter().map(|parts| {
            if parts.is_empty() {
                quote! { String::new() }
            } else if parts.len() == 1 {
                let p = &parts[0];
                quote! { #p.to_string() }
            } else {
                quote! { [#(#parts),*].concat() }
            }
        }).collect();

        let n_needles = needle_exprs.len();
        let needles_ident = format_ident!("__needles_{}", li);
        let refs_ident = format_ident!("__refs_{}", li);
        let result_ident = format_ident!("__result_{}", li);
        let pat_line_str = *pat_line;

        stmts.push(quote! {
            let #needles_ident: [String; #n_needles] = [#(#needle_exprs),*];
            let #refs_ident: Vec<&str> = #needles_ident.iter().map(|s| s.as_str()).collect();
            let #result_ident = ::bough_cli_test::find_line(&__lines[__cursor..], &#refs_ident)
                .unwrap_or_else(|| panic!(
                    "no output line matched pattern: {:?}\nremaining output:\n{}",
                    #pat_line_str,
                    __lines[__cursor..].join("\n"),
                ));
            __cursor += #result_ident.0 + 1;
        });

        for (ci, name) in captures.iter().enumerate() {
            let ident = format_ident!("{}", name);
            let ri = result_ident.clone();
            stmts.push(quote! {
                let #ident = #ri.1[#ci].clone();
            });
        }
    }

    quote! {
        let __lines: Vec<&str> = #output.lines().collect();
        let mut __cursor: usize = 0;
        #(#stmts)*
    }
}

#[proc_macro]
pub fn cmd(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as CmdArgs);
    let dir = &args.dir;
    let cmd_lit = &args.cmd;
    let pattern = &args.pattern;

    let pattern_str = pattern.value();
    let cmd_expr = build_cmd_expr(cmd_lit);

    if let Some(stderr_pat) = &args.stderr {
        let stderr_str = stderr_pat.value();
        let match_code = build_match_code(&quote! { __stderr }, &stderr_str);
        quote! {
            let __stderr = #dir.run_failure(#cmd_expr);
            #match_code
        }
        .into()
    } else {
        let match_code = build_match_code(&quote! { __stdout }, &pattern_str);
        quote! {
            let __stdout = #dir.run_success(#cmd_expr);
            #match_code
        }
        .into()
    }
}
