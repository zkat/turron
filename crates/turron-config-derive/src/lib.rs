use darling::{ast, FromDeriveInput, FromField, ToTokens};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Lit, LitStr};

#[proc_macro_derive(TurronConfigLayer, attributes(turron_config))]
pub fn derive_turron_command(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let cmd = TurronConfigLayer::from_derive_input(&input).unwrap();
    quote!(#cmd).into()
}

#[derive(Debug, FromDeriveInput)]
#[darling(supports(struct_named))]
struct TurronConfigLayer {
    ident: syn::Ident,
    generics: syn::Generics,
    data: ast::Data<(), TurronCommandField>,
}

#[derive(Debug, FromField)]
#[darling(forward_attrs)]
struct TurronCommandField {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    attrs: Vec<syn::Attribute>,
}

fn inner_type_of_option(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(syn::TypePath { path, .. }) = ty {
        if let Some(p) = path.segments.iter().next() {
            // TODO: could be extended to support `Vec` too?
            if p.ident != "Option" {
                return None;
            }

            if let syn::PathArguments::AngleBracketed(ab) = &p.arguments {
                if let Some(syn::GenericArgument::Type(t)) = ab.args.first() {
                    return Some(t);
                }
            }
        }
    }
    None
}

fn turron_ignored(attr: &syn::Attribute) -> bool {
    if let Ok(syn::Meta::List(meta_list)) = attr.parse_meta() {
        if meta_list.path.get_ident().unwrap() == "turron_config" {
            if let Some(syn::NestedMeta::Meta(syn::Meta::Path(p))) = meta_list.nested.first() {
                return p.get_ident().unwrap() == "ignore";
            }
        }
        return meta_list.path.get_ident().unwrap() == "clap"
            && !meta_list.nested.iter().any(|n| match n {
                syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) => {
                    let path = nv.path.get_ident().unwrap();
                    path == "long" || path == "short"
                }
                _ => false,
            });
    }
    false
}

fn should_be_ignored(field: &TurronCommandField) -> bool {
    field.attrs.iter().any(|attr| turron_ignored(attr))
}

impl ToTokens for TurronConfigLayer {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let TurronConfigLayer {
            ref data,
            ref ident,
            ..
        } = *self;
        let fields = data
            .as_ref()
            .take_struct()
            .expect(
                "Enums not supported by derive macro. Implement TurronCommandLayerConfig manually.",
            )
            .fields;
        let field_defs = fields
            .clone()
            .into_iter()
            .filter(|field| !should_be_ignored(field))
            .map(|field| {
                let TurronCommandField { ident, ty, .. } = field;
                let ident = ident.clone().unwrap();
                let lit_str = Lit::Str(LitStr::new(&ident.to_string(), ident.span()));

                if let Some(inner) = inner_type_of_option(ty) {
                    quote! {
                        if args.occurrences_of(#lit_str) == 0 {
                            if let Ok(val) = config.get_str(#lit_str) {
                                self.#ident = #inner::from_str(&val).ok();
                            }
                        }
                    }
                } else {
                    quote! {
                        if args.occurrences_of(#lit_str) == 0 {
                            if let Ok(val) = config.get_str(#lit_str) {
                                self.#ident = #ty::from_str(&val).map_err(|e| TurronConfigError::ConfigParseError(Box::new(e)))?;
                            }
                        }
                    }
                }
            });

        let ts = quote! {
            mod turron_command_layer_config {
                use super::*;

                use std::str::FromStr;

                impl turron_config::TurronConfigLayer for #ident {
                    fn layer_config(&mut self, args: &clap::ArgMatches, config: &turron_config::TurronConfig) -> turron_common::miette::Result<()> {
                        #(#field_defs)*
                        Ok(())
                    }
                }
            }
        };
        tokens.extend(ts);
    }
}
