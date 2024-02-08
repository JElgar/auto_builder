use darling::{FromDeriveInput, FromField, FromMeta};
use proc_macro::TokenStream;
use quote::quote;
use syn::{self, parse_macro_input, DeriveInput, Ident};

#[derive(Debug, FromDeriveInput)]
#[darling(supports(struct_named))]
struct StructReceiver {
    ident: Ident,
    data: darling::ast::Data<(), FieldReceiver>,
}

#[derive(Debug, FromField)]
#[darling(attributes(auto_builder))]
struct FieldReceiver {
    ident: Option<Ident>,
    ty: syn::Type,

    // TODO Support method
    // See: https://github.com/serde-rs/serde/blob/846f865de2e94408e0edc6a2c6863c063cd234be/serde_derive/src/internals/attr.rs#L397
    //    : https://serde.rs/attr-default.html

    // #[darling(rename = "default")]
    default: Option<bool>,
}

struct StructInfo {
    name: Ident,
    builder_name: Ident,
    fields: Vec<FieldInfo>,
}

struct FieldInfo {
    name: Ident,
    ty: syn::Type,
    generic: syn::GenericParam,
    default: bool,
}

impl From<FieldReceiver> for FieldInfo {
    fn from(field: FieldReceiver) -> Self {
        let name = field
            .ident
            .expect("Only named fields are supported - enforced by darling");
        let generic_name = Ident::from_string(format!("T{}", name).as_str()).unwrap();

        FieldInfo {
            name,
            ty: field.ty,
            generic: syn::GenericParam::Type(syn::TypeParam {
                ident: generic_name.clone(),
                attrs: vec![],
                colon_token: None,
                bounds: syn::punctuated::Punctuated::new(),
                eq_token: None,
                default: None,
            }),
            default: matches!(field.default, Some(true)),
        }
    }
}

impl From<StructReceiver> for StructInfo {
    fn from(struct_receiver: StructReceiver) -> Self {
        let fields = struct_receiver
            .data
            .take_struct()
            .expect("Only named structs are supported - enforced by darling")
            .fields;

        StructInfo {
            name: struct_receiver.ident.clone(),
            builder_name: Ident::from_string(format!("{}Builder", struct_receiver.ident).as_str())
                .unwrap(),
            fields: fields.into_iter().map(Into::into).collect(),
        }
    }
}

impl FieldInfo {
    fn is_option(&self) -> bool {
        matches!(&self.ty, syn::Type::Path(syn::TypePath {
                    path: syn::Path { segments, .. },
                    ..
            }) if segments.len() == 1 && segments.first().unwrap().ident == "Option"
        )
    }

    fn new_generic(&self) -> proc_macro2::TokenStream {
        match self.is_option() || self.default {
            true => {
                let ty = &self.ty;
                quote! { #ty }
            }
            false => quote! { auto_builder::NoValue },
        }
    }
}

#[proc_macro_derive(Builder, attributes(auto_builder))]
pub fn builder(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let struct_info = StructReceiver::from_derive_input(&input);
    match struct_info {
        Err(e) => TokenStream::from(e.write_errors()),
        Ok(struct_info) => impl_builder(struct_info.into()).into(),
    }
}

fn impl_builder(struct_info: StructInfo) -> proc_macro2::TokenStream {
    let builder_struct = impl_builder_struct(&struct_info);
    let new_impl = impl_new_method(&struct_info);
    let build_impl = impl_build_method(&struct_info);
    let builder_method_impl = impl_builder_method(&struct_info);
    let setters_impl = impl_setter_methods(&struct_info);

    quote! {
        #builder_struct
        #new_impl
        #build_impl
        #builder_method_impl
        #setters_impl
    }
}

fn impl_builder_struct(struct_info: &StructInfo) -> proc_macro2::TokenStream {
    let builder_name = &struct_info.builder_name;
    let generics = struct_info.fields.iter().map(|f| &f.generic);

    let builder_struct_attributes = struct_info.fields.iter().map(|field| {
        let field_name = &field.name;
        let generic = &field.generic;
        quote! {
            #field_name: #generic
        }
    });

    quote! {
        struct #builder_name<#(#generics),*> {
            #(#builder_struct_attributes),*
        }
    }
}

