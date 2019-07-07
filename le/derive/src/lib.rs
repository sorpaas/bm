#![recursion_limit="128"]

extern crate proc_macro;

use quote::{quote, quote_spanned};
use syn::{parse_macro_input, DeriveInput, Field, Data, Fields};
use syn::spanned::Spanned;
use syn::punctuated::Punctuated;
use syn::token::Comma;

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
    unimplemented!()
}

fn struct_fields(data: &Data) -> Option<&Punctuated<Field, Comma>> {
    match data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => Some(&fields.named),
                Fields::Unnamed(ref fields) => Some(&fields.unnamed),
                Fields::Unit => None,
            }
        },
        Data::Enum(_) | Data::Union(_) => None,
    }
}
