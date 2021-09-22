mod kw_arg_macro;
mod rsx_macro;
mod widget_macro;

use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use proc_macro_error::proc_macro_error;
use quote::quote;
use regex::Regex;

#[proc_macro_error]
#[proc_macro_attribute]
pub fn widget(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    widget_macro::widget(args, item)
}

#[proc_macro_error]
#[proc_macro]
pub fn rsx(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    rsx_macro::rsx(input).into()
}

#[proc_macro_error]
#[proc_macro]
pub fn rsx_toplevel(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let rsx = rsx_macro::rsx(input);
    let narui = narui_crate();
    let loc = get_span_start_byte(Span::call_site());

    (quote! {
        #narui::UnevaluatedFragment {
            key: Default::default(),
            gen: std::rc::Rc::new(|context: &mut #narui::WidgetContext| {
                let __widget_loc_start = #loc;
                #narui::FragmentInner::Node {
                    children: #narui::smallvec![ #rsx ],
                    layout: Box::new(#narui::Transparent),
                    is_clipper: false,
                }
            }),
        }
    })
    .into()
}

#[proc_macro_error]
#[proc_macro]
pub fn kw_arg_call(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    kw_arg_macro::kw_arg_call(input)
}


fn narui_crate() -> TokenStream {
    let found_crate = found_crate_to_tokens(
        crate_name("narui")
            .or_else(|_| crate_name("narui_core"))
            .expect("narui is present in `Cargo.toml`"),
    );

    quote! { #found_crate::_macro_api }
}
fn narui_macros() -> TokenStream {
    crate_name("narui")
        .map(|x| {
            let found_crate = found_crate_to_tokens(x);
            quote! { #found_crate::_macros }
        })
        .and_then(|_| {
            let found_crate = found_crate_to_tokens(crate_name("narui_macros")?);
            Ok(quote! { #found_crate })
        })
        .expect("narui is present in `Cargo.toml`")
}
fn found_crate_to_tokens(x: FoundCrate) -> TokenStream {
    match x {
        FoundCrate::Itself => quote!(crate),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( #ident )
        }
    }
}


fn get_span_start_byte(span: Span) -> usize {
    let re = Regex::new(r"#\d+ bytes\((\d+)..(\d+)\)").unwrap();
    let span_str = &format!("{:?}", span);
    let start = re
        .captures(span_str)
        .expect(&format!("span format changed ('{}')", span_str))
        .get(1)
        .unwrap()
        .as_str();
    start.parse().unwrap()
}
