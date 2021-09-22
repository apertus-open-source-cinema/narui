use crate::narui_crate;
use proc_macro2::{Ident, LineColumn, Span, TokenStream};
use proc_macro_error::{abort, abort_call_site};
use quote::{quote, quote_spanned};
use std::collections::BTreeMap;
use syn::spanned::Spanned;
use syn_rsx::{Node, NodeType};

pub fn rsx(input: proc_macro::TokenStream) -> TokenStream {
    let mut parsed =
        syn_rsx::parse2(input.into()).unwrap_or_else(|err| abort!(err.span(), err.to_string()));
    if parsed.len() != 1 {
        abort_call_site!("the rsx macro must have exactly one child!");
    };
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
            Ident::new(&format!("__{}_{}_args", name_str, loc), node_span(&x));
        let key_ident = Ident::new(&format!("__{}_{}_key", name_str, loc), node_span(&x));
        let idx_ident = Ident::new(&format!("__{}_{}_idx", name_str, loc), node_span(&x));
        let mut key = quote! {#narui::KeyPart::Fragment { widget_id: #name::WIDGET_ID.load(std::sync::atomic::Ordering::SeqCst), location_id: #loc }};

        let span_ident = Ident::new("_span_", node_span(&x));

        let constructor_path = {
            let constructor_ident = Ident::new(&format!("{}_constructor", name), node_name.span());
            let mod_ident = Ident::new(&format!("{}", name), node_span(&x));
            quote! {#mod_ident::#constructor_ident}
        };
        let mut processed_attributes = BTreeMap::new();
        for attribute in &x.attributes {
            let name = attribute.name.as_ref().unwrap();
            let value = if let Some(x) = attribute.value.as_ref() {
                let span = name.span().join(x.span()).unwrap_or_else(|| name.span());
                quote_spanned! {span=> { #[allow(unused_braces)] #x } }
            } else {
                let span = name.span();
                quote_spanned! {span=> { compile_error!("All rsx attributes need to have values!") } }
            };

            if name.to_string() == "key" {
                key = quote! {#narui::KeyPart::FragmentKey { widget_id: #node_name::WIDGET_ID.load(std::sync::atomic::Ordering::SeqCst), location_id: #loc, key: #value as _ }}
            } else {
                processed_attributes.insert(name.to_string(), quote! {#name=#value});
            }
        }
        let beginning = if x.children.is_empty() {
            quote! {}
        } else if x.children.len() == 1 && x.children[0].node_type == NodeType::Block {
            let value = x.children[0].value.as_ref().unwrap();
            processed_attributes.insert("children".to_string(), quote! {children=(#value)});
            quote! {}
        } else {
            let len = x.children.len();
            let (beginning, inplace): (Vec<_>, Vec<_>) =
                x.children.into_iter().map(handle_rsx_node).unzip();
            if len == 1 {
                processed_attributes
                    .insert("children".to_string(), quote! {children={#(#inplace)*.into()}});
            } else {
                processed_attributes.insert(
                    "children".to_string(),
                    quote! {children={#narui::smallvec![#(#inplace,)*]}},
                );
            }
            quote! {#(#beginning)*}
        };

        let processed_attributes = processed_attributes.values();
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
                    span=#span_ident,
                    context=context,
                    idx=#idx_ident,
                    #(#processed_attributes,)*
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
        abort!(
            x.value.unwrap().span(),
            "you shall not give inputs of type '{}' to the rsx macro",
            x.node_type
        );
    }
}
fn node_span(node: &Node) -> Span {
    match node.node_type {
        NodeType::Element => node.attributes.iter().fold(node.name.span(), |acc, v| {
            acc.join(node_span(v)).unwrap_or_else(|| node.name.span())
        }),
        NodeType::Attribute => {
            node.name.span().join(node.value.span()).unwrap_or_else(|| node.name.span())
        }
        NodeType::Text => node.value.span(),
        NodeType::Comment => node.value.span(),
        NodeType::Doctype => node.value.span(),
        NodeType::Fragment => node.value.span(),
        NodeType::Block => node.value.span(),
    }
}
