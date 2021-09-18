use crate::{narui_crate, narui_macros};
use bind_match::bind_match;
use core::result::Result::Ok;
use proc_macro2::{Ident, LineColumn, Literal, Span, TokenStream};
use proc_macro_error::{abort, abort_call_site};
use quote::{quote, ToTokens};
use std::collections::HashMap;
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

pub fn widget(
    defaults: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let function: ItemFn = syn::parse2(item.into()).unwrap();
    let function_ident = function.sig.ident.clone();
    let mod_ident = Ident::new(&format!("{}", function_ident), function_ident.span());

    let mut not_in_mod = Vec::new();
    let mut in_mod = Vec::new();

    check_function(&function);
    generate_function(&function, &mut not_in_mod, &mut in_mod);
    generate_constructor_macro(&function, defaults, &mod_ident, &mut not_in_mod, &mut in_mod);
    generate_data_constructor_function(&function, &mod_ident, &mut in_mod);

    let function_vis = function.vis;
    let transformed = quote! {
        #(#not_in_mod)*
        #function_vis mod #mod_ident {
            #(#in_mod)*
        }
    };
    // println!("widget: \n{}\n\n", transformed);
    transformed.into()
}

fn generate_data_constructor_function(
    function: &ItemFn,
    mod_ident: &Ident,
    in_mod: &mut Vec<TokenStream>,
) {
    let narui = narui_crate();

    let span = function.sig.ident.span();
    let LineColumn { line, column } = span.start();
    let source_loc = format!("unknown:{}:{}", line, column);
    let arg_names = get_arg_names(function);

    in_mod.push(quote! {
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
    })
}

fn generate_constructor_macro(
    function: &ItemFn,
    defaults: proc_macro::TokenStream,
    mod_ident: &Ident,
    not_in_mod: &mut Vec<TokenStream>,
    in_mod: &mut Vec<TokenStream>,
) {
    let narui = narui_crate();
    let arg_names = get_arg_names(function);

    let shout_args =
        generate_shout_args_macro_part(&function, defaults, &mod_ident, not_in_mod, in_mod);
    let arg_numbers: Vec<_> = (0..(arg_names.len())).map(Literal::usize_unsuffixed).collect();
    let arg_numbers_plus_one: Vec<_> =
        (0..(arg_names.len())).map(|i| Literal::usize_unsuffixed(i + 1)).collect();

    let function_ident = &function.sig.ident;

    let function_call = quote! {{
        #[allow(unused_unsafe)]
        unsafe {
            #mod_ident::#function_ident(
                #($listenables.#arg_numbers_plus_one.parse(&*args[#arg_numbers]).clone(),)*
                $context
            )
        }
    }};

    let return_type = function.sig.output.to_token_stream().to_string().replace("-> ", "");
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

    let macro_ident = Ident::new(&format!("__{}_constructor", function_ident), Span::call_site());
    let macro_ident_pub = Ident::new(&format!("{}_constructor", function_ident), Span::call_site());

    in_mod.push(quote! {
        #[macro_export]
        macro_rules! #macro_ident {
            #shout_args

            (@construct listenable=$listenables:ident, context=$context:expr) => {{
                #[allow(unused)]
                let args = #narui::listen_args($context, &$listenables.0);
                #function_call
            }}
        }

        // we do this to have correct scoping of the macro. It should not just be placed at the
        // crate root but rather at the path of the original function.
        pub use #macro_ident as #macro_ident_pub;
    });
}

