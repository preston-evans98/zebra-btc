use proc_macro::TokenStream;
use quote::quote;
use syn::{self};

pub fn impl_ser_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = ast.ident.clone();
    match ast.data {
        syn::Data::Struct(ref data) => {
            let statements: Vec<quote::__private::TokenStream> = data
                .fields
                .iter()
                .zip(0..1_000_000)
                .map(|(field, index)| serialize_field(field, index))
                .collect(); //.map(|field| &field.ty),

            let expanded = quote! {
                impl BitcoinSerialize for #name {
                    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
                    where
                        W: std::io::Write,
                    {
                        #(#statements)*
                        Ok(())
                    }
                }
            };
            return TokenStream::from(expanded);
        }
        syn::Data::Enum(ref data) => {
            let variants: Vec<quote::__private::TokenStream> = data
                .variants
                .iter()
                .map(|variant| serialize_variant(variant, &name))
                .collect();
            // vec![quoted]

            let expanded: quote::__private::TokenStream = quote! {
                impl BitcoinSerialize for #name {
                    fn bitcoin_serialize<W>(&self, mut target: W) -> Result<(), std::io::Error>
                    where
                        W: std::io::Write,
                    {
                        match *self {
                            #(#variants)*
                        }
                        Ok(())

                    }
                }
            };
            return TokenStream::from(expanded);
        }
        _ => unimplemented!(),
    }
}

fn serialize_field(field: &syn::Field, index: usize) -> quote::__private::TokenStream {
    match field.ident.clone() {
        Some(id) => quote! { self.#id.bitcoin_serialize(&mut target)?; },
        None => {
            let index = syn::Index::from(index);
            quote! {self.#index.bitcoin_serialize(&mut target)?;}
        } // None => Ident::new(&index.to_string(), Span::call_site()),
    }
}

// fn serialize_ref(field: &syn::Field) -> quote::__private::TokenStream {
//     let ident = field
//         .ident
//         .clone()
//         .expect("Can only serialize named fields");
//     quote! { #ident.serialize(target)?; }
// }

fn serialize_variant(variant: &syn::Variant, name: &syn::Ident) -> quote::__private::TokenStream {
    let ident = variant.ident.clone();

    let subfields: Vec<quote::__private::TokenStream> = variant
        .fields
        .iter()
        .map(|field| {
            if let Some(ident) = field.ident.clone() {
                quote! { ref #ident , }
            } else {
                quote!(ref inner)
            }
        })
        .collect();

    let statements: Vec<quote::__private::TokenStream> = variant
        .fields
        .iter()
        .map(|field| {
            if let Some(ident) = field.ident.clone() {
                quote! { #ident.bitcoin_serialize(&mut target)?; }
            } else {
                quote! { inner.bitcoin_serialize(&mut target)?;}
            }
        })
        .collect();

    if subfields.len() > 0 {
        quote! { #name::#ident ( #(#subfields)* ) => {
            #(#statements)*
        },}
    } else {
        quote! { #name::#ident => {
            #(#statements)*
        },}
    }
}
