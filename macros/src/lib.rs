use heck::AsTitleCase;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use rust_format::Formatter;
use syn::{parse_file, parse_macro_input, punctuated::Punctuated, ItemStruct, LitStr, Meta, Token, Ident};

#[proc_macro_derive(UiMarker, attributes(ui))]
pub fn marker(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}

fn _debug_token_stream(input: TokenStream2) -> TokenStream {
    let s = input.to_string();
    let s = rust_format::RustFmt::default().format_str(s).unwrap();
    std::fs::write("test.rs", s).unwrap();
    input.into()
}

#[proc_macro]
pub fn generate_ui_impl(input: TokenStream) -> TokenStream {
    let dir = parse_macro_input!(input as LitStr).value();

    let file = std::fs::read_to_string(dir).unwrap();

    let whole_file = parse_file(&file).unwrap();
    let mut impls = TokenStream2::new();
    // let mut full_definition = TokenStream2::new();
    
    // let mut full_impl = TokenStream2::new();

    for item in whole_file.items {
        let item_struct = match item {
            syn::Item::Struct(item_struct) => item_struct,
            _ => continue,
        };
        let struct_is_marked = item_struct
            .attrs
            .iter()
            .find(|attr| {
                if attr.path().is_ident("derive") {
                    let mut has_ui_marker = false;
                    let nested = attr
                        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                        .unwrap();
                    for meta in nested {
                        has_ui_marker |= meta.path().is_ident("UiMarker")
                    }
                    has_ui_marker
                } else {
                    false
                }
            })
            .is_some();

        if struct_is_marked {
            let (the_impl, _struct_name) = playground_ui(item_struct);
            impls.extend(the_impl);
        }
    }


    // debug_token_stream(impls)
    impls.into()
}


fn playground_ui(input: ItemStruct) -> (TokenStream2, Ident) {
    let struct_name = input.ident;
    
    let mut definitions = TokenStream2::new();
    let mut defaults = TokenStream2::new();
    let mut uis = TokenStream2::new();

    for field in input.fields.iter() {
        for attr in &field.attrs {
            if !attr.path().is_ident("playground") {
                continue;
            }
            let _meta_list =  match &attr.meta{
                syn::Meta::List(meta_list) => meta_list,
                _ => panic!("The attribute must be of the form #[playground(name1 = value1, name2 = value2)]"),
            };
        }

        let ident = field
            .ident
            .as_ref()
            .expect("The struct cannot be a tuple struct");

        let title_case = format!("{}", heck::AsTitleCase(ident.to_string()));
        let def = quote!(#ident: BoundedSlider,);
        definitions.extend(def);

        let default = quote!(#ident: BoundedSlider{
            name: #title_case,
            min: -100.,
            min_str: (-100.).to_string(),
            max: 100.,
            max_str: (100.).to_string(),
        },);
        defaults.extend(default);


        let ui_impl = quote!(
            changed |= self.#ident.show(ui, &mut data.#ident);
        );
        
        uis.extend(ui_impl);
    }

    let ui_struct_name = format_ident!("{}Ui", struct_name);

    let definition = quote!(
        pub struct #ui_struct_name{
            #definitions
            id: usize,
        }
    );

    let struct_name_title = format!("{}", AsTitleCase(struct_name.to_string()));

    let ui_impl = quote!(
        impl #ui_struct_name{
            pub fn new(id: usize) -> Self{
                Self{
                    #defaults
                    id,
                }
            }
            pub fn show(&mut self, ui: &mut Ui, data: &mut #struct_name) -> bool{
                let mut changed = false;
                CollapsingHeader::new(#struct_name_title).id_source(self.id).show(ui, |ui|{
                    #uis
                });
                changed
            }
        }
    );

    let out = quote!(
        #definition
        // #default
        #ui_impl
    );

    (out, ui_struct_name)
}
