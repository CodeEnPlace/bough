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
        if i + 2 < bytes.len() && bytes[i] == b'{' && (bytes[i + 1] == b'!' || bytes[i + 1] == b'?') {
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
    let pieces: Vec<proc_macro2::TokenStream> = parts.iter().map(|seg| match seg {
        Segment::Literal(s) => quote! { #s },
        Segment::BackRef(name) => {
            let ident = format_ident!("{}", name);
            quote! { #ident.as_str() }
        }
        Segment::Capture(_) => panic!("{{!capture}} not allowed in command string"),
    }).collect();

    quote! { &[#(#pieces),*].concat() }
}

fn build_match_code(
    output: &proc_macro2::TokenStream,
    segments: &[Segment],
    pattern_str: &str,
) -> proc_macro2::TokenStream {
    if segments.is_empty() {
        return quote! {};
    }

    let has_captures = segments.iter().any(|s| matches!(s, Segment::Capture(_)));

    if !has_captures {
        let pieces: Vec<proc_macro2::TokenStream> = segments.iter().map(|seg| match seg {
            Segment::Literal(s) => quote! { #s },
            Segment::BackRef(name) => {
                let ident = format_ident!("{}", name);
                quote! { #ident.as_str() }
            }
            Segment::Capture(_) => unreachable!(),
        }).collect();

        return quote! {
            let __expected: String = [#(#pieces),*].concat();
            let __expected = __expected.trim();
            let __output = #output.trim();
            assert!(
                __output.contains(__expected),
                "expected output to contain:\n  {}\ngot:\n  {}",
                __expected,
                __output,
            );
        };
    }

    // With captures: collect the fixed "needle" segments between captures.
    // Walk through the output string finding needles in order, extracting
    // the text between consecutive needles as captured values.

    // Flatten segments into: a sequence of needles (built from consecutive
    // literal/backref segments merged together) separated by capture slots.
    // Each needle is a token stream that evaluates to a String.
    // We track which captures appear between which needles.

    let mut needles: Vec<Vec<proc_macro2::TokenStream>> = vec![vec![]];
    let mut capture_names: Vec<String> = Vec::new();

    for seg in segments {
        match seg {
            Segment::Literal(s) => {
                needles.last_mut().unwrap().push(quote! { #s });
            }
            Segment::BackRef(name) => {
                let ident = format_ident!("{}", name);
                needles.last_mut().unwrap().push(quote! { #ident.as_str() });
            }
            Segment::Capture(name) => {
                capture_names.push(name.clone());
                needles.push(vec![]);
            }
        }
    }

    // needles[0] is before first capture, needles[1] after first capture, etc.
    // So capture[i] is between needles[i] and needles[i+1].
    // Some needles may be empty (capture at start/end or adjacent captures).

    let needle_exprs: Vec<proc_macro2::TokenStream> = needles.iter().map(|parts| {
        if parts.is_empty() {
            quote! { String::new() }
        } else {
            quote! { [#(#parts),*].concat() }
        }
    }).collect();

    let n_needles = needle_exprs.len();
    let n_captures = capture_names.len();

    let bindings: Vec<proc_macro2::TokenStream> = capture_names.iter().enumerate().map(|(i, name)| {
        let ident = format_ident!("{}", name);
        quote! { let #ident = __captures[#i].clone(); }
    }).collect();

    quote! {
        let __output = #output.trim().to_string();
        let __needles: [String; #n_needles] = [#(#needle_exprs),*];
        let mut __captures: Vec<String> = vec![String::new(); #n_captures];
        let mut __cursor: usize = 0;

        // Find the first needle (before any capture)
        if !__needles[0].is_empty() {
            let __pos = __output.find(__needles[0].as_str()).unwrap_or_else(|| {
                panic!(
                    "pattern did not match\npattern: {}\ncould not find {:?}\noutput:  {}",
                    #pattern_str, __needles[0], __output,
                );
            });
            __cursor = __pos + __needles[0].len();
        }

        // For each capture, find the needle that follows it
        for __ci in 0..#n_captures {
            let __next_needle = &__needles[__ci + 1];
            if __next_needle.is_empty() && __ci + 1 == #n_needles - 1 {
                // Trailing capture with no following needle — take rest
                __captures[__ci] = __output[__cursor..].trim().to_string();
            } else if __next_needle.is_empty() {
                // Empty needle between captures — take one whitespace-delimited token
                let __rest = &__output[__cursor..].trim_start();
                let __end = __rest.find(char::is_whitespace).unwrap_or(__rest.len());
                __captures[__ci] = __rest[..__end].to_string();
                __cursor = __output.len() - __rest.len() + __end;
            } else {
                let __trimmed = __next_needle.trim_start();
                let __pos = __output[__cursor..].find(__trimmed).unwrap_or_else(|| {
                    panic!(
                        "pattern did not match\npattern: {}\ncould not find {:?} after position {}\noutput:  {}",
                        #pattern_str, __next_needle, __cursor, __output,
                    );
                });
                __captures[__ci] = __output[__cursor..__cursor + __pos].trim().to_string();
                __cursor += __pos + __trimmed.len();
            }
        }

        #(#bindings)*
    }
}

#[proc_macro]
pub fn cmd(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as CmdArgs);
    let dir = &args.dir;
    let cmd_lit = &args.cmd;
    let pattern = &args.pattern;

    let pattern_segments = parse_pattern(&pattern.value());
    let cmd_expr = build_cmd_expr(cmd_lit);
    let pattern_str = pattern.value();

    if let Some(stderr_pat) = &args.stderr {
        let stderr_segments = parse_pattern(&stderr_pat.value());
        let stderr_str = stderr_pat.value();
        let match_code = build_match_code(&quote! { __stderr }, &stderr_segments, &stderr_str);
        quote! {
            let __stderr = #dir.run_failure(#cmd_expr);
            #match_code
        }
        .into()
    } else {
        let match_code = build_match_code(&quote! { __stdout }, &pattern_segments, &pattern_str);
        quote! {
            let __stdout = #dir.run_success(#cmd_expr);
            #match_code
        }
        .into()
    }
}
