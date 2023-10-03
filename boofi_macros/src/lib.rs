use proc_macro::TokenStream;
use syn::{Attribute, parse_macro_input, DeriveInput, Data, Fields, Expr, Type, Token, ExprAssign,
          Field, Lit, GenericParam, parse_quote, PathArguments};
use syn::__private::quote::quote;
use syn::__private::ToTokens;
use syn::punctuated::Punctuated;

fn const_fix(typ: &mut Type, cnst: impl ToTokens) -> impl ToTokens {
    match typ {
        Type::Path(p) => {
            match &mut p.path.segments[0].arguments {
                PathArguments::AngleBracketed(p) => {
                    p.colon2_token = Some(parse_quote!(::));
                }
                _ => {}
            }
            quote!(#typ::#cnst)
        }
        Type::Tuple(_) => quote!(<#typ>::#cnst),
        _ => panic!("unsupported field typ")
    }
}

/// Represents a field with their information
#[derive(Debug)]
struct FieldAttributes {
    name: Option<String>,
    kind: Option<String>,
    description: Option<String>,
    typ: Type,
}

impl FieldAttributes {
    // use field name from attribute, struct or type name
    fn kind(&mut self) -> impl ToTokens {
        let typ = &mut self.typ;
        if let Some(n) = &self.kind {
            parse_quote!(#n)
        } else {
            const_fix(typ, quote!(KIND)).to_token_stream()
        }
    }

    // use field name from attribute, struct or type name
    fn name(&mut self) -> impl ToTokens {
        let typ = &mut self.typ;
        if let Some(n) = &self.name {
            parse_quote!(#n)
        } else {
           // quote!(#typ::NAME)
            const_fix(typ, quote!(NAME)).to_token_stream()
        }
    }

    // use field description from attribute or type
    fn description(&mut self) -> impl ToTokens {
        let typ = &mut self.typ;
        if let Some(n) = &self.description {
            parse_quote!(#n)
        } else {
            const_fix(typ, quote!(DESCRIPTION)).to_token_stream()
        }
    }

    /// parse attribute key=value (separated by ,) and store it
    fn add_key_value(&mut self, kv: &Expr) {
        #[allow(unused_assignments)]
        let mut key = None;
        #[allow(unused_assignments)]
        let mut value = None;

        // attribute
        match kv {
            Expr::Assign(a) => {
                match a {
                    ExprAssign { left, right, .. } => {
                        match left.as_ref() {
                            Expr::Path(p) => {
                                key = Some(p.path.segments[0].ident.to_string());
                            }
                            _ => panic!("invalid key expression")
                        }

                        match right.as_ref() {
                            Expr::Lit(p) => {
                                match &p.lit {
                                    Lit::Str(s) => value =  Some(s.value()),
                                    _ => panic!("unsupported value type")

                                }
                            }
                            _ => panic!("assignment invalid")
                        }
                    }
                }
            }
            _ => panic!(r#"valid expressions: name = "..", .. "#)
        }

        // assign known attribute
        match key.expect("identifier missing (name/kind/description)").as_str() {
            "name" => self.name = value,
            "kind" => self.kind = value,
            "description" => self.description = value,
            _=> {}
        }
    }
}

fn parse_attributes(attrs: &[Attribute], field_attributes: &mut FieldAttributes)  {
    for attr in attrs {
        if attr.meta.path().segments[0].ident.to_string() == "desc" {
            for kv in attr.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated).unwrap() {
                field_attributes.add_key_value(&kv);
            }
        }
    }
}

fn parse_field_attributes(field: &Field)  -> FieldAttributes {
    let mut desc = FieldAttributes {
        name: None,
        kind: None,
        description: None,
        typ: field.ty.clone(),
    };

    // field name - eventually overridden by attribute
    desc.name = field.ident.as_ref().map(|i|i.to_string());

    // attributes
    parse_attributes(field.attrs.as_slice(), &mut desc);
    desc
}

/// Generates Description implementation for the provided object.
/// Name, kind and description can be override by attribute `desc`
///
/// Description is used to generate serializable documentation.
///
#[proc_macro_derive(Description, attributes(desc))]
pub fn desc(item: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(item as DeriveInput);

    // parse object attributes
    let i = ast.ident.clone();
    let mut desc = FieldAttributes {
        name: None,
        kind: None,
        description: None,
        typ: parse_quote!(#i),
    };
    parse_attributes(ast.attrs.as_slice(), &mut desc);

    // parse struct fields
    let mut fields = vec![];
    match ast.data {
        Data::Struct(s) => {
            match s.fields {
                Fields::Named(n) => {
                    for name in n.named {
                        fields.push(parse_field_attributes(&name));
                    }
                }
                Fields::Unnamed(_) => {}
                Fields::Unit => {}
            }
        }
        Data::Enum(_) => {}
        Data::Union(_) => {}
    }

    let ident = ast.ident.to_token_stream();
    let generics = &mut ast.generics;
    let mut field_impls = vec![];

    for f in fields.iter_mut(){
        let kind = f.kind();
        let name = f.name();
        let description = f.description();

        let typ = &mut f.typ;
        let fields = const_fix(typ, quote!(FIELDS)).to_token_stream();

        field_impls.push(quote!{
            crate::description::DescriptionField {
                kind: #kind,
                name: #name,
                description: #description,
                fields: #fields
            }
        });
    }

    // add description bound to generic
    let mut generics_with_bounds = generics.clone();
    for p in &mut generics_with_bounds.params {
        match p {
            GenericParam::Type(ref mut t) => {
                t.bounds.push(parse_quote!(crate::description::Description));
            }
         _ => panic!("generic parameter unsupported")
        }
    }

    // generate description implementation with fields
    // use attribute if provided
    let kind = if let Some(n) = desc.kind {
        quote!(#n)
    } else {
        let i = ident.to_string();
        quote!(#i)
    };

    let name = if let Some(n) = desc.name {
        quote!(const NAME: &'static str = #n;)
    } else { quote!() };

    let description = if let Some(d) = desc.description {
        quote!(const DESCRIPTION: &'static str = #d;)
    } else { quote!() };

    let q = quote! {
        impl #generics_with_bounds crate::description::Description for #ident #generics {
            const KIND: &'static str =  #kind;
            #name
            #description

            const FIELDS: &'static [crate::description::DescriptionField] = &[
                #(#field_impls),*
            ];
        }
    };

    q.into()
}