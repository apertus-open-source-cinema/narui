mod rsx_macro;
mod widget_macro;

use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;

#[proc_macro_attribute]
pub fn widget(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    widget_macro::widget(args, item)
}


#[proc_macro]
pub fn rsx(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    rsx_macro::rsx(input).into()
}

#[proc_macro]
pub fn rsx_toplevel(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let rsx = rsx_macro::rsx(input);
    let narui = narui_crate();

    (quote! {
        UnevaluatedFragment {
            key: Default::default(),
            gen: std::rc::Rc::new(|context: &mut WidgetContext| {
                FragmentInner::Node {
                    children: #narui::smallvec![ #rsx ],
                    layout: Box::new(#narui::Transparent),
                    is_clipper: false,
                }
            }),
        }
    })
    .into()
}


fn narui_crate() -> TokenStream {
    let found_crate = crate_name("narui")
        .or_else(|_| crate_name("narui_core"))
        .expect("narui is present in `Cargo.toml`");

    match found_crate {
        FoundCrate::Itself => quote!(crate::_macro_api),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( #ident::_macro_api )
        }
    }
}
