use bind_match::bind_match;
use core::result::{Result, Result::Ok};
use proc_macro2::{Ident, LineColumn, Literal, Span};
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

const WIDGET_CONTEXT_TYPE_STRING: &str = "& mut WidgetContext";

// allows for kwarg-style calling of functions
pub fn widget(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // parse the function
    let parsed: Result<ItemFn, _> = syn::parse2(item.into());
    let function = parsed.unwrap();
    let function_ident = function.sig.ident.clone();
    let mod_ident = Ident::new(&format!("{}", function_ident.clone()), Span::call_site());

    let last_name = get_arg_names(&function).into_iter().last().unwrap().to_string();
    let last_type = get_arg_types(&function)[&last_name].clone();
    assert_eq!(last_type.to_token_stream().to_string(), WIDGET_CONTEXT_TYPE_STRING);
    assert_eq!(last_name.to_string(), "context");

    let return_type = function.sig.output.clone().to_token_stream().to_string().replace("-> ", "");

    // parse & format the default arguments
    let parser = Punctuated::<AttributeParameter, Token![,]>::parse_terminated;
    let parsed_args = parser.parse(args).unwrap();

    let macro_ident =
        Ident::new(&format!("__{}_constructor_", function_ident.clone()), Span::call_site());
    let macro_ident_pub =
        Ident::new(&format!("__{}_constructor", function_ident.clone()), Span::call_site());

    let arg_types = get_arg_types(&function);
    let match_arms: Vec<_> = {
        let args_with_default: HashSet<_> =
            parsed_args.clone().into_iter().map(|x| x.ident.to_string()).collect();

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
                    fn #dummy_function_ident(arg: #arg_type) -> #arg_type { arg }
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
                        #dummy_function
                        let $#unhygienic = #dummy_function_ident(#value);
                        #mod_ident::#macro_ident_pub!(@parse_args [#($#arg_names,)*] $($rest)*);
                    };
                }
            })
            .collect()
    };

    let shout_args_arm = {
        let initializers = parsed_args.clone().into_iter().map(|x| {
            let unhygienic = &x.ident;
            let ident = desinfect_ident(unhygienic);
            let value = x.expr;
            let arg_type = &arg_types[&unhygienic.to_string()];
            let dummy_function_ident =
                Ident::new(&format!("_constrain_arg_type_{}", unhygienic), Span::call_site());
            let dummy_function = quote! {
                // this is needed to be able to use the default argument with the correct type &
                // mute unusesd warnings
                #[allow(non_snake_case, unused)]
                fn #dummy_function_ident(arg: #arg_type) -> #arg_type { arg }
            };

            quote! {
                let #ident = {
                    #dummy_function
                    #dummy_function_ident(#value)
                }
            }
        });
        let arg_names: Vec<_> = get_arg_names(&function)
            .into_iter()
            .filter(|ident| &ident.to_string() != "context")
            .map(|ident| desinfect_ident(&ident))
            .collect();

        quote! {
            (@shout_args context=$context:expr, $($args:tt)*) => {
                {
                    #(#initializers;)*
                    #mod_ident::#macro_ident_pub!(@parse_args [#(#arg_names,)*] $($args)*);

                    shout_args!($context, $context.widget_local.key, [#(#arg_names,)*])
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
        let arg_numbers_plus_one: Vec<_> = (0..(get_arg_names(&function).len() - 1))
            .map(|i| Literal::usize_unsuffixed(i + 1))
            .collect();

        let transformer = if return_type == "FragmentInner" {
            quote! {
                fn transformer(input: FragmentInner) -> FragmentInner {
                    input
                }
            }
        } else if return_type == "Fragment" {
            quote! {
                fn transformer(input: Fragment) -> FragmentInner {
                    FragmentInner::Node {
                        children: vec![ input ],
                        layout: Box::new(rutter_layout::Transparent),
                    }
                }
            }
        } else {
            panic!("widgets need to either return Fragment or FragmentInner, not {}", return_type)
        };


        quote! {
            #[macro_export]
            macro_rules! #macro_ident {
                #shout_args_arm

                #(#match_arms)*
                (@parse_args [#($#arg_names:ident,)*] ) => { };

                (@construct listenable=$listenables:ident, context=$context:expr) => {{
                    use narui::args::ContextArgs;
                    #transformer
                    let args = $context.listen_args(&$listenables.0);
                    unsafe {
                        transformer(#mod_ident::#function_ident(
                            #($listenables.#arg_numbers_plus_one.parse(&*args[#arg_numbers]).clone(),)*
                            $context
                        ))
                    }
                }}
            }

            // we do this to have correct scoping of the macro. It should not just be placed at the
            // crate root but rather at the path of the original function.
            pub use #macro_ident as #macro_ident_pub;
        }
    };


    let data_constructor_function = {
        let span = function_ident.span();
        let LineColumn { line, column } = span.start();
        let arg_names =
            get_arg_names(&function).into_iter().filter(|ident| &ident.to_string() != "context");
        let source_loc = format!("unknown:{}:{}", line, column);

        quote! {
            pub static WIDGET_ID: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(0);

            #[narui::internal::ctor]
            fn _init_widget() {
                let mut lock = narui::internal::WIDGET_INFO.write();
                let id = lock.len();
                WIDGET_ID.store(id as u16, std::sync::atomic::Ordering::SeqCst);
                lock.push(narui::internal::WidgetDebugInfo {
                    name: stringify!(#mod_ident).to_string(),
                    loc: #source_loc.to_string(),
                    arg_names: vec![#(stringify!(#arg_names).to_string(),)*],
                })
            }
        }
    };

    let (transformed_function, original_ident, new_ident) =
        transform_function_args_to_context(function.clone());
    let function_vis = function.vis;

    let transformed = quote! {
        #transformed_function
        #function_vis mod #mod_ident {
            #data_constructor_function
            #constructor_macro
            pub use super::#new_ident as #original_ident;
        }
    };
    println!("widget: \n{}\n\n", transformed.clone());
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
fn transform_function_args_to_context(
    function: ItemFn,
) -> (proc_macro2::TokenStream, Ident, Ident) {
    let function_clone = function.clone();
    let ItemFn { attrs, vis: _, mut sig, block } = function;
    let original_ident = sig.ident;
    let new_ident = desinfect_ident(&original_ident);
    sig.ident = new_ident.clone();
    let stmts = &block.stmts;
    let context_string = get_arg_types(&function_clone)
        .iter()
        .filter(|(_, ty)| {
            ty.to_token_stream().to_string().replace("-> ", "") == WIDGET_CONTEXT_TYPE_STRING
        })
        .next()
        .unwrap()
        .0
        .to_string();
    let context_ident = Ident::new(&context_string, Span::call_site());
    let LineColumn { line, column } = Span::call_site().start();
    let function_transformed = quote! {
        #(#attrs)* pub #sig {
            context.widget_loc = (#line, #column);
            let to_return = {
                #(#stmts)*
            };
            std::mem::drop(&#context_ident);
            to_return
        }
    };
    (function_transformed, original_ident, new_ident)
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
