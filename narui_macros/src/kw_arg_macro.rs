use core::result::Result::Ok;
use proc_macro2::Ident;
use quote::{quote, quote_spanned, ToTokens};
use std::collections::HashMap;
use syn::{
    braced,
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token,
    Expr,
    ExprAssign,
    LitStr,
    Path,
    Token,
};

#[derive(Debug, Clone)]
enum MaybeExpr {
    Some(Expr),
    None(Token![@]),
}
impl Parse for MaybeExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(if input.peek(Token![@]) {
            MaybeExpr::None(input.parse()?)
        } else {
            MaybeExpr::Some(input.parse()?)
        })
    }
}

#[derive(Debug, Clone)]
struct ExprAssignMaybe {
    left: Ident,
    eq: Token![=],
    right: MaybeExpr,
}
impl Parse for ExprAssignMaybe {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(ExprAssignMaybe { left: input.parse()?, eq: input.parse()?, right: input.parse()? })
    }
}

#[derive(Debug, Clone)]
struct KwArgCallParse {
    span: Ident,
    name: LitStr,
    function: Path,
    bang: Option<Token![!]>,
    brace_1: token::Brace,
    function_args: Punctuated<ExprAssignMaybe, Token![,]>,
    brace_2: token::Paren,
    given_args: Punctuated<ExprAssign, Token![,]>,
}
impl Parse for KwArgCallParse {
    fn parse(input: ParseStream<'_>) -> syn::parse::Result<Self> {
        let expected_args_stream;
        let given_args_stream;
        Ok(KwArgCallParse {
            span: input.parse()?,
            name: input.parse()?,
            function: input.parse()?,
            bang: input.parse().unwrap_or(None),
            brace_1: braced!(expected_args_stream in input),
            function_args: expected_args_stream.parse_terminated(ExprAssignMaybe::parse)?,
            brace_2: parenthesized!(given_args_stream in input),
            given_args: given_args_stream.parse_terminated(ExprAssign::parse)?,
        })
    }
}

/// example: kw_arg_call!(span lol!{a=1, b=@}(b=2))
pub fn kw_arg_call(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input_clone = input;
    let KwArgCallParse {
        span,
        name,
        function,
        bang,
        brace_1: _,
        function_args,
        brace_2: _,
        given_args,
    } = parse_macro_input!(input_clone);
    let name = name.value();

    let mut given_args_map: HashMap<_, _> = given_args
        .iter()
        .map(|x| (x.left.to_token_stream().to_string(), x.right.clone()))
        .collect();

    let args: Vec<_> = function_args
        .iter()
        .map(|x| {
            let arg_name = x.left.to_token_stream().to_string();
            if let Some(expr) = given_args_map.remove(&arg_name) {
                quote! { #expr }
            } else if let MaybeExpr::Some(default) = &x.right {
                quote! { #default }
            } else {
                let span = span.span();
                let error =
                    format!("argument '{}' of '{}' is non-optional but missing", arg_name, name);
                quote_spanned! {span=>
                    compile_error!(#error)
                }
            }
        })
        .collect();

    let unknown_argument_errors = given_args_map.iter().map(|(x, expr)| {
        let mut args_string = String::new();
        for (i, name) in function_args.iter().enumerate() {
            if i == 0 {
                args_string += &format!("'{}'", name.left);
            } else if i < function_args.len() - 1 {
                args_string += &format!(", '{}'", name.left);
            } else {
                args_string += &format!(" and '{}'", name.left);
            }
        }
        let error = format!(
            "found unexpected argument {}. '{}' only accepts {} as arguments.",
            x, name, args_string
        );

        let span = expr.span();
        quote_spanned! {span=>
            compile_error!(#error);
        }
    });

    let function = if bang.is_some() {
        quote! { #function! }
    } else {
        quote! { #function }
    };

    let transformed = quote! {{
        #(#unknown_argument_errors)*
        #function(#(#args,)*)
    }};
    // println!("{}", transformed);
    transformed.into()
}
