use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, Fields, Meta, Type,
};

#[proc_macro_derive(ProcKeyValue)]
pub fn proc_key_value_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_gen, ty_gen, where_gen) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("ProcKeyValue only supports structs with named fields"),
        },
        _ => panic!("ProcKeyValue only supports structs"),
    };

    let mut field_parsers = Vec::new();
    let mut field_defaults = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().expect("named field");
        let field_type = &field.ty;

        let proc_key = extract_proc_key_attr(&field.attrs).unwrap_or_else(|| {
            panic!(
                "Field {} must have #[proc_key = \"...\"] attribute",
                field_name
            )
        });

        let parse_expr = generate_parse_expr(field_type);

        field_parsers.push(quote! {
            #proc_key => {
                __instance.#field_name = Some(#parse_expr);
            }
        });

        field_defaults.push(quote! {
            #field_name: None
        });
    }

    let expanded = quote! {
        impl #impl_gen procfs2::util::parse::ParseFromBytes for #name #ty_gen #where_gen {
            fn from_bytes(bytes: &[u8]) -> procfs2::error::Result<Self> {
                use procfs2::util::parse::{trim_start, parse_key_value_line};
                use procfs2::types::Kibibytes;
                use procfs2::types::Bytes;

                let mut __instance = #name {
                    #(#field_defaults)*
                };

                for line in bytes.split(|&b| b == b'\n') {
                    if line.is_empty() {
                        continue;
                    }

                    let (key, value) = match parse_key_value_line(line) {
                        Some(kv) => kv,
                        None => continue,
                    };

                    let key_str = unsafe {
                        std::str::from_utf8_unchecked(key)
                    };
                    match key_str {
                        #(#field_parsers)*
                        _ => {}
                    }
                }

                Ok(__instance)
            }
        }
    };

    expanded.into()
}

fn extract_proc_key_attr(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if let Meta::NameValue(nv) = &attr.meta {
            if nv.path.is_ident("proc_key") {
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(lit_str), .. }) = &nv.value {
                    return Some(lit_str.value());
                }
            }
        }
    }
    None
}

fn generate_parse_expr(field_type: &Type) -> proc_macro2::TokenStream {
    if let Type::Path(type_path) = field_type {
        if let Some(segment) = type_path.path.segments.last() {
            match segment.ident.to_string().as_str() {
                "Kibibytes" => {
                    quote! {
                        if let Ok(s) = std::str::from_utf8(trim_start(value)) {
                            let num = s.split(|c: char| c == ' ').next().unwrap_or(s);
                            Kibibytes(
                                procfs2::util::parse::parse_dec_u64(num.as_bytes())
                                    .unwrap_or(0)
                            )
                        } else {
                            Kibibytes(0)
                        }
                    }
                }
                "Bytes" => {
                    quote! {
                        if let Ok(s) = std::str::from_utf8(trim_start(value)) {
                            Bytes(
                                procfs2::util::parse::parse_dec_u64(s.as_bytes())
                                    .unwrap_or(0)
                            )
                        } else {
                            Bytes(0)
                        }
                    }
                }
                "Option" => {
                    if let Some(inner) = extract_option_inner(field_type) {
                        let inner_expr = generate_parse_expr(&inner);
                        quote! { Some(#inner_expr) }
                    } else {
                        quote! { None }
                    }
                }
                "u32" => {
                    quote! {
                        procfs2::util::parse::parse_dec_u32(trim_start(value)).unwrap_or(0)
                    }
                }
                "u64" => {
                    quote! {
                        procfs2::util::parse::parse_dec_u64(trim_start(value)).unwrap_or(0)
                    }
                }
                "i32" => {
                    quote! {
                        procfs2::util::parse::parse_dec_i32(trim_start(value)).unwrap_or(0)
                    }
                }
                "i64" => {
                    quote! {
                        procfs2::util::parse::parse_dec_i64(trim_start(value)).unwrap_or(0)
                    }
                }
                "f32" => {
                    quote! {
                        procfs2::util::parse::parse_dec_f32(trim_start(value)).unwrap_or(0.0)
                    }
                }
                "f64" => {
                    quote! {
                        procfs2::util::parse::parse_dec_f64(trim_start(value)).unwrap_or(0.0)
                    }
                }
                _ => {
                    quote! { Default::default() }
                }
            }
        } else {
            quote! { Default::default() }
        }
    } else {
        quote! { Default::default() }
    }
}

fn extract_option_inner(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(ref args) = segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner.clone());
                    }
                }
            }
        }
    }
    None
}
