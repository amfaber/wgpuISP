use heck::{AsTitleCase, AsSnakeCase};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use rust_format::Formatter;
use syn::{parse_file, parse_macro_input, punctuated::Punctuated, ItemStruct, LitStr, Meta, Token, Ident, Type, parse_quote};

#[proc_macro_derive(UiMarker, attributes(ui))]
pub fn marker(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}

#[proc_macro_derive(UiAggregation, attributes(ui))]
pub fn marker_aggregation(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}

fn _debug_token_stream(input: TokenStream2) -> TokenStream {
    let s = input.to_string();
    let s = rust_format::RustFmt::default().format_str(s).unwrap();
    std::fs::write("viewer/src/macro_debug.rs", s).unwrap();
    input.into()
}

#[proc_macro]
pub fn generate_ui_impl(input: TokenStream) -> TokenStream {
    let dir = parse_macro_input!(input as LitStr).value();

    let file = std::fs::read_to_string(dir).unwrap();

    let whole_file = parse_file(&file).unwrap();
    let mut impls = TokenStream2::new();

    
    let mut full_definition = TokenStream2::new();
    
    let mut full_new = TokenStream2::new();
    
    let mut full_ui = TokenStream2::new();
    
    let mut aggregation: Option<Ident> = None;

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
        
        let struct_is_aggregation = item_struct
            .attrs
            .iter()
            .find(|attr| {
                if attr.path().is_ident("derive") {
                    let mut has_ui_marker = false;
                    let nested = attr
                        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                        .unwrap();
                    for meta in nested {
                        has_ui_marker |= meta.path().is_ident("UiAggregation")
                    }
                    has_ui_marker
                } else {
                    false
                }
            })
            .is_some();

        if struct_is_aggregation{
            aggregation = Some(item_struct.ident.clone());
        }

        if struct_is_marked {
            let (the_impl, struct_name, ui_struct_name) = playground_ui(item_struct);
            let snake_case = format!("{}", AsSnakeCase(struct_name.to_string()));
            let var_name = format_ident!("{}", snake_case);
            full_definition.extend(quote!(#var_name: #ui_struct_name,));

            full_new.extend(quote!(#var_name: #ui_struct_name::new(ids()), ));

            full_ui.extend(quote!(changed |= self.#var_name.show(ui, &mut data.#var_name);));
            
            impls.extend(the_impl);
        }
    }

    let aggregation = aggregation.expect("The macro expects a single struct to be marked with derive(UiAggregation)");

    
    let out = quote!(
        #impls

        pub struct FullUi{
            #full_definition
        }

        impl FullUi{
            pub fn new(mut ids: impl FnMut() -> usize) -> Self{
                Self{
                    #full_new
                }
            }

            pub fn show(&mut self, ui: &mut Ui, data: &mut #aggregation) -> bool{
                let mut changed = false;
                #full_ui
                changed
            }
        }
    );

    // _debug_token_stream(out)
    out.into()
}


fn ui_element_by_type(ident: &Ident, title_case: &String, ty: &Type) -> (TokenStream2, TokenStream2){
    let float: Type = parse_quote!(f32);
    let int: Type = parse_quote!(i32);
    let glam_mat4: Type = parse_quote!(glam::Mat4);
    let mat4: Type = parse_quote!(Mat4);

    if ty == &float{
        let def = quote!(#ident: BoundedSlider,);
        let new = quote!(#ident: BoundedSlider{
            name: #title_case.to_string(),
            min: -100.,
            min_str: (-100.).to_string(),
            max: 100.,
            max_str: (100.).to_string(),
        },);

        (def, new)
    } else if ty == &int{
        let def = quote!(#ident: IntCheckbox,);
        let new = quote!(#ident: IntCheckbox{
            name: #title_case,
        },);

        (def, new)
    } else if ty == &mat4 || ty == &glam_mat4{
        let def = quote!(#ident: Mat4Slider,);
        let new = quote!(#ident: Mat4Slider::new(#title_case.to_string(), -1., 2.),);

        (def, new)
    } else {
        panic!("Unrecognized type in field of struct marked with UiMarker")
    }
}

fn playground_ui(input: ItemStruct) -> (TokenStream2, Ident, Ident) {
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
        let (def, new) = ui_element_by_type(ident, &title_case, &field.ty);
        
        definitions.extend(def);
        defaults.extend(new);


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
        #ui_impl
    );

    (out, struct_name, ui_struct_name)
}
