use proc_macro::TokenStream;
use quote::quote;
use syn;
pub fn impl_deser_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = ast.ident.clone();
    let is_tuple_struct = match ast.data {
        syn::Data::Struct(ref data) => match data.fields {
            syn::Fields::Unnamed(_) => true,
            _ => false,
        },
        _ => false,
    };
    let statements: Vec<quote::__private::TokenStream> = match ast.data {
        // syn::Data::Struct(ref data) => &data.fields, //.map(|field| &field.ty),
        syn::Data::Struct(ref data) => data
            .fields
            .iter()
            .zip(0..1_000_000)
            .map(|(field, index)| deserialize_field(field, index))
            .collect(), //.map(|field| &field.ty),
        syn::Data::Enum(ref data) => {
            let variants: Vec<quote::__private::TokenStream> = data
                .variants
                .iter()
                .map(|variant| deserialize_variant(variant, &name))
                .collect();

            let expanded: quote::__private::TokenStream = quote! {
                impl BitcoinDeserialize for #name {
                    fn bitcoin_deserialize<R: std::io::Read>(&self, mut target: R) -> Result<#name, std::io::Error>
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
    };
    let expanded = if is_tuple_struct {
        quote! {
            impl BitcoinDeserialize for #name {
                fn bitcoin_deserialize<R: std::io::Read>(mut target: R) -> Result<Self, SerializationError>
                {
                    Ok(#name (
                        #(#statements)*
                    ))
                }
            }
        }
    } else {
        quote! {
            impl BitcoinDeserialize for #name {
                fn bitcoin_deserialize<R: std::io::Read>(mut target: R) -> Result<Self, SerializationError>
                {
                    Ok(#name {
                        #(#statements)*
                    })
                }
            }
        }
    };

    TokenStream::from(expanded)
}

fn deserialize_field(field: &syn::Field, _index: usize) -> quote::__private::TokenStream {
    let ty = field.ty.clone();
    match field.ident.clone() {
        Some(name) => {
            quote! { #name: <#ty as BitcoinDeserialize>::bitcoin_deserialize(&mut target)?, }
        }
        None => {
            quote! {<#ty>::bitcoin_deserialize(&mut target)?,}
        }
    }

    // let name = field.ident.clone().expect("Missing identifier for field");
    // let ty = field.ty.clone();

    // quote! { #name: 0, }
    // quote! { #name: format!("shared::<{}>::deserialize(target),", #ty)  }
}

fn deserialize_variant(variant: &syn::Variant, name: &syn::Ident) -> quote::__private::TokenStream {
    let ident = variant.ident.clone();

    // let subfields: Vec<quote::__private::TokenStream> = variant
    //     .fields
    //     .iter()
    //     .map(|field| {
    //         let ident = field
    //             .ident
    //             .clone()
    //             .expect("Can only derive serialize for named variant fields");
    //         quote! { ref #ident , }
    //     })
    //     .collect();

    let statements: Vec<quote::__private::TokenStream> = variant
        .fields
        .iter()
        .map(|field| {
            let ty = field.ty.clone();
            // let ident = field
            //     .ident
            //     .clone()
            //     .expect("Can only derive serialize for named variant fields");
            quote! { #ty::bitcoin_deserialize(&mut target)?; }
        })
        .collect();

    quote! { #name::#ident {
        #(#statements)*
    } }
}
