use std::hash::{Hash, Hasher};

//type QuoteRes = quote::__private::TokenStream;
use lgn_utils::DefaultHasher;
use proc_macro2::{TokenStream, TokenTree};
use quote::{quote, ToTokens};

const LEGION_TAG: &str = "legion";
const DEFAULT_ATTR: &str = "default";
const HIDDEN_ATTR: &str = "hidden";
const OFFLINE_ATTR: &str = "offline";
const TOOLTIP_ATTR: &str = "tooltip";
const READONLY_ATTR: &str = "readonly";
const GROUP_ATTR: &str = "group";
const TRANSIENT_ATTR: &str = "transient";
const RESOURCE_TYPE_ATTR: &str = "resource_type";

pub struct DataContainerMetaInfo {
    pub name: String,
    pub need_life_time: bool,
    pub members: Vec<MemberMetaInfo>,
    pub is_resource: bool,
    pub is_component: bool,
}

#[allow(clippy::struct_excessive_bools)]
pub struct MemberMetaInfo {
    pub name: String,
    pub type_path: syn::Path,
    pub resource_type: Option<syn::Path>,
    pub imports: Vec<syn::Path>,
    pub offline: bool,
    pub group: String,
    pub hidden: bool,
    pub readonly: bool,
    pub transient: bool,
    pub tooltip: String,
    pub default_literal: Option<TokenStream>,
}

impl DataContainerMetaInfo {
    #[allow(clippy::unused_self)]
    pub fn need_life_time(&self) -> bool {
        false
        // TODO: Add proper support for life_time with inplace deserialization
        //self.members.iter().any(|a| a.type_name == "String")
    }

    pub fn imports(&self) -> Vec<syn::Path> {
        let mut output = vec![];
        for member in &self.members {
            for import in &member.imports {
                if !output.contains(import) {
                    output.push(import.clone());
                }
            }
        }
        output
    }

    pub fn calculate_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.name.hash(&mut hasher);
        self.members.iter().for_each(|m| {
            m.name.hash(&mut hasher);
            m.get_type_name().hash(&mut hasher);
            if let Some(res) = m.resource_type.as_ref() {
                res.hash(&mut hasher);
            }
            m.offline.hash(&mut hasher);
            m.transient.hash(&mut hasher);
            m.default_literal
                .to_token_stream()
                .to_string()
                .hash(&mut hasher);
        });

        hasher.finish()
    }
}

pub fn get_data_container_info(
    item_struct: &syn::ItemStruct,
) -> Result<DataContainerMetaInfo, String> {
    let mut data_container_meta_info = DataContainerMetaInfo {
        name: item_struct.ident.to_string(),
        need_life_time: false,
        is_resource: item_struct
            .attrs
            .iter()
            .any(|attr| attr.path.segments.len() == 1 && attr.path.segments[0].ident == "resource"),
        is_component: item_struct.attrs.iter().any(|attr| {
            attr.path.segments.len() == 1 && attr.path.segments[0].ident == "component"
        }),
        members: Vec::new(),
    };

    if let syn::Fields::Named(named_fields) = &item_struct.fields {
        for field in &named_fields.named {
            if let syn::Type::Path(type_path) = &field.ty {
                data_container_meta_info
                    .members
                    .push(get_member_info(field, type_path.path.clone()));
            } else {
                let str = format!(
                    "Legion: unsupported field type: {}",
                    field.ident.as_ref().unwrap().to_string()
                );
                return Err(str);
            }
        }
    }
    Ok(data_container_meta_info)
}

impl MemberMetaInfo {
    pub fn is_option(&self) -> bool {
        !self.type_path.segments.is_empty() && self.type_path.segments[0].ident == "Option"
    }

    pub fn is_vec(&self) -> bool {
        !self.type_path.segments.is_empty() && self.type_path.segments[0].ident == "Vec"
    }

    pub fn get_type_name(&self) -> String {
        self.type_path.to_token_stream().to_string()
    }

    pub fn get_runtime_type(&self) -> Option<syn::Path> {
        match self.get_type_name().as_str() {
            "Option < ResourcePathId >" => {
                let ty = if let Some(resource_type) = &self.resource_type {
                    format!(
                        "Option<lgn_data_runtime::Reference<{}>>",
                        resource_type.to_token_stream().to_string()
                    )
                } else {
                    "Option<lgn_data_runtime::Reference<lgn_data_runtime::Resource>>".into()
                };
                syn::parse_str(ty.as_str()).ok()
            }
            "Vec < ResourcePathId >" => {
                let ty = if let Some(resource_type) = &self.resource_type {
                    format!(
                        "Vec<lgn_data_runtime::Reference<{}>>",
                        resource_type.to_token_stream().to_string()
                    )
                } else {
                    "Vec<lgn_data_runtime::Reference<lgn_data_runtime::Resource>>".into()
                };
                syn::parse_str(ty.as_str()).ok()
            }
            _ => Some(self.type_path.clone()), // Keep same
        }
    }

    pub fn _clone_on_compile(&self) -> bool {
        !self.type_path.segments.is_empty() && self.type_path.segments[0].ident == "Vec"
    }
}

