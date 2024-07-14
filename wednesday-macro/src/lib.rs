extern crate proc_macro;

use convert_case::{Boundary, Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(DeExchange)]
pub fn de_exchange_derive(input: TokenStream) -> TokenStream {
    // Parse Rust code abstract syntax tree with Syn from TokenStream -> DeriveInput
    let ast: DeriveInput = syn::parse(input).expect("de_exchange_derive() failed to parse input TokenStream");

    // Determine exchange name
    let exchange = &ast.ident;

    let generated = quote! {
        impl<'de> serde::Deserialize<'de> for #exchange {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::de::Deserializer<'de>
            {
                let input = <String as serde::Deserialize>::deserialize(deserializer)?;
                let expected = #exchange::ID.as_str();

                if input.as_str() == expected {
                    Ok(Self::default())
                } else {
                    Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(input.as_str()),
                        &expected
                    ))
                }
            }
        }
    };

    TokenStream::from(generated)
}

#[proc_macro_derive(SerExchange)]
pub fn ser_exchange_derive(input: TokenStream) -> TokenStream {
    // Parse Rust code abstract syntax tree with Syn from TokenStream -> DeriveInput
    let ast: DeriveInput = syn::parse(input).expect("ser_exchange_derive() failed to parse input TokenStream");

    // Determine Exchange
    let exchange = &ast.ident;

    let generated = quote! {
        impl serde::Serialize for #exchange {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::ser::Serializer,
            {
                let exchange_id = #exchange::ID.as_str();
                serializer.serialize_str(exchange_id)
            }
        }
    };

    TokenStream::from(generated)
}

#[proc_macro_derive(DeSubscriptionKind)]
pub fn de_subscription_kind_derive(input: TokenStream) -> TokenStream {
    // Parse Rust code abstract syntax tree with Syn from TokenStream -> DeriveInput
    let ast: DeriveInput = syn::parse(input).expect("de_sub_kind_derive() failed to parse input TokenStream");

    // Determine SubKind name
    let sub_kind = &ast.ident;

    let expected_sub_kind = sub_kind
        .to_string()
        .from_case(Case::Pascal)
        .without_boundaries(&Boundary::letter_digit())
        .to_case(Case::Snake);

    let generated = quote! {
        impl<'de> serde::Deserialize<'de> for #sub_kind {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::de::Deserializer<'de>
            {
                let input = <String as serde::Deserialize>::deserialize(deserializer)?;

                if input == #expected_sub_kind {
                    Ok(Self)
                } else {
                    Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(input.as_str()),
                        &#expected_sub_kind
                    ))
                }
            }
        }
    };

    TokenStream::from(generated)
}

#[proc_macro_derive(SerSubscriptionKind)]
pub fn ser_subscription_kind_derive(input: TokenStream) -> TokenStream {
    // Parse Rust code abstract syntax tree with Syn from TokenStream -> DeriveInput
    let ast: DeriveInput = syn::parse(input).expect("ser_sub_kind_derive() failed to parse input TokenStream");

    // Determine SubKind name
    let sub_kind = &ast.ident;
    let sub_kind_string = sub_kind.to_string().to_case(Case::Snake);
    let sub_kind_str = sub_kind_string.as_str();

    let generated = quote! {
        impl serde::Serialize for #sub_kind {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::ser::Serializer,
            {
                serializer.serialize_str(#sub_kind_str)
            }
        }
    };

    TokenStream::from(generated)
}

// #[proc_macro_derive(AsUrlParams)]
// pub fn derive_as_string(input: TokenStream) -> TokenStream {
//     // Parse the input tokens into a syntax tree
//     let input = parse_macro_input!(input as DeriveInput);

//     // Get the struct name
//     let name = &input.ident;

//     // Extract the field names in the order they are declared
//     let fields = if let syn::Data::Struct(data) = &input.data {
//         data.fields.iter().map(|f| {
//             f.ident.as_ref().unwrap().to_string()
//         }).collect::<Vec<_>>()
//     } else {
//         panic!("AsUrlParams can only be derived for structs");
//     };

//     // Generate the implementation
//     let expanded = quote! {
//         impl #name {
//             pub fn to_url_params(&self) -> String {
//                 let mut params = Vec::new();

//                 // Use serde_json to convert struct fields to key-value pairs
//                 let json_string = serde_json::to_string(&self).expect("Failed to serialize struct");
//                 let json_value: serde_json::Value = serde_json::from_str(&json_string).expect("Failed to parse JSON");

//                 if let serde_json::Value::Object(map) = json_value {
//                     let ordered_keys = vec![#(#fields),*];
//                     for key in ordered_keys {
//                         if let Some(value) = map.get(&key) {
//                             if let serde_json::Value::Null = value {
//                                 continue; // Skip null values
//                             }
//                             // Remove quotes from string values
//                             let value_str = match value {
//                                 serde_json::Value::String(s) => s.clone(),
//                                 _ => value.to_string(),
//                             };
//                             params.push(format!("{}={}", key, value_str));
//                         }
//                     }
//                 }

//                 params.join("&")
//             }
//         }
//     };

//     // Convert the expanded code into a TokenStream and return it
//     TokenStream::from(expanded)
// }

// use proc_macro::TokenStream;
// use quote::quote;
// use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(AsUrlParams)]
pub fn derive_as_string(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let fields = if let syn::Data::Struct(data) = &input.data {
        data.fields
            .iter()
            .filter_map(|f| f.ident.as_ref().map(|ident| ident.to_string()))
            .collect::<Vec<_>>()
    } else {
        panic!("AsUrlParams can only be derived for structs");
    };

    let expanded = quote! {
        impl #name {
            pub fn to_url_params(&self) -> String {
                let mut params = Vec::new();

                let json_value = serde_json::json!(self);
                if let serde_json::Value::Object(map) = json_value {
                    let ordered_keys = vec![#(#fields),*];
                    for key in ordered_keys {
                        if let Some(value) = map.get(&*key) {
                            if !value.is_null() {
                                let value_str = match value {
                                    serde_json::Value::String(s) => s.to_string(),
                                    _ => value.to_string(),
                                };
                                params.push(format!("{}={}", key, value_str));
                            }
                        }
                    }
                }

                params.join("&")
            }
        }
    };

    TokenStream::from(expanded)
}
