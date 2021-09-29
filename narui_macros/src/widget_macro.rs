use crate::{get_span_start_byte, narui_crate, narui_macros};
use bind_match::bind_match;
use proc_macro2::{Ident, LineColumn, Literal, Span, TokenStream};
use proc_macro_error::{abort, abort_call_site};
use quote::{quote, quote_spanned, ToTokens};
use std::{collections::HashMap, mem};
use syn::{spanned::Spanned, FnArg, ItemFn, Pat, PathArguments, Type, TypeParamBound};

// TODO(anujen): this breaks when it is not imported
const WIDGET_CONTEXT_TYPE_STRING: &str = "& mut WidgetContext";

pub fn widget(
    defaults: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    assert!(defaults.is_empty(), "supplying default arguments via attribute args is unsupported!");

    let function: ItemFn = syn::parse2(item.into()).unwrap();
    let function_ident = function.sig.ident.clone();
    let mod_ident = Ident::new(&format!("{}", function_ident), function_ident.span());

    let mut in_mod = Vec::new();

    check_function(&function);
    generate_function(&function, &mut in_mod);
    generate_constructor_macro(&function, &mod_ident, &mut in_mod);
    generate_data_constructor_function(&function, &mod_ident, &mut in_mod);

    let function_vis = function.vis;
    let transformed = quote! {
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

fn generate_constructor_macro(function: &ItemFn, mod_ident: &Ident, in_mod: &mut Vec<TokenStream>) {
    let narui = narui_crate();
    let arg_names = get_arg_names(function);

    let shout_args = generate_shout_args_macro_part(function, mod_ident, in_mod);
    let arg_numbers: Vec<_> = (0..(arg_names.len())).map(Literal::usize_unsuffixed).collect();
    let function_ident = &function.sig.ident;

    let function_call = quote! {{
        #[allow(unused_unsafe)]
        unsafe {
            #mod_ident::#function_ident(
                #($listenables.1.#arg_numbers.parse(&*args[#arg_numbers]).clone(),)*
                $context
            )
        }
    }};

    let return_type = function.sig.output.to_token_stream().to_string().replace("-> ", "");
    let function_call = if return_type == "FragmentInner" {
        function_call
    } else if return_type == "Fragment" {
        quote! { #mod_ident::FragmentInner::from_fragment(#function_call) }
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
                let args = #mod_ident::listen_args($context, &$listenables.0);
                #function_call
            }}
        }

        pub use #narui::listen_args as listen_args;
        pub use #narui::FragmentInner as FragmentInner;

        // we do this to have correct scoping of the macro. It should not just be placed at the
        // crate root but rather at the path of the original function.
        pub use #macro_ident as #macro_ident_pub;
    });
}

