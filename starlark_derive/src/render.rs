/*
 * Copyright 2019 The Starlark in Rust Authors.
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use gazebo::prelude::*;
use proc_macro2::TokenStream;
use quote::{format_ident, quote_spanned};

use crate::{typ::*, util::*};

pub(crate) fn render(x: StarModule) -> TokenStream {
    let span = x.span();
    let StarModule {
        name,
        globals_builder,
        visibility,
        stmts,
        module_kind,
    } = x;
    let statics = format_ident!("{}", module_kind.statics_type_name());
    let stmts = stmts.into_map(render_stmt);
    quote_spanned! {
        span=>
        #visibility fn #name(globals_builder: #globals_builder) {
            fn build(globals_builder: #globals_builder) {
                #( #stmts )*
                // Mute warning if stmts is empty.
                let _ = globals_builder;
            }
            static RES: starlark::environment::#statics = starlark::environment::#statics::new();
            RES.populate(build, globals_builder);
        }
    }
}

fn render_stmt(x: StarStmt) -> TokenStream {
    match x {
        StarStmt::Const(x) => render_const(x),
        StarStmt::Attr(x) => render_attr(x),
        StarStmt::Fun(x) => render_fun(x),
    }
}

fn render_const(x: StarConst) -> TokenStream {
    let StarConst { name, ty, value } = x;
    let span = name.span();
    let name = ident_string(&name);
    quote_spanned! {
        span=>
        globals_builder.set::<#ty>(#name, #value);
    }
}

fn render_attr(x: StarAttr) -> TokenStream {
    let span = x.span();
    let StarAttr {
        name,
        arg,
        attrs,
        return_type,
        speculative_exec_safe,
        body,
    } = x;
    let name_str = ident_string(&name);
    let set_speculative_exec_safe = if speculative_exec_safe {
        Some(quote_spanned! {
            span=>
            attr.set_speculative_exec_safe();
        })
    } else {
        None
    };
    quote_spanned! {
        span=>
        #( #attrs )*
        #[allow(non_snake_case)] // Starlark doesn't have this convention
        fn #name<'v>(
            #[allow(unused_variables)]
            this: starlark::values::Value<'v>,
            heap: &'v starlark::values::Heap,
        ) -> anyhow::Result<starlark::values::Value<'v>> {
             fn inner<'v>(
                this: Value<'v>,
                #[allow(unused_variables)]
                heap: &'v starlark::values::Heap,
            ) -> anyhow::Result<#return_type> {
                #[allow(unused_variables)]
                let this: #arg = match starlark::values::UnpackValue::unpack_value(this) {
                    None => return Err(starlark::values::ValueError::IncorrectParameterTypeNamedWithExpected(
                        "this".to_owned(),
                        <#arg as starlark::values::UnpackValue>::expected(),
                        this.get_type().to_owned(),
                    ).into()),
                    Some(v) => v,
                };
                #body
            }
            Ok(heap.alloc(inner(this, heap)?))
        }
        #[allow(unused_mut)]
        let mut attr = starlark::values::function::NativeAttribute::new(#name);
        #set_speculative_exec_safe
        globals_builder.set(#name_str, attr);
    }
}

fn render_fun(x: StarFun) -> TokenStream {
    let span = x.span();

    let name_str = ident_string(&x.name);
    let signature = render_signature(&x);
    let binding = render_binding(&x);
    let is_method = x.is_method();

    let StarFun {
        name,
        type_attribute,
        attrs,
        args: _,
        return_type,
        speculative_exec_safe,
        body,
        source: _,
    } = x;

    let set_type = type_attribute.map(|x| {
        quote_spanned! {
            span=>
            const TYPE_N: usize = #x.len();
            static TYPE: starlark::values::StarlarkStrNRepr<TYPE_N> =
                starlark::values::StarlarkStrNRepr::new(#x);
            func.set_type(TYPE.unpack());
        }
    });
    let set_speculative_exec_safe = if speculative_exec_safe {
        Some(quote_spanned! {
            span=>
            func.set_speculative_exec_safe();
        })
    } else {
        None
    };

    let signature_arg = signature.as_ref().map(
        |_| quote_spanned! {span=> __signature: &starlark::eval::ParametersSpec<starlark::values::FrozenValue>,},
    );
    let signature_val = signature
        .as_ref()
        .map(|_| quote_spanned! {span=> __signature});
    let signature_val_ref = signature
        .as_ref()
        .map(|_| quote_spanned! {span=> &__signature});

    let (this_param, this_arg, new_function_or_method) = if is_method {
        (
            quote_spanned! {span=> __this: starlark::values::Value<'v>, },
            quote_spanned! {span=> __this, },
            quote_spanned! {span=> starlark::values::function::NativeMethod::new_direct },
        )
    } else {
        (
            quote_spanned! {span=> },
            quote_spanned! {span=> },
            quote_spanned! {span=> starlark::values::function::NativeFunction::new_direct },
        )
    };

    quote_spanned! {
        span=>
        #( #attrs )*
        #[allow(non_snake_case)] // Starlark doesn't have this convention
        fn #name<'v>(
            eval: &mut starlark::eval::Evaluator<'v, '_>,
            #this_param
            parameters: starlark::eval::Arguments<'v, '_>,
            #signature_arg
        ) -> anyhow::Result<starlark::values::Value<'v>> {
            fn inner<'v>(
                #[allow(unused_variables)]
                eval: &mut starlark::eval::Evaluator<'v, '_>,
                #this_param
                __args: starlark::eval::Arguments<'v, '_>,
                #signature_arg
            ) -> anyhow::Result<#return_type> {
                #[allow(unused_variables)]
                let heap = eval.heap();
                #binding
                #body
            }
            match inner(eval, #this_arg parameters, #signature_val) {
                Ok(v) => Ok(eval.heap().alloc(v)),
                Err(e) => Err(e),
            }
        }
        {
            #signature
            #[allow(unused_mut)]
            #[allow(clippy::redundant_closure)]
            let mut func = #new_function_or_method (
                move |eval, #this_arg parameters| #name(eval, #this_arg parameters, #signature_val_ref),
                #name_str.to_owned(),
            );
            #set_type
            #set_speculative_exec_safe
            globals_builder.set(#name_str, func);
        }
    }
}

// Given __args and __signature (if render_signature was Some)
// create bindings for all the arguments
fn render_binding(x: &StarFun) -> TokenStream {
    let span = x.args_span();
    match x.source {
        StarFunSource::Parameters => {
            let StarArg {
                span,
                attrs,
                name,
                ty,
                ..
            } = &x.args[0];
            let span = *span;
            quote_spanned! { span=> #( #attrs )* let #name : #ty = __args; }
        }
        StarFunSource::ThisParameters => {
            let StarArg {
                span,
                attrs,
                name,
                ty,
                ..
            } = &x.args[1];
            let span = *span;
            let this = render_binding_arg(&x.args[0]);
            quote_spanned! {
                span=>
                #this
                #( #attrs )* let #name : #ty = __args;
            }
        }
        StarFunSource::Argument(arg_count) => {
            let bind_args = x.args.map(render_binding_arg);
            quote_spanned! {
                span=>
                let __args: [_; #arg_count] = __signature.collect_into(__args, eval.heap())?;
                #( #bind_args )*
            }
        }
        StarFunSource::Positional(required, optional) => {
            let bind_args = x.args.map(render_binding_arg);
            if optional == 0 {
                quote_spanned! {
                    span=>
                    __args.no_named_args()?;
                    let __required: [_; #required] = __args.positional(eval.heap())?;
                    #( #bind_args )*
                }
            } else {
                quote_spanned! {
                    span=>
                    __args.no_named_args()?;
                    let (__required, __optional): ([_; #required], [_; #optional]) = __args.optional(eval.heap())?;
                    #( #bind_args )*
                }
            }
        }
        ref s => unreachable!("Unknown StarFunSource: {:?}", s),
    }
}

// Create a binding for an argument given. If it requires an index, take from the index
fn render_binding_arg(arg: &StarArg) -> TokenStream {
    let span = arg.span;
    let name = &arg.name;
    let name_str = ident_string(name);
    let ty = &arg.ty;

    let source = match arg.source {
        StarArgSource::This => quote_spanned! {span=> __this},
        StarArgSource::Argument(i) => quote_spanned! {span=> __args[#i].get()},
        StarArgSource::Required(i) => quote_spanned! {span=> Some(__required[#i])},
        StarArgSource::Optional(i) => quote_spanned! {span=> __optional[#i]},
        ref s => unreachable!("unknown source: {:?}", s),
    };

    // Rust doesn't have powerful enough nested if yet
    let next = if arg.is_this() {
        quote_spanned! { span=> starlark::eval::Arguments::check_this(#source)? }
    } else if arg.is_option() {
        assert!(
            arg.default.is_none(),
            "Can't have Option argument with a default, for `{}`",
            name_str
        );
        quote_spanned! { span=> starlark::eval::Arguments::check_optional(#name_str, #source)? }
    } else if !arg.is_value() && arg.default.is_some() {
        let default = arg
            .default
            .as_ref()
            .unwrap_or_else(|| unreachable!("Checked on the line above"));
        quote_spanned! { span=>
            {
                // Combo
                #[allow(clippy::manual_unwrap_or)]
                #[allow(clippy::unnecessary_lazy_evaluations)]
                #[allow(clippy::redundant_closure)]
                let x = starlark::eval::Arguments::check_optional(#name_str, #source)?.unwrap_or_else(|| #default);
                x
            }
        }
    } else {
        quote_spanned! { span=> starlark::eval::Arguments::check_required(#name_str, #source)? }
    };

    let mutability = mut_token(arg.mutable);
    let attrs = &arg.attrs;
    quote_spanned! {
        span=>
        #( #attrs )*
        let #mutability #name: #ty = #next;
    }
}

// Given the arguments, create a variable `signature` with a `ParametersSpec` object.
// Or return None if you don't need a signature
fn render_signature(x: &StarFun) -> Option<TokenStream> {
    let span = x.args_span();
    if let StarFunSource::Argument(args_count) = x.source {
        let name_str = ident_string(&x.name);
        let sig_args = x.args.map(render_signature_arg);
        Some(quote_spanned! {
            span=>
            #[allow(unused_mut)]
            let mut __signature = starlark::eval::ParametersSpec::with_capacity(#name_str.to_owned(), #args_count);
            #( #sig_args )*
        })
    } else {
        None
    }
}

// Generate a statement that modifies signature to add a new argument in.
fn render_signature_arg(arg: &StarArg) -> TokenStream {
    let span = arg.span;

    let mut name_str_full = (if arg.by_ref { "$" } else { "" }).to_owned();
    name_str_full += &ident_string(&arg.name);
    let name_str = name_str_full.trim_matches('_');

    if arg.is_args() {
        assert!(arg.default.is_none(), "Can't have *args with a default");
        quote_spanned! { span=> __signature.args();}
    } else if arg.is_kwargs() {
        assert!(arg.default.is_none(), "Can't have **kwargs with a default");
        quote_spanned! { span=> __signature.kwargs();}
    } else if arg.is_this() {
        quote_spanned! { span=> }
    } else if arg.is_option() {
        quote_spanned! { span=> __signature.optional(#name_str);}
    } else if let Some(default) = &arg.default {
        // For things that are type Value, we put them on the frozen heap.
        // For things that aren't type value, use optional and then next_opt/unwrap
        // to avoid the to/from value conversion.
        if arg.is_value() {
            quote_spanned! { span=> __signature.defaulted(#name_str, globals_builder.alloc(#default));}
        } else {
            quote_spanned! { span=> __signature.optional(#name_str);}
        }
    } else {
        quote_spanned! { span=> __signature.required(#name_str);}
    }
}
