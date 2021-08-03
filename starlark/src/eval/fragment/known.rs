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

//! Things that operate on known values where we know we can do better.

use crate::{
    codemap::Spanned,
    eval::compiler::{Compiler, ExprCompiled, ExprCompiledValue},
    syntax::ast::{AstExpr, Expr},
};

/// Conditional statements are fairly common, some have literals (or imported values)
/// and quite a few start with a `not`, so encode those options statically.
pub(crate) enum Conditional {
    True,
    False,
    Normal(ExprCompiled),
    Negate(ExprCompiled),
}

impl Compiler<'_> {
    pub fn conditional(&mut self, expr: AstExpr) -> Conditional {
        let (expect, val) = match expr {
            Spanned {
                node: Expr::Not(box expr),
                ..
            } => (false, self.expr(expr)),
            _ => (true, self.expr(expr)),
        };
        match val {
            ExprCompiledValue::Value(x) => {
                if x.get_ref().to_bool() == expect {
                    Conditional::True
                } else {
                    Conditional::False
                }
            }
            ExprCompiledValue::Compiled(v) => {
                if expect {
                    Conditional::Normal(v)
                } else {
                    Conditional::Negate(v)
                }
            }
        }
    }
}