fn generate_shout_args_macro_part(
    function: &ItemFn,
    mod_ident: &Ident,
    in_mod: &mut Vec<TokenStream>,
) -> TokenStream {
    let narui = narui_crate();
    let narui_macros = narui_macros();

    let parsed_defaults: HashMap<_, _> = function
        .sig
        .inputs
        .iter()
        .filter_map(|input| {
            let input = bind_match!(input, FnArg::Typed(x) => x).unwrap();
            let name = input.pat.to_token_stream().to_string();

            let default =
                input.attrs.iter().find(|x| x.path.to_token_stream().to_string() == "default");
            if default.is_none() {
                return None;
            }
            let default = default.unwrap();
            let expr = if default.tokens.is_empty() {
                if let Some(default_fn) = generate_default_function(&input.ty) {
                    default_fn
                } else {
                    quote_spanned! {input.span()=>
                        Default::default()
                    }
                }
            } else {
                let tokens = &default.tokens;
                quote! { #tokens }
            };

            Some((name, quote! { #expr }))
        })
        .collect();

    let arg_names: Vec<_> = get_arg_names(function);
    let arg_types = get_arg_types(function);

    let mut initializers = vec![];
    for arg in &arg_names {
        let ty = arg_types.get(&arg.to_string()).unwrap().as_ref();
        if let Some(default) = parsed_defaults.get(&arg.to_string()) {
            let default_fn_ident_pub = Ident::new(&format!("{}_default_arg", arg), arg.span());

            in_mod.push(quote! {
                #[allow(unused)]
                pub fn #default_fn_ident_pub() -> #ty {
                    #default
                }
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
    let constrain_types_ident_pub = Ident::new("constrain_types", function.sig.span());
    in_mod.push(quote! {
        use super::*;
        #[allow(clippy::unused_unit)]
        pub fn #constrain_types_ident_pub(#(#inputs,)*) -> ((#(#types,)*), (#(#narui::ArgRef<#types>,)*)) {
            let arg_refs = (#(#narui::ArgRef::for_value(&#constrain_fn_input_idents),)*);
            ((#(#constrain_fn_input_idents,)*), arg_refs)
        }

        pub use #narui::shout_args as shout_args;
        pub use #narui_macros::kw_arg_call as kw_arg_call;
    });

    let arg_numbers: Vec<_> = (0..(arg_names.len())).map(Literal::usize_unsuffixed).collect();
    let widget_name = function.sig.ident.to_string();

    quote! {
        (@shout_args span=$span:ident, context=$context:expr, idx=$idx:ident, $($args:tt)*) => {{
            #[allow(unused_variables)]
            let (args_ordered, arg_refs) = #mod_ident::kw_arg_call!($span #widget_name
                #mod_ident::#constrain_types_ident_pub{#(#initializers,)*}($($args)*)
            );
            #mod_ident::shout_args!($context, $idx, #(args_ordered.#arg_numbers,)*);
            ($idx, arg_refs)
        }};
    }
}

fn generate_default_function(ty: &Type) -> Option<TokenStream> {
    fn generate_closure(arg_count: usize, span: Span) -> TokenStream {
        let args: Vec<_> =
            (0..arg_count).map(|i| Ident::new(&*format!("arg_{}", i), span)).collect();
        quote_spanned! {span=>
            (|#(#args,)*| {})
        }
    }
    match ty {
        Type::BareFn(bare_fn) => {
            if !bare_fn.output.to_token_stream().is_empty() {
                None
            } else {
                Some(generate_closure(bare_fn.inputs.len(), bare_fn.span()))
            }
        }
        Type::Paren(paren) => generate_default_function(paren.elem.as_ref()),
        Type::ImplTrait(impl_trait) => {
            for bound in impl_trait.bounds.iter() {
                if let TypeParamBound::Trait(trait_bound) = bound {
                    for segment in trait_bound.path.segments.iter() {
                        if let PathArguments::Parenthesized(args) = &segment.arguments {
                            return if !args.output.to_token_stream().is_empty() {
                                None
                            } else {
                                Some(generate_closure(args.inputs.len(), impl_trait.span()))
                            };
                        }
                    }
                }
            }
            None
        }
        _ => None,
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
    let last_type = get_arg_types(function)[&last_name].clone();
    if last_type.to_token_stream().to_string() != WIDGET_CONTEXT_TYPE_STRING {
        abort!(
            last_arg.span(),
            "the last arg need to be the context and have type '{}'",
            WIDGET_CONTEXT_TYPE_STRING
        )
    }
}

fn generate_function(function: &ItemFn, in_mod: &mut Vec<TokenStream>) {
    let ItemFn { attrs, vis: _, mut sig, block } = function.clone();
    for input in sig.inputs.iter_mut() {
        match input {
            FnArg::Receiver(_) => {}
            FnArg::Typed(t) => {
                let attrs = mem::take(&mut t.attrs);
                t.attrs = attrs
                    .into_iter()
                    .filter(|attr| attr.path.to_token_stream().to_string() != "default")
                    .collect()
            }
        }
    }

    let stmts = &block.stmts;
    let context_string = get_arg_types(function)
        .iter()
        .find(|(_, ty)| {
            ty.to_token_stream().to_string().replace("-> ", "") == WIDGET_CONTEXT_TYPE_STRING
        })
        .unwrap()
        .0
        .to_string();
    let context_ident = Ident::new(&context_string, Span::call_site());
    let loc = get_span_start_byte(function.span());
    in_mod.push(quote! {
        #(#attrs)* pub #sig {
            let __widget_loc_start = #loc;
            let to_return = {
                #(#stmts)*
            };
            #[allow(unused)]
            fn __swallow<T>(_context: T) {}
            __swallow(#context_ident);
            to_return
        }
    });
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
fn get_arg_types(function: &ItemFn) -> HashMap<String, Box<Type>> {
    function
        .clone()
        .sig
        .inputs
        .into_iter()
        .map(|arg| {
            let pat_type = bind_match!(arg.clone(), FnArg::Typed(x) => x).unwrap();
            (arg_ident(arg).to_string(), pat_type.ty)
        })
        .collect()
}
