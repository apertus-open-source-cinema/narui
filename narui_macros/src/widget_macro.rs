use crate::narui_crate;
use bind_match::bind_match;
use core::result::{Result, Result::Ok};
use proc_macro2::{Ident, LineColumn, Literal, Span};
use proc_macro_error::{abort, abort_call_site};
use quote::{quote, ToTokens};
use std::collections::{HashMap, HashSet};
use syn::{
    parse::{Parse, ParseStream, Parser},
    punctuated::Punctuated,
    spanned::Spanned,
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

// TODO(anujen): this breaks when it is not imported
const WIDGET_CONTEXT_TYPE_STRING: &str = "& mut WidgetContext";

// allows for kwarg-style calling of functions
pub fn widget(
    defaults: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let narui = narui_crate();

    // parse the function
    let parsed: Result<ItemFn, _> = syn::parse2(item.into());
    let function = parsed.unwrap();
    let function_ident = function.sig.ident.clone();
    let mod_ident = Ident::new(&format!("{}", function_ident), Span::call_site());

    let last_arg = get_arg_names(&function).into_iter().last().unwrap_or_else(|| {
        abort_call_site!("widget functions need to take at least one argument (the context)")
    });
    let last_name = last_arg.to_string();
    let last_type = get_arg_types(&function)[&last_name].clone();
    if last_name != "context" {
        abort!(last_arg.span(), "widget functions need to the context as the last argument")
    }
    if last_type.to_token_stream().to_string() != WIDGET_CONTEXT_TYPE_STRING {
        abort!(
            last_arg.span(),
            "the last arg need to be the context and have type '{}'",
            WIDGET_CONTEXT_TYPE_STRING
        )
    }

    let return_type = function.sig.output.to_token_stream().to_string().replace("-> ", "");

    // parse & format the default arguments
    let parser = Punctuated::<AttributeParameter, Token![,]>::parse_terminated;
    let parsed_defaults = parser.parse(defaults).unwrap();

    let macro_ident = Ident::new(&format!("__{}_constructor_", function_ident), Span::call_site());
    let macro_ident_pub = Ident::new(&format!("{}_constructor", function_ident), Span::call_site());

    let arg_types = get_arg_types(&function);
    let mut has_default = HashSet::new();
    let mut default_fns = vec![];
    let mut default_fn_uses = vec![];
    let mut initializers = vec![];
    for x in &parsed_defaults {
        let ty = if let Some(ty) = arg_types.get(&x.ident.to_string()) {
            ty.as_ref()
        } else {
            abort!(x.ident.span(), "specified default value for non existent argument {}", &x.ident)
        };

        has_default.insert(x.ident.to_string());

        let default_fn_ident =
            Ident::new(&format!("__{}_{}_default_arg", function_ident, x.ident), x.ident.span());
        let default_fn_ident_pub = Ident::new(&format!("{}_default_arg", x.ident), x.ident.span());

        let expr = &x.expr;

        default_fns.push(quote! {
            #[allow(unused)]
            pub fn #default_fn_ident() -> #ty {
                #expr
            }
        });
        default_fn_uses.push(quote! {
            pub use super::#default_fn_ident as #default_fn_ident_pub;
        });
        let ident = &x.ident;
        initializers.push(quote! {
            #[allow(unused)]
            let #ident = #mod_ident::#default_fn_ident_pub();
        });
    }

    let mut constrain_fns = vec![];
    let mut constrain_fn_uses = vec![];
    let mut arg_names = get_arg_names(&function);
    arg_names.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
    arg_names = arg_names.into_iter().filter(|ident| &ident.to_string() != "context").collect();

    let mut match_arms = Vec::new();
    let first_arg = arg_names.get(0);
    for (this_arg, next_arg) in
        arg_names.iter().zip(arg_names.iter().skip(1).map(|x| Some(x)).chain(Some(None)))
    {
        let constrain_fn_ident = Ident::new(
            &format!("__{}_{}_constrain_arg", function_ident, this_arg),
            this_arg.span(),
        );
        let constrain_fn_ident_pub =
            Ident::new(&format!("{}_constrain_arg", this_arg), this_arg.span());
        let arg_type = arg_types[&this_arg.to_string()].as_ref();

        constrain_fns.push(quote! {
            #[allow(unused)]
            pub fn #constrain_fn_ident(arg: #arg_type) -> #arg_type { arg }
        });
        constrain_fn_uses.push(quote! {
            pub use super::#constrain_fn_ident as #constrain_fn_ident_pub;
        });

        let next_branch = if let Some(next) = next_arg {
            quote! {
                #mod_ident::#macro_ident_pub!(@parse_arg(#next) [#($#arg_names,)*] $($rest)*);
            }
        } else {
            quote! {}
        };
        match_arms.push(if has_default.contains(&this_arg.to_string()) {
            quote! {
                (@parse_arg(#this_arg) [#($#arg_names:ident,)*] #this_arg = $value:expr, $($rest:tt)*) => {
                    #[allow(unused)]
                    let $#this_arg = #mod_ident::#constrain_fn_ident_pub($value);
                    #next_branch
                };
                (@parse_arg(#this_arg) [#($#arg_names:ident,)*] $($rest:tt)*) => {
                    #next_branch
                };
            }
        } else {
            let error_message =
                format!("argument '{}' of {} is non-optional and missing", this_arg, function_ident);
            quote! {
                (@parse_arg(#this_arg) [#($#arg_names:ident,)*] #this_arg = $value:expr,$($rest:tt)*) => {
                    #[allow(unused)]
                    let $#this_arg = #mod_ident::#constrain_fn_ident_pub($value);
                    #next_branch
                };
                (@parse_arg(#this_arg) [#($#arg_names:ident,)*] $($rest:tt)*) => {
                    compile_error!(#error_message);
                };
            }
        })
    }

    let constructor_macro = {
        let arg_numbers: Vec<_> = (0..(arg_names.len())).map(Literal::usize_unsuffixed).collect();
        let arg_numbers_plus_one: Vec<_> =
            (0..(arg_names.len())).map(|i| Literal::usize_unsuffixed(i + 1)).collect();

        let function_call = quote! {{
            #[allow(unused_unsafe)]
            unsafe {
                #mod_ident::#function_ident(
                    #($listenables.#arg_numbers_plus_one.parse(&*args[#arg_numbers]).clone(),)*
                    $context
                )
            }
        }};
        let function_call = if return_type == "FragmentInner" {
            function_call
        } else if return_type == "Fragment" {
            quote! { #narui::FragmentInner::from_fragment(#function_call) }
        } else {
            abort!(
                function.sig.output.to_token_stream().span(),
                "widgets need to either return Fragment or FragmentInner, not '{}'",
                return_type
            )
        };

        let mut args_string = String::new();
        for (i, name) in arg_names.iter().enumerate() {
            if i == 0 {
                args_string += &format!("{}", name);
            } else if i < arg_names.len() {
                args_string += &format!(", {}", name);
            } else {
                args_string += &format!(" and {}", name);
            }
        }

        let unexpected_arg_error =
            format!("{} only accepts {} as arguments", function_ident, args_string);

        let first_parse = if let Some(next) = first_arg {
            quote! {
                #mod_ident::#macro_ident_pub!(@parse_arg(#next) [#(#arg_names,)*] $($args)*);
            }
        } else {
            quote! {}
        };
        let arg_names_in_function_order: Vec<_> = get_arg_names(&function)
            .into_iter()
            .filter(|ident| &ident.to_string() != "context")
            .collect();
        quote! {
            #[macro_export]
            macro_rules! #macro_ident {
                (@shout_args context=$context:expr, idx=$idx:ident, $($args:tt)*) => {{
                    #(#initializers)*
                    #first_parse
                    #narui::shout_args!($context, $idx, [#(#arg_names_in_function_order,)*])
                }};

                #(#match_arms)*
                // generate an error if a needed thing is not present
                (@parse_arg($expected_next:ident) [#($#arg_names:ident,)*] $unexpected:ident=$value:expr, $($rest:tt)*) => {
                    $unexpected;
                    compile_error!(#unexpected_arg_error);
                };

                (@construct listenable=$listenables:ident, context=$context:expr) => {{
                    #[allow(unused)]
                    let args = #narui::listen_args($context, &$listenables.0);
                    #function_call
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
        let source_loc = format!("unknown:{}:{}", line, column);

        quote! {
            pub static WIDGET_ID: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(0);

            #[#narui::ctor]
            fn _init_widget() {
                let mut lock = #narui::WIDGET_INFO.write();
                let id = lock.len();
                WIDGET_ID.store(id as u16, std::sync::atomic::Ordering::SeqCst);
                lock.push(#narui::WidgetDebugInfo {
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
        #(#default_fns)*
        #(#constrain_fns)*
        #function_vis mod #mod_ident {
            #(#default_fn_uses)*
            #(#constrain_fn_uses)*
            #data_constructor_function
            #constructor_macro
            pub use super::#new_ident as #original_ident;
        }
    };
    // println!("widget: \n{}\n\n", transformed);
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
        .find(|(_, ty)| {
            ty.to_token_stream().to_string().replace("-> ", "") == WIDGET_CONTEXT_TYPE_STRING
        })
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
            #[allow(unused)]
            fn __swallow<T>(_context: T) {}
            __swallow(#context_ident);
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

// creates a hygienic (kind of) identifier from an unhyginic one
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
