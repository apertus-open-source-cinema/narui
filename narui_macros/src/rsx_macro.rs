use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn_rsx::{Node, NodeType};

pub fn rsx(input: proc_macro::TokenStream) -> TokenStream {
    let parsed = syn_rsx::parse2(input.into()).unwrap();
    let (begining, inplace) =
        handle_rsx_nodes(&parsed, "rsx");
    let transformed = quote! {{
        #begining

        #inplace
    }};
    println!("rsx: \n{}\n\n", transformed);
    transformed.into()
}

fn handle_rsx_nodes(
    input: &Vec<Node>,
    parent: &str,
) -> (TokenStream, TokenStream) {
    let fragment_ident = Ident::new(&format!("__{}_children", parent), Span::call_site());

    if input.iter().all(|x| x.node_type == NodeType::Element) {
        let (beginning, inplace): (Vec<_>, Vec<_>) = input.iter().map(|x| {
            let name = x.name.as_ref().unwrap();
            let name_str = name.to_string();
            let loc = format!("{}_{}", name.span().start().line, name.span().start().column);

            let args_listenable_ident = Ident::new(&format!("__{}_{}_args", name_str, loc), Span::call_site());

            let this_str = &format!("{}_{}", name_str, loc);
            let mut key = quote! {KeyPart::Fragment { name: #name_str, loc: #loc }};

            let constructor_ident = Ident::new(&format!("__{}_constructor", name), Span::call_site());
            let mut processed_attributes = vec![];
            for attribute in &x.attributes {
                let name = attribute.name.as_ref().unwrap();
                let value = attribute.value.as_ref().unwrap().clone();
                if name.to_string() == "key" {
                    key = quote! {KeyPart::FragmentKey { name: #name_str, loc: #loc, hash: KeyPart::calculate_hash(#value) }}
                } else {
                    processed_attributes.push(quote! {#name=#value});
                }
            }
            let (beginning, children_processed) = if x.children.is_empty() {
                (quote! {}, quote! {})
            } else {
                let (begining, inplace) = handle_rsx_nodes(&x.children, this_str);
                (begining, quote! {
                    children=#inplace,
                })
            };

            let beginning = quote! {
                #beginning
                let #args_listenable_ident = #constructor_ident!(@shout_args context=context, key_part=#key, #(#processed_attributes,)* #children_processed);
            };
            let inplace = quote! {(
                #key,
                std::sync::Arc::new(move |context: Context| {
                    #constructor_ident!(@construct listenable=#args_listenable_ident, context=context )
                })
            )};

            (beginning, inplace)
        }).unzip();

        let to_beginning = quote! {
            #(#beginning)*

            let #fragment_ident = Fragment {
                key_part: context.widget_local.key.last_part(),
                children: vec![#(#inplace,)*],
                layout_object: None,
            };
        };
        let inplace = quote! {#fragment_ident.clone()};
        (to_beginning, inplace)
    }
    else if input.len() == 1 {
        let value = input.iter().next().unwrap().value.as_ref().unwrap();

        let to_beginning = quote! {};
        let inplace = quote! {#value};
        (to_beginning, inplace)
    }
    else {
        panic!("each rsx node can either contain n nodes or one block, got {:?}", input);
    }
}
