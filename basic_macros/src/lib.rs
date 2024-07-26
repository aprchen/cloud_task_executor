extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, ItemFn, Lit, Meta, NestedMeta};

#[proc_macro_attribute]
pub fn cte_task(args: TokenStream, input: TokenStream) -> TokenStream {
    // 解析宏的参数和输入函数
    let args = parse_macro_input!(args as AttributeArgs);
    let input = parse_macro_input!(input as ItemFn);

    let name = if let Some(NestedMeta::Meta(Meta::NameValue(nv))) = args.first() {
        if nv.path.is_ident("name") {
            if let Lit::Str(ref s) = nv.lit {
                s.value()
            } else {
                panic!("Expected name to be a string literal");
            }
        } else {
            panic!("Expected name argument");
        }
    } else {
        panic!("Expected name argument");
    };
    let fn_name = &input.sig.ident;
    let fn_block = &input.block;
    let expanded = quote! {
        pub fn #fn_name() -> crate::executor::Task {
            crate::executor::Task::new(#name, |ctx, payload| {
                Box::pin(async move #fn_block)
            })
        }
    };
    expanded.into()
}