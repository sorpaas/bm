#![recursion_limit="128"]

extern crate proc_macro;

use quote::{quote, quote_spanned};
use syn::{parse_macro_input, DeriveInput};
use syn::spanned::Spanned;
use deriving::struct_fields;

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
    };

    proc_macro::TokenStream::from(expanded)
}

#[proc_macro_derive(FromTree, attributes(bm))]
pub fn from_tree_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let fields = struct_fields(&input.data)
        .expect("Not supported derive type")
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let name = &f.ident;

            quote_spanned! {
                f.span() =>
                    #name: bm_le::FromTree::from_tree(
                        &vector.get(db, #i)?,
                        db,
                    )?,
            }
        });

    let fields_count = struct_fields(&input.data)
        .expect("Not supported derive type")
        .iter()
        .count();

    let expanded =
        quote! {
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
        };

    proc_macro::TokenStream::from(expanded)
}
