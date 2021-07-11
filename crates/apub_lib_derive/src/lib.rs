use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(ActivityHandlerNew)]
pub fn derive_activity_handler(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
  // Parse the input tokens into a syntax tree.
  let input = parse_macro_input!(input as DeriveInput);

  // Used in the quasi-quotation below as `#name`.
  let name = input.ident;

  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

  let input_enum = if let Data::Enum(d) = input.data {
    d
  } else {
    unimplemented!()
  };

  let impl_verify = input_enum
    .variants
    .iter()
    .map(|variant| variant_impl_verify(&name, variant));
  let impl_receive = input_enum
    .variants
    .iter()
    .map(|variant| variant_impl_receive(&name, variant));
  let impl_common = input_enum
    .variants
    .iter()
    .map(|variant| variant_impl_common(&name, variant));

  // The generated impl.
  let expanded = quote! {
      #[async_trait::async_trait(?Send)]
      impl #impl_generics lemmy_apub_lib::ActivityHandlerNew for #name #ty_generics #where_clause {
          async fn verify(
              &self,
              context: &LemmyContext,
              request_counter: &mut i32,
            ) -> Result<(), LemmyError> {
            match self {
              #(#impl_verify)*
            }
          }
          async fn receive(
            &self,
            context: &LemmyContext,
            request_counter: &mut i32,
          ) -> Result<(), LemmyError> {
            match self {
              #(#impl_receive)*
            }
          }
          fn common(&self) -> &ActivityCommonFields {
            match self {
              #(#impl_common)*
            }
          }
      }
  };

  // Hand the output tokens back to the compiler.
  proc_macro::TokenStream::from(expanded)
}

fn variant_impl_common(name: &syn::Ident, variant: &syn::Variant) -> TokenStream {
  let id = &variant.ident;
  match &variant.fields {
    syn::Fields::Unnamed(_) => {
      quote! {
        #name::#id(a) => a.common(),
      }
    }
    _ => unimplemented!(),
  }
}

fn variant_impl_verify(name: &syn::Ident, variant: &syn::Variant) -> TokenStream {
  let id = &variant.ident;
  match &variant.fields {
    syn::Fields::Unnamed(_) => {
      quote! {
        #name::#id(a) => a.verify(context, request_counter).await,
      }
    }
    _ => unimplemented!(),
  }
}

fn variant_impl_receive(name: &syn::Ident, variant: &syn::Variant) -> TokenStream {
  let id = &variant.ident;
  match &variant.fields {
    syn::Fields::Unnamed(_) => {
      quote! {
        #name::#id(a) => a.receive(context, request_counter).await,
      }
    }
    _ => unimplemented!(),
  }
}