fn generate_shout_args_macro_part(
    function: &ItemFn,
    defaults: proc_macro::TokenStream,
    mod_ident: &Ident,
    not_in_mod: &mut Vec<TokenStream>,
    in_mod: &mut Vec<TokenStream>,
) -> TokenStream {
    let narui = narui_crate();
    let narui_macros = narui_macros();

    let parser = Punctuated::<AttributeParameter, Token![,]>::parse_terminated;
    let parsed_defaults: HashMap<_, _> = parser
        .parse(defaults)
        .unwrap()
        .iter()
        .map(|x| (x.ident.to_string(), x.expr.clone()))
        .collect();

    let arg_names: Vec<_> = get_arg_names(&function);
    let arg_types = get_arg_types(function);

    let mut initializers = vec![];
    for arg in &arg_names {
        let ty = arg_types.get(&arg.to_string()).unwrap().as_ref();
        if let Some(default) = parsed_defaults.get(&arg.to_string()) {
            let default_fn_ident =
                Ident::new(&format!("__{}_{}_default_arg", function.sig.ident, arg), arg.span());
            let default_fn_ident_pub = Ident::new(&format!("{}_default_arg", arg), arg.span());

            not_in_mod.push(quote! {
                #[allow(unused)]
                pub fn #default_fn_ident() -> #ty {
                    #default
                }
            });
            in_mod.push(quote! {
                pub use super::#default_fn_ident as #default_fn_ident_pub;
            });
            initializers.push(quote! {
                #arg = #mod_ident::#default_fn_ident_pub()
            });
        } else {
            initializers.push(quote! {
                #arg = @
            });
        }
    }

    let types: Vec<_> = arg_names.iter().map(|x| arg_types[&x.to_string()].as_ref()).collect();
    let constrain_fn_input_idents: Vec<_> = arg_names
        .iter()
        .enumerate()
        .map(|(i, x)| Ident::new(&*format!("arg_{}", i), x.span()))
        .collect();
    let inputs: Vec<_> = arg_names
        .iter()
        .zip(constrain_fn_input_idents.clone())
        .map(|(name, x)| {
            let ty = arg_types[&name.to_string()].as_ref();
            quote! { #x: #ty }
        })
        .collect();

    let arg_numbers: Vec<_> = (0..(arg_names.len())).map(Literal::usize_unsuffixed).collect();
    let widget_name = function.sig.ident.to_string();

    quote! {
        (@shout_args span=$span:ident, context=$context:expr, idx=$idx:ident, $($args:tt)*) => {{
            fn constrain_types(#(#inputs,)*) -> (#(#types,)*) {
                (#(#constrain_fn_input_idents,)*)
            }
            let args_ordered = #narui_macros::kw_arg_call!($span #widget_name
                constrain_types{#(#initializers,)*}($($args)*)
            );
            #narui::shout_args!($context, $idx, #(args_ordered.#arg_numbers,)*)
        }};
    }
}

fn check_function(function: &ItemFn) {
    let last_arg = arg_ident(
        function
            .sig
            .inputs
            .last()
            .unwrap_or_else(|| {
                abort_call_site!(
                    "widget functions need to take at least one argument (the context)"
                )
            })
            .clone(),
    );
    let last_name = last_arg.to_string();
    if last_name != "context" {
        abort!(last_arg.span(), "widget functions need to the context as the last argument")
    }
    let last_type = get_arg_types(&function)[&last_name].clone();
    if last_type.to_token_stream().to_string() != WIDGET_CONTEXT_TYPE_STRING {
        abort!(
            last_arg.span(),
            "the last arg need to be the context and have type '{}'",
            WIDGET_CONTEXT_TYPE_STRING
        )
    }
}

// adds the function arguments to the context as a `Listenable` and listen on it
// for partial re-evaluation.
fn generate_function(
    function: &ItemFn,
    not_in_mod: &mut Vec<TokenStream>,
    in_mod: &mut Vec<TokenStream>,
) {
    let ItemFn { attrs, vis: _, mut sig, block } = function.clone();
    let original_ident = sig.ident;
    let new_ident = desinfect_ident(&original_ident);
    sig.ident = new_ident.clone();
    let stmts = &block.stmts;
    let context_string = get_arg_types(&function)
        .iter()
        .find(|(_, ty)| {
            ty.to_token_stream().to_string().replace("-> ", "") == WIDGET_CONTEXT_TYPE_STRING
        })
        .unwrap()
        .0
        .to_string();
    let context_ident = Ident::new(&context_string, Span::call_site());
    let LineColumn { line, column } = Span::call_site().start();
    not_in_mod.push(quote! {
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
    });
    in_mod.push(quote! { pub use super::#new_ident as #original_ident; });
}

fn get_arg_names(function: &ItemFn) -> Vec<Ident> {
    function
        .clone()
        .sig
        .inputs
        .into_iter()
        .map(arg_ident)
        .filter(|ident| &ident.to_string() != "context")
        .collect()
}

fn arg_ident(arg: FnArg) -> Ident {
    let pat_type = bind_match!(arg, FnArg::Typed(x) => x).unwrap();
    let pat_ident = bind_match!(*pat_type.pat, Pat::Ident(x) => x).unwrap();
    pat_ident.ident
}

// creates a hygienic (kind of) identifier from an unhyginic one
fn desinfect_ident(ident: &Ident) -> Ident { Ident::new(&format!("__{}", ident), ident.span()) }

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
