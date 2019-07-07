#![recursion_limit="128"]

extern crate proc_macro;

use quote::{quote, quote_spanned};
use syn::{parse_macro_input, DeriveInput, Expr};
use syn::spanned::Spanned;
use deriving::{struct_fields, has_attribute, attribute_value};

use proc_macro::TokenStream;

#[proc_macro_derive(IntoTree, attributes(bm))]
pub fn into_tree_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let fields = struct_fields(&input.data)
        .expect("Not supported derive type")
        .iter()
        .map(|f| {
            let name = &f.ident;

            quote_spanned! { f.span() => {
                vector.push(bm_le::IntoTree::into_tree(&self.#name, db)?);
            } }
        });

    let expanded = quote! {
        impl<DB> bm_le::IntoTree<DB> for #name where
            DB: bm_le::Backend<Intermediate=bm_le::Intermediate, End=bm_le::End>
        {
            fn into_tree(&self, db: &mut DB) -> Result<bm_le::ValueOf<DB>, bm_le::Error<DB::Error>> {
                let mut vector = Vec::new();
                #(#fields)*
                bm_le::utils::vector_tree(&vector, db, None)
            }
        }

        impl bm_le::Composite for #name { }
    };

    proc_macro::TokenStream::from(expanded)
}

#[proc_macro_derive(FromTree, attributes(bm))]
pub fn from_tree_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let config_trait = attribute_value("bm", &input.attrs, "config_trait");

    let fields = struct_fields(&input.data)
        .expect("Not supported derive type")
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let name = &f.ident;

            if has_attribute("bm", &f.attrs, "vector") {
                let config = attribute_value("bm", &f.attrs, "len")
                    .expect("Must specify the length")
                    .parse::<Expr>().expect("Invalid syntax");

                if config_trait.is_some() {
                    quote_spanned! {
                        f.span() =>
                            #name: bm_le::FromVectorTreeWithConfig::from_vector_tree_with_config(
                                &vector.get(db, #i)?,
                                db,
                                #config,
                                None,
                                config,
                            )?,
                    }
                } else {
                    quote_spanned! {
                        f.span() =>
                            #name: bm_le::FromVectorTree::from_vector_tree(
                                &vector.get(db, #i)?,
                                db,
                                #config,
                                None,
                            )?,
                    }
                }
            } else if has_attribute("bm", &f.attrs, "list") {
                let config = attribute_value("bm", &f.attrs, "max_len")
                    .expect("Must specify the maximum length")
                    .parse::<Expr>().expect("Invalid syntax");

                if config_trait.is_some() {
                    quote_spanned! {
                        f.span() =>
                            #name: bm_le::FromListTreeWithConfig::from_list_tree_with_config(
                                &vector.get(db, #i)?,
                                db,
                                Some(#config),
                                config,
                            )?,
                    }
                } else {
                    quote_spanned! {
                        f.span() =>
                            #name: bm_le::FromListTree::from_list_tree(
                                &vector.get(db, #i)?,
                                db,
                                Some(#config)
                            )?,
                    }
                }
            } else {
                if config_trait.is_some() {
                    quote_spanned! {
                        f.span() =>
                            #name: bm_le::FromTreeWithConfig::from_tree_with_config(
                                &vector.get(db, #i)?,
                                db,
                                config,
                            )?,
                    }
                } else {
                    quote_spanned! {
                        f.span() =>
                            #name: bm_le::FromTree::from_tree(
                                &vector.get(db, #i)?,
                                db,
                            )?,
                    }
                }
            }
        });

    let fields_count = struct_fields(&input.data)
        .expect("Not supported derive type")
        .iter()
        .count();

    let expanded = if let Some(config_trait) = config_trait.clone() {
        let config_trait = config_trait.parse::<syn::TraitBound>().expect("Invalid syntax");
        quote! {
            impl<C: #config_trait, DB> bm_le::FromTreeWithConfig<C, DB> for #name where
                DB: bm_le::Backend<Intermediate=bm_le::Intermediate, End=bm_le::End>
            {
                fn from_tree_with_config(
                    root: &bm_le::ValueOf<DB>,
                    db: &DB,
                    config: &C
                ) -> Result<Self, bm_le::Error<DB::Error>> {
                    use bm_le::Leak;

                    let vector = bm_le::DanglingVector::<DB>::from_leaked(
                        (root.clone(), #fields_count, None)
                    );

                    Ok(Self {
                        #(#fields)*
                    })
                }
            }
        }
    } else {
        quote! {
            bm_le::impl_from_trait_with_empty_config!(#name);
            impl<DB> bm_le::FromTree<DB> for #name where
                DB: bm_le::Backend<Intermediate=bm_le::Intermediate, End=bm_le::End>
            {
                fn from_tree(
                    root: &bm_le::ValueOf<DB>,
                    db: &DB,
                ) -> Result<Self, bm_le::Error<DB::Error>> {
                    use bm_le::Leak;

                    let vector = bm_le::DanglingVector::<DB>::from_leaked(
                        (root.clone(), #fields_count, None)
                    );

                    Ok(Self {
                        #(#fields)*
                    })
                }
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}
