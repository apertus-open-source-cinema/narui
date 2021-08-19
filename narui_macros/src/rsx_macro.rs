use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};
use syn_rsx::{Node, NodeType};

pub fn rsx(input: proc_macro::TokenStream) -> TokenStream {
    let mut parsed = syn_rsx::parse2(input.into()).unwrap();
    if parsed.len() == 0 {
        let loc = {
            let mut s = DefaultHasher::new();
            (format!("{:?}", Span::call_site())).hash(&mut s);
            s.finish()
        };
        return quote! {{
            Fragment {
                key: context.widget_local.key.with(KeyPart::Rsx(#loc)),
                gen: std::sync::Arc::new(move |_context| FragmentInner {
                    layout_object: None,
                    children: vec![],
                }),
            }
        }};
    }
    assert!(parsed.len() == 1);
    let (begining, inplace) = handle_rsx_node(parsed.remove(0));

    let transformed = quote! {{
        #begining
        #inplace
    }};

    // println!("rsx: \n{}\n\n", transformed);
    transformed.into()
}
fn handle_rsx_node(x: Node) -> (TokenStream, TokenStream) {
    if x.node_type == NodeType::Element {
        let name = x.name.as_ref().unwrap();
        let name_str = name.to_string();
        let loc = {
            let mut s = DefaultHasher::new();
            (format!("{:?}", name.span())).hash(&mut s);
            s.finish()
        };
        let loc_str = format!("{}", loc);

        let args_listenable_ident =
            Ident::new(&format!("__{}_{}_args", name_str, loc_str), Span::call_site());
        let mut key = quote! {KeyPart::Fragment { name: #name_str, loc: #loc_str }};

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
                key = quote! {KeyPart::FragmentKey { name: #name_str, loc: #loc_str, hash: KeyPart::calculate_hash(#value) }}
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
            let (beginning, inplace): (Vec<_>, Vec<_>) =
                x.children.into_iter().map(|child| handle_rsx_node(child)).unzip();
            (quote! {#(#beginning)*}, quote! {children={vec![#(#inplace,)*]},})
        };

        let beginning = quote! {
            let #args_listenable_ident = {
                let context = {
                    let mut context = context.clone();
                    context.widget_local.key = context.widget_local.key.with(#key);
                    context
                };

                #beginning

                #constructor_path!(
                    @shout_args
                    context=(context.clone()),
                    #(#processed_attributes,)*
                    #children_processed
                )
            };
        };
        let inplace = quote! {
            Fragment {
                key: context.widget_local.key.with(#key),
                gen: std::sync::Arc::new(move |context: Context| {
                    #constructor_path!(@construct listenable=#args_listenable_ident, context=context)
                })
            }
        };
        (beginning, inplace)
    } else {
        panic!("you shal not give this input to the rsx macro")
    }
}