fn get_attribute_literal(
    group_iter: &mut std::iter::Peekable<proc_macro2::token_stream::IntoIter>,
) -> String {
    if let Some(TokenTree::Punct(punct)) = group_iter.next() {
        if punct.as_char() == '=' {
            if let Some(TokenTree::Literal(lit)) = group_iter.next() {
                return lit
                    .to_string()
                    .trim_start_matches('"')
                    .trim_end_matches('"')
                    .to_string();
            }
        }
    }
    panic!("Legion proc-macro: invalid literal for attribute");
}

// Retrieive the token for the "default" attributes. Manually parse token to
// support tuple, arrays, constants, literal.
fn get_default_token_stream(
    group_iter: &mut std::iter::Peekable<proc_macro2::token_stream::IntoIter>,
) -> Option<TokenStream> {
    if let Some(TokenTree::Punct(punct)) = group_iter.next() {
        if punct.as_char() == '=' {
            if let Some(default_value) = group_iter.next() {
                let value = match default_value {
                    TokenTree::Ident(ident) => {
                        let mut token_ident = ident.to_string();
                        loop {
                            if let Some(TokenTree::Punct(punct)) = group_iter.peek() {
                                if punct.as_char() == ':' || punct.as_char() == '.' {
                                    token_ident.push(punct.as_char());
                                } else {
                                    break;
                                }
                            } else if let Some(TokenTree::Ident(ident)) = group_iter.peek() {
                                token_ident.push_str(ident.to_string().as_str());
                            } else if let Some(TokenTree::Group(group)) = group_iter.peek() {
                                token_ident.push_str(group.to_string().as_str());
                                group_iter.next();
                                break;
                            } else {
                                break;
                            }
                            group_iter.next();
                        }
                        let token_val: proc_macro2::TokenStream = token_ident.parse().unwrap();

                        Some(quote! { #token_val })
                    }

                    TokenTree::Literal(lit) => {
                        let lit_str = lit.to_string();
                        let token_val: proc_macro2::TokenStream = lit_str.parse().unwrap();
                        Some(quote! { #token_val })
                    }

                    TokenTree::Group(group) => {
                        let token_val: proc_macro2::TokenStream =
                            group.to_string().parse().unwrap();
                        Some(quote! { #token_val.into() })
                    }
                    TokenTree::Punct(_punct) => panic!(
                        "Legion proc-macro: unexpected punct in syntax for attribute 'default'"
                    ),
                };
                return value;
            }
        }
    }
    panic!("Legion proc-macro: invalid syntax for attribute 'default'");
}

fn get_resource_type(
    group_iter: &mut std::iter::Peekable<proc_macro2::token_stream::IntoIter>,
) -> Option<syn::Path> {
    let mut attrib_str = String::new();

    if let Some(TokenTree::Punct(punct)) = group_iter.peek() {
        if punct.as_char() == '=' {
            group_iter.next();
        } else {
            return None;
        }
    }

    loop {
        match group_iter.peek() {
            Some(TokenTree::Punct(punct)) => {
                if punct.as_char() == ',' {
                    break;
                }
                attrib_str.push_str(&group_iter.next().unwrap().to_string());
            }
            None => break,
            Some(_) => attrib_str.push_str(&group_iter.next().unwrap().to_string()),
        }
    }

    if attrib_str.is_empty() {
        return None;
    }

    syn::parse_str(&attrib_str).ok()
}

pub fn get_member_info(field: &syn::Field, type_path: syn::Path) -> MemberMetaInfo {
    let mut member_info = MemberMetaInfo {
        name: field.ident.as_ref().unwrap().to_string(),
        type_path,
        resource_type: None,
        imports: vec![],
        group: String::default(),
        offline: false,
        hidden: false,
        readonly: false,
        transient: false,
        tooltip: String::default(),
        default_literal: None,
    };

    field
        .attrs
        .iter()
        .filter(|attr| attr.path.is_ident(LEGION_TAG))
        .for_each(|attr| {
            let token_stream = attr.tokens.clone();
            let mut token_iter = token_stream.into_iter();

            if let Some(TokenTree::Group(group)) = token_iter.next() {
                let mut group_iter = group.stream().into_iter().peekable();

                while let Some(TokenTree::Ident(ident)) = group_iter.next() {
                    match ident.to_string().as_str() {
                        DEFAULT_ATTR => {
                            member_info.default_literal = get_default_token_stream(&mut group_iter);
                        }
                        READONLY_ATTR => member_info.readonly = true,
                        HIDDEN_ATTR => member_info.hidden = true,
                        OFFLINE_ATTR => member_info.offline = true,
                        TRANSIENT_ATTR => member_info.transient = true,
                        RESOURCE_TYPE_ATTR => {
                            member_info.resource_type = get_resource_type(&mut group_iter);
                            member_info
                                .imports
                                .push(syn::parse_str("lgn_data_offline::ResourcePathId").unwrap());
                        }
                        TOOLTIP_ATTR => {
                            member_info.tooltip = get_attribute_literal(&mut group_iter);
                        }
                        GROUP_ATTR => {
                            member_info.group = get_attribute_literal(&mut group_iter);
                        }
                        _ => {}
                    }

                    if let Some(TokenTree::Punct(punct)) = group_iter.next() {
                        if punct.as_char() != ',' {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        });

    member_info
}
