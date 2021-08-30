use proc_macro2::{Ident, LineColumn, Span, TokenStream};
use quote::quote;

use crate::narui_crate;
use syn_rsx::{Node, NodeType};

pub fn rsx(input: proc_macro::TokenStream) -> TokenStream {
    let mut parsed = syn_rsx::parse2(input.into()).unwrap();
    assert_eq!(parsed.len(), 1, "the rsx macro must have exactly one child!");
    let (beginning, inplace) = handle_rsx_node(parsed.remove(0));

    let transformed = quote! {{
        context.local_hook = false;
        #beginning
        let ret = { #inplace };
        context.local_hook = true;
        ret
    }};

    // println!("rsx: \n{}\n\n", transformed);
    transformed
}
fn handle_rsx_node(x: Node) -> (TokenStream, TokenStream) {
    let narui = narui_crate();

    if x.node_type == NodeType::Element {
        let name = x.name.as_ref().unwrap();
        let node_name = name;
        let name_str = name.to_string();
        let loc = {
            let LineColumn { line, column } = name.span().start();
            // only every fourth part of the column is relevant, because the minimum size
            // for a rsx node is four: <a />
            let column = (column / 4) & 0b11_1111;
            // use 6 bits for column (good for up to 256 columns)
            // use 10 bits for line (good for up to 1024 lines)
            let line = (line & 0b11_1111_1111) << 6;
            (column as u16) | (line as u16)
        };

        let args_listenable_ident =
            Ident::new(&format!("__{}_{}_args", name_str, loc), Span::call_site());
        let key_ident = Ident::new(&format!("__{}_{}_key", name_str, loc), Span::call_site());
        let idx_ident = Ident::new(&format!("__{}_{}_idx", name_str, loc), Span::call_site());
        let mut key = quote! {#narui::KeyPart::Fragment { widget_id: #name::WIDGET_ID.load(std::sync::atomic::Ordering::SeqCst), location_id: #loc }};

        let constructor_path = {
            let constructor_ident =
                Ident::new(&format!("__{}_constructor", name), Span::call_site());
            let mod_ident = Ident::new(&format!("{}", name), Span::call_site());
            quote! {#mod_ident::#constructor_ident}
        };
        let mut processed_attributes = vec![];
        for attribute in &x.attributes {
            let name = attribute.name.as_ref().unwrap();
            let value = attribute.value.as_ref().unwrap().clone();
            if name.to_string() == "key" {
                key = quote! {#narui::KeyPart::FragmentKey { widget_id: #node_name::WIDGET_ID.load(std::sync::atomic::Ordering::SeqCst), location_id: #loc, key: #value as _ }}
            } else {
                processed_attributes.push(quote! {#name=#value});
            }
        }
        let (beginning, children_processed) = if x.children.is_empty() {
            (quote! {}, quote! {})
        } else if x.children.len() == 1 && x.children[0].node_type == NodeType::Block {
            let value = x.children[0].value.as_ref().unwrap();
            (quote! {}, quote! {children=(#value),})
        } else {
            let len = x.children.len();
            let (beginning, inplace): (Vec<_>, Vec<_>) =
                x.children.into_iter().map(handle_rsx_node).unzip();
            if len == 1 {
                (quote! {#(#beginning)*}, quote! {children={#(#inplace)*.into()},})
            } else {
                (quote! {#(#beginning)*}, quote! {children={#narui::smallvec![#(#inplace,)*]},})
            }
        };

        let beginning = quote! {
            let (#key_ident, #idx_ident) = {
                let fragment_store = &mut context.fragment_store;
                let key = context.key_map.key_with(context.widget_local.key, #key, || fragment_store.add_empty_fragment().into());
                let idx = #narui::Fragment::from(key);
                (key, idx)
            };
            let #args_listenable_ident = {
                let old_key = context.widget_local.key;
                context.widget_local.key = #key_ident;

                #beginning
                let to_return = #constructor_path!(
                    @shout_args
                    context=context,
                    idx=#idx_ident,
                    #(#processed_attributes,)*
                    #children_processed
                );

                context.widget_local.key = old_key;
                context.fragment_store.reset_external_hook_count(#idx_ident);
                to_return
            };
        };
        let inplace = quote! {
            context.fragment_store.add_fragment(#idx_ident, || {
                #narui::UnevaluatedFragment {
                    key: #key_ident,
                    gen: std::rc::Rc::new(move |context: &mut #narui::WidgetContext| {
                        #constructor_path!(@construct listenable=#args_listenable_ident, context=context)
                    })
                }
            })
        };
        (beginning, inplace)
    } else {
        panic!("you shall not give this input to the rsx macro")
    }
}