fn impl_new_method(struct_info: &StructInfo) -> proc_macro2::TokenStream {
    let builder_name = &struct_info.builder_name;

    let all_unit_generics = struct_info.fields.iter().map(|_| {
        quote! {
            ()
        }
    });

    let new_builder_generics = struct_info.fields.iter().map(|field| field.new_generic());

    let new_impl = struct_info.fields.iter().map(|field| {
        let field_name = &field.name;
        let field_type = &field.ty;
        let field_value = if field.is_option() {
            quote! { None }
        } else if field.default {
            quote! { #field_type::default() }
        } else {
            quote! { auto_builder::NoValue }
        };

        quote! {
            #field_name: #field_value
        }
    });

    quote! {
        impl #builder_name<#(#all_unit_generics),*> {
            fn new() -> #builder_name<#(#new_builder_generics),*> {
                #builder_name {
                    #(#new_impl),*
                }
            }
        }
    }
}

fn impl_builder_method(struct_info: &StructInfo) -> proc_macro2::TokenStream {
    let struct_name = &struct_info.name;
    let builder_name = &struct_info.builder_name;
    let new_builder_generics = struct_info.fields.iter().map(|f| f.new_generic());

    quote! {
        impl #struct_name {
            fn builder() -> #builder_name<#(#new_builder_generics),*> {
                #builder_name::new()
            }
        }
    }
}

fn impl_build_method(struct_info: &StructInfo) -> proc_macro2::TokenStream {
    let struct_name = &struct_info.name;
    let builder_name = &struct_info.builder_name;

    // After build is complete the generics match the field types
    let built_generics = struct_info.fields.iter().map(|f| &f.ty);

    let build_attrs = struct_info.fields.iter().map(|field| {
        let field_name = &field.name;

        quote! {
            #field_name: self.#field_name
        }
    });

    quote! {
        impl #builder_name<#(#built_generics),*> {
            fn build(self) -> #struct_name {
                #struct_name {
                    #(#build_attrs),*
                }
            }
        }
    }
}

fn impl_setter_methods(struct_info: &StructInfo) -> proc_macro2::TokenStream {
    let builder_name = &struct_info.builder_name;
    let generics = struct_info.fields.iter().map(|f| &f.generic);
    let methods = struct_info
        .fields
        .iter()
        .map(|field| impl_attribute_setter(field, struct_info));

    let generics = quote! { #(#generics),* };

    quote! {
        impl <#generics> #builder_name<#generics> {
            #(#methods)*
        }
    }
}

fn impl_attribute_setter(
    setter_field: &FieldInfo,
    struct_info: &StructInfo,
) -> proc_macro2::TokenStream {
    let assigments = struct_info.fields.iter().map(|field| {
        let field_name = &field.name;
        if setter_field.name == field.name {
            quote! {
                #field_name: input.into()
            }
        } else {
            quote! {
                #field_name: self.#field_name
            }
        }
    });

    let updated_generics = struct_info.fields.iter().map(|field| {
        if field.name == setter_field.name {
            let ty = &field.ty;
            quote!(#ty)
        } else {
            let ty = &field.generic;
            quote!(#ty)
        }
    });

    let method_name = Ident::from_string(format!("set_{}", setter_field.name).as_str()).unwrap();
    let builder_name = &struct_info.builder_name;
    let field_type = &setter_field.ty;

    quote! {
        pub fn #method_name(self, input: impl Into<#field_type>) -> #builder_name<#(#updated_generics),*> {
            #builder_name {
                #(#assigments),*
            }
        }
    }
}
