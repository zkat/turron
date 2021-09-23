use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

#[derive(Debug)]
pub struct TurronConfigLayer {
    generics: syn::Generics,
    ident: syn::Ident,
    command: String,
    fields: Vec<ConfigField>,
}

#[derive(Debug)]
struct ConfigField {
    member: syn::Member,
    field_type: ConfigFieldType,
}

#[derive(Debug)]
enum ConfigFieldType {
    OptionOption,
    OptionVec,
    Option,
    Plain,
    Vec,
}

impl ConfigField {
    fn from_field(i: usize, field: syn::Field) -> Result<Option<Self>, syn::Error> {
        if let Some(attr) = field.attrs.iter().find(|attr| attr.path.is_ident("clap")) {
            let meta = attr.parse_meta()?;
            if let syn::Meta::List(list) = meta {
                if list.nested.iter().any(|x| {
                    if let syn::NestedMeta::Meta(syn::Meta::NameValue(name_value)) = x {
                        let p = &name_value.path;
                        p.is_ident("from_global") || p.is_ident("long") || p.is_ident("short")
                    } else if let syn::NestedMeta::Meta(syn::Meta::Path(path)) = x {
                        let p = path;
                        p.is_ident("from_global") || p.is_ident("long") || p.is_ident("short")
                    } else if let syn::NestedMeta::Meta(syn::Meta::List(list)) = x {
                        let p = &list.path;
                        p.is_ident("from_global") || p.is_ident("long") || p.is_ident("short")
                    } else {
                        false
                    }
                }) {
                    // TODO
                    let ty = &field.ty;
                    let member = if let Some(ident) = field.ident.clone() {
                        syn::Member::Named(ident)
                    } else {
                        syn::Member::Unnamed(syn::Index {
                            index: i as u32,
                            span: field.span(),
                        })
                    };
                    if is_generic_ty(ty, "Vec") {
                        return Ok(Some(ConfigField {
                            member,
                            field_type: ConfigFieldType::Vec,
                        }));
                    } else if let Some(subty) = subty_if_name(ty, "Option") {
                        if is_generic_ty(subty, "Option") {
                            return Ok(Some(ConfigField {
                                member,
                                field_type: ConfigFieldType::OptionOption,
                            }));
                        } else if is_generic_ty(subty, "Vec") {
                            return Ok(Some(ConfigField {
                                member,
                                field_type: ConfigFieldType::OptionVec,
                            }));
                        } else {
                            return Ok(Some(ConfigField {
                                member,
                                field_type: ConfigFieldType::Option,
                            }));
                        }
                    } else {
                        return Ok(Some(ConfigField {
                            member,
                            field_type: ConfigFieldType::Plain,
                        }));
                    }
                }
            }
            Ok(None)
        } else {
            Ok(None)
        }
    }
}

impl TurronConfigLayer {
    pub fn from_derive_input(input: syn::DeriveInput) -> Result<Self, syn::Error> {
        match input.data {
            syn::Data::Struct(data_struct) => {
                let span = input.ident.span();
                let cmd = input
                    .attrs
                    .iter()
                    .find(|x| x.path.is_ident("config_layer"))
                    .map(|attr| -> Result<String, syn::Error> {
                        let meta = attr.parse_meta()?;
                        if let syn::Meta::NameValue(syn::MetaNameValue { lit: syn::Lit::Str(lit_str), .. }) = meta {
                            Ok(lit_str.value())
                        } else {
                            Err(syn::Error::new(
                               attr.span(),
                               "`#[config_layer]` attribute must be a literal string assignment, such as `#[config_layer = \"my.command\"]`."
                            ))
                        }
                    })
                    .transpose()?
                    .ok_or_else(move || {
                        syn::Error::new(
                            span,
                            "#[config_layer = \"my.command\")] attribute is required for config layering.",
                        )
                    })?;
                Ok(TurronConfigLayer {
                    command: cmd,
                    fields: data_struct
                        .fields
                        .into_iter()
                        .enumerate()
                        .map(|(i, field)| ConfigField::from_field(i, field))
                        .filter_map(|x| x.transpose())
                        .collect::<Result<Vec<_>, syn::Error>>()?,
                    ident: input.ident,
                    generics: input.generics,
                })
            }
            syn::Data::Enum(_) => Err(syn::Error::new(
                input.ident.span(),
                "Can't derive TurronConfigLayer for Enums",
            )),
            syn::Data::Union(_) => Err(syn::Error::new(
                input.ident.span(),
                "Can't derive TurronConfigLayer for Unions",
            )),
        }
    }

    pub fn gen(&self) -> TokenStream {
        let ident = &self.ident;
        let generics = &self.generics;
        quote! {
            impl turron_command::turron_config::TurronConfigLayer for #ident #generics {
                fn layer_config(
                    &mut self,
                    matches: &turron_command::turron_config::ArgMatches,
                    config: &turron_command::turron_config::TurronConfig,
                ) -> turron_common::miette::Result<()> {
                    Ok(())
                }
            }
        }
    }
}

// Below is taken from structopt
fn only_last_segment(ty: &syn::Type) -> Option<&syn::PathSegment> {
    match ty {
        syn::Type::Path(syn::TypePath {
            qself: None,
            path:
                syn::Path {
                    leading_colon: None,
                    segments,
                },
        }) => only_one(segments.iter()),

        _ => None,
    }
}

fn subty_if<F>(ty: &syn::Type, f: F) -> Option<&syn::Type>
where
    F: FnOnce(&syn::PathSegment) -> bool,
{
    let ty = strip_group(ty);

    only_last_segment(ty)
        .filter(|segment| f(segment))
        .and_then(|segment| {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                only_one(args.args.iter()).and_then(|generic| {
                    if let syn::GenericArgument::Type(ty) = generic {
                        Some(ty)
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        })
}

fn subty_if_name<'a>(ty: &'a syn::Type, name: &str) -> Option<&'a syn::Type> {
    subty_if(ty, |seg| seg.ident == name)
}

fn strip_group(mut ty: &syn::Type) -> &syn::Type {
    while let syn::Type::Group(group) = ty {
        ty = &*group.elem;
    }

    ty
}

fn is_generic_ty(ty: &syn::Type, name: &str) -> bool {
    subty_if_name(ty, name).is_some()
}

fn only_one<I, T>(mut iter: I) -> Option<T>
where
    I: Iterator<Item = T>,
{
    iter.next().filter(|_| iter.next().is_none())
}
