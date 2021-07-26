use bind_match::bind_match;
use core::result::{Result, Result::Ok};
use proc_macro2::{Ident, Literal, Span};
use quote::{quote, ToTokens};
use std::collections::{HashMap, HashSet};
use syn::{
    parse::{Parse, ParseStream, Parser},
    punctuated::Punctuated,
    Expr,
    FnArg,
    ItemFn,
    Pat,
    Token,
    Type,
};

// a helper to parse the parameters to the widget proc macro attribute
// we cant use the syn AttributeArgs here because it can only handle literals
// and we want expressions (e.g. for closures)
#[derive(Debug, Clone)]
struct AttributeParameter {
    ident: Ident,
    expr: Expr,
}
impl Parse for AttributeParameter {
    fn parse(input: ParseStream<'_>) -> syn::parse::Result<Self> {
        let ident = input.parse::<Ident>()?;
        input.parse::<Token![=]>()?;
        let expr = input.parse::<Expr>()?;

        Ok(AttributeParameter { ident, expr })
    }
}

// allows for kwarg-style calling of functions
pub fn widget(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // parse the function
    let parsed: Result<ItemFn, _> = syn::parse2(item.into());
    let function = parsed.unwrap();
    let function_ident = function.sig.ident.clone();

    let last_name = get_arg_names(&function).into_iter().last().unwrap().to_string();
    let last_type = get_arg_types(&function)[&last_name].clone();
    assert_eq!(last_type.to_token_stream().to_string(), "Context");
    assert_eq!(last_name.to_string(), "context");

    let return_type = function.sig.output.clone().to_token_stream().to_string().replace("-> ", "");
    assert_eq!(return_type, "Fragment");

    // parse & format the default arguments
    let parser = Punctuated::<AttributeParameter, Token![,]>::parse_terminated;
    let parsed_args = parser.parse(args).unwrap();

    let macro_ident =
        Ident::new(&format!("__{}_constructor_", function_ident.clone()), Span::call_site());
    let macro_ident_pub =
        Ident::new(&format!("__{}_constructor", function_ident.clone()), Span::call_site());

    let match_arms: Vec<_> = {
        let args_with_default: HashSet<_> =
            parsed_args.clone().into_iter().map(|x| x.ident.to_string()).collect();

        let arg_types = get_arg_types(&function);
        get_arg_names(&function)
            .iter()
            .filter(|ident| &ident.to_string() != "context")
            .map(|unhygienic| {
                let arg_names: Vec<_> = get_arg_names(&function).into_iter().filter(|ident| &ident.to_string() != "context").collect();

                let arg_type = &arg_types[&unhygienic.to_string()];
                let dummy_function_ident =
                    Ident::new(&format!("_constrain_arg_type_{}", unhygienic), Span::call_site());
                let dummy_function = quote! {
                    // this is needed to be able to use the default argument with the correct type &
                    // mute unusesd warnings
                    #[allow(non_snake_case, unused)]
                    fn #dummy_function_ident(_arg: #arg_type) {  }
                };
                let value = if args_with_default.contains(&unhygienic.to_string()) {
                    quote! {{
                        #dummy_function
                        #dummy_function_ident($#unhygienic);
                        $value
                    }}
                } else {
                    quote! {$value}
                };

                quote! {
                    (@parse_args [#($#arg_names:ident,)*] #unhygienic = $value:expr,$($rest:tt)*) => {
                        #[allow(unused_braces)]
                        let $#unhygienic = #value;
                        #macro_ident_pub!(@parse_args [#($#arg_names,)*] $($rest)*);
                    };
                }
            })
            .collect()
    };

    let shout_args_arm = {
        let initializers = parsed_args.clone().into_iter().map(|x| {
            let ident = desinfect_ident(&x.ident);
            let value = x.expr;
            quote! { let #ident = #value }
        });
        let arg_names: Vec<_> = get_arg_names(&function)
            .into_iter()
            .filter(|ident| &ident.to_string() != "context")
            .map(|ident| desinfect_ident(&ident))
            .collect();

        let arg_names_listenables: Vec<_> = get_arg_names(&function)
            .into_iter()
            .filter(|ident| get_arg_types(&function)[&ident.to_string()].to_token_stream().to_string() != "Context")
            .map(|ident| desinfect_ident(&ident))
            .enumerate()
            .map(|(i, ident)| {
                let i = i as u64;
                quote! {{
                    let listenable = unsafe { Listenable::uninitialized($context.widget_local.key.with(KeyPart::Arg(#i))) };
                    shout!($context, listenable, #ident);
                    listenable
                }}
            })
            .collect();


        quote! {
            (@shout_args context=$context:ident, $($args:tt)*) => {
                {
                    #(#initializers;)*
                    #macro_ident_pub!(@parse_args [#(#arg_names,)*] $($args)*);

                    (#(#arg_names_listenables,)*)
                }
            };
        }
    };

    let constructor_macro = {
        let arg_names: Vec<_> = get_arg_names(&function)
            .into_iter()
            .filter(|ident| &ident.to_string() != "context")
            .map(|ident| desinfect_ident(&ident))
            .collect();
        let arg_numbers: Vec<_> = (0..(get_arg_names(&function).len() - 1))
            .map(|i| Literal::usize_unsuffixed(i))
            .collect();

        quote! {
            #[macro_export]
            macro_rules! #macro_ident {
                #shout_args_arm

                #(#match_arms)*
                (@parse_args [#($#arg_names:ident,)*] ) => { };

                (@construct listenable=$listenable:ident, context=$context:ident) => {{
                    #function_ident(#($context.listen($listenable.#arg_numbers),)* $context)
                }}
            }

            // we do this to have correct scoping of the macro. It should not just be placed at the
            // crate root but rather at the path of the original function.
            pub use #macro_ident as #macro_ident_pub;
        }
    };

    let transformed_function = transform_function_args_to_context(function);

    let transformed = quote! {
        #constructor_macro

        #transformed_function
    };
    //println!("widget: \n{}\n\n", transformed.clone());
    transformed.into()
}
// a (simplified) example of the kind of macro this proc macro generates:
/*
macro_rules! button_constructor {
    (@initial $($args:tt)*) => {
        {
            let size = 12.0;
            button_constructor!(@parse [size, text] $($args)*);

            button(text, size)
        }
    };
    (@parse [$size:ident, $text:ident] size = $value:expr,$($rest:tt)*) => {
        let $size = $value;
        button_constructor!(@parse [$size, $text] $($rest)*);
    };
    (@parse [$size:ident, $text:ident] text = $value:expr,$($rest:tt)*) => {
        let $text = $value;
        button_constructor!(@parse [$size, $text] $($rest)*);
    };
    (@parse [$size:ident, $text:ident] ) => { };
}
*/

// adds the function arguments to the context as a `Listenable` and listen on it
// for partial re-evaluation.
fn transform_function_args_to_context(function: ItemFn) -> proc_macro2::TokenStream {
    let function_clone = function.clone();
    let ItemFn { attrs, vis, sig, block } = function;
    let stmts = &block.stmts;
    let context_string = get_arg_types(&function_clone)
        .iter()
        .filter(|(_, ty)| ty.to_token_stream().to_string().replace("-> ", "") == "Context")
        .next()
        .unwrap()
        .0
        .to_string();
    let context_ident = Ident::new(&context_string, Span::call_site());
    let function_transformed = quote! {
        #(#attrs)* #vis #sig {
            let to_return = {
                #(#stmts)*
            };

            std::mem::drop(#context_ident);  // we consume the context here to prevent the other widgets from giving it out
            to_return
        }
    };
    function_transformed
}

fn get_arg_names(function: &ItemFn) -> Vec<Ident> {
    function
        .clone()
        .sig
        .inputs
        .into_iter()
        .map(|arg| {
            let pat_type = bind_match!(arg, FnArg::Typed(x) => x).unwrap();
            let pat_ident = bind_match!(*pat_type.pat, Pat::Ident(x) => x).unwrap();
            pat_ident.ident
        })
        .collect()
}

// creates a hyginic (kind of) identifier from an unhyginic one
fn desinfect_ident(ident: &Ident) -> Ident {
    Ident::new(&format!("__{}", ident), Span::call_site())
}

fn get_arg_types(function: &ItemFn) -> HashMap<String, Box<Type>> {
    function
        .clone()
        .sig
        .inputs
        .into_iter()
        .map(|arg| {
            let pat_type = bind_match!(arg, FnArg::Typed(x) => x).unwrap();
            let pat_ident = bind_match!(*pat_type.pat, Pat::Ident(x) => x).unwrap();

            (pat_ident.ident.to_string(), pat_type.ty)
        })
        .collect()
}
