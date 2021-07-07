mod rsx_macro;
mod widget_macro;

use quote::quote;

#[proc_macro]
pub fn rsx(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    rsx_macro::rsx(input).into()
}

#[proc_macro]
pub fn rsx_toplevel(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let rsx = rsx_macro::rsx(input);

    (quote! {
        Fragment {
            key: Default::default(),
            children: vec![(KeyPart::Nop, std::sync::Arc::new(|context| { #rsx }))],
            layout_object: None,
        }
    })
    .into()
}


#[proc_macro_attribute]
pub fn widget(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    widget_macro::widget(args, item)
}

#[proc_macro]
pub fn color(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let string = input.to_string();
    let trimmed = string.trim_start_matches("# ");
    let str_to_float =
        |s| i64::from_str_radix(s, 16).unwrap() as f32 / 16u32.pow(s.len() as u32) as f32;
    match trimmed.len() {
        6 => {
            let r = str_to_float(&trimmed[0..2]);
            let g = str_to_float(&trimmed[2..4]);
            let b = str_to_float(&trimmed[4..6]);
            (quote! {
                Color {
                    color: palette::rgb::Rgb {
                        red: #r,
                        green: #g,
                        blue: #b,
                        standard: core::marker::PhantomData,
                    },
                    alpha: 1.0
                }
            })
            .into()
        }
        _ => {
            unimplemented!()
        }
    }
}
