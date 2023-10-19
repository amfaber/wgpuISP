use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};


#[proc_macro_derive(PlaygroundUi)]
pub fn playground_ui(input: TokenStream) -> TokenStream{
    let input = parse_macro_input!(input as DeriveInput);

    let struct_data = match input.data{
        syn::Data::Struct(_struct) => _struct,
        _ => panic!("Only structs are supported for this derive"),
    };

    for field in &struct_data.fields{
        // field.;
        for attr in &field.attrs{
            if !attr.path().is_ident("playground"){
                continue;
            }
            let meta_list =  match &attr.meta{
                syn::Meta::List(meta_list) => meta_list,
                _ => panic!("The attribute must be of the form #[playground(name1 = value1, name2 = value2)]"),
            };
        }
    }
    
    todo!()
}
