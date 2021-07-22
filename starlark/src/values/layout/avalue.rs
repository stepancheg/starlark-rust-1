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

use crate::{
    codemap::Span,
    environment::Globals,
    eval::{Evaluator, Parameters},
    values::{
        ComplexValue, ConstFrozenValue, Freezer, Heap, SimpleValue, StarlarkValue, Tracer, Value,
    },
};
use gazebo::{
    any::AnyLifetime,
    coerce::{coerce, Coerce},
};
use std::{
    any::TypeId,
    cmp::Ordering,
    fmt::{self, Debug},
};

/// A trait that covers [`StarlarkValue`]. If you need a real [`StarlarkValue`] see `as_starlark_value`.
pub trait AValue<'v>: StarlarkValue<'v> {
    #[doc(hidden)]
    fn trace(&mut self, tracer: &Tracer<'v>);

    #[doc(hidden)]
    fn into_simple(
        self: Box<Self>,
        freezer: &Freezer,
    ) -> anyhow::Result<Box<dyn AValue<'static> + Send + Sync>>;
}

pub(crate) fn simple(x: impl SimpleValue) -> impl AValue<'static> + Send + Sync {
    Wrapper(Simple, x)
}

pub(crate) fn complex<'v>(x: impl ComplexValue<'v>) -> impl AValue<'v> {
    Wrapper(Complex, x)
}

// A type that implements SimpleValue.
struct Simple;

// A type that implements ComplexValue.
struct Complex;

// We want to define several types (Simple, Complex) that wrap a StarlarkValue,
// reimplement it, and do some things custom. The easiest way to avoid repeating
// the StarlarkValue trait each time is to make them all share a single wrapper,
// where Mode is one of Simple/Complex.
#[repr(C)]
struct Wrapper<Mode, T>(Mode, T);

// Safe because Complex/Simple are ZST
unsafe impl<T> Coerce<T> for Wrapper<Complex, T> {}
unsafe impl<T> Coerce<T> for Wrapper<Simple, T> {}

impl<'v, T: SimpleValue> AValue<'v> for Wrapper<Simple, T>
where
    'v: 'static,
{
    fn trace(&mut self, _tracer: &Tracer<'v>) {
        // Nothing to do
    }

    fn into_simple(
        self: Box<Self>,
        _freezer: &Freezer,
    ) -> anyhow::Result<Box<dyn AValue<'static> + Send + Sync>> {
        Ok(self)
    }
}

impl<'v, T: ComplexValue<'v>> AValue<'v> for Wrapper<Complex, T> {
    fn trace(&mut self, tracer: &Tracer<'v>) {
        self.1.trace(tracer)
    }

    fn into_simple(
        self: Box<Self>,
        freezer: &Freezer,
    ) -> anyhow::Result<Box<dyn AValue<'static> + Send + Sync>> {
        let x: Box<T> = coerce(self);
        let res = x.freeze(freezer)?;
        Ok(box simple(res))
    }
}

impl<Mode, T: Debug> Debug for Wrapper<Mode, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.1.fmt(f)
    }
}

// This is somewhat dubious. We can't define Wrapper<T> to be AnyLifetime, since these things
// don't compose properly. We do layout Wrapper such that it is really a T underneath,
// so we can pretend it really is just a T.
unsafe impl<'v, Mode: 'static, T: AnyLifetime<'v>> AnyLifetime<'v> for Wrapper<Mode, T> {
    fn static_type_id() -> TypeId
    where
        Self: Sized,
    {
        T::static_type_id()
    }

    fn static_type_of(&self) -> std::any::TypeId {
        Self::static_type_id()
    }
}

impl<'v, Mode: 'static, T: StarlarkValue<'v>> StarlarkValue<'v> for Wrapper<Mode, T> {
    fn get_type(&self) -> &'static str {
        self.1.get_type()
    }
    fn get_type_value(&self) -> &'static ConstFrozenValue {
        self.1.get_type_value()
    }
    fn matches_type(&self, ty: &str) -> bool {
        self.1.matches_type(ty)
    }
    fn get_methods(&self) -> Option<&'static Globals> {
        self.1.get_methods()
    }
    fn collect_repr(&self, collector: &mut String) {
        self.1.collect_repr(collector)
    }
    fn to_json(&self) -> anyhow::Result<String> {
        self.1.to_json()
    }
    fn to_bool(&self) -> bool {
        self.1.to_bool()
    }
    fn to_int(&self) -> anyhow::Result<i32> {
        self.1.to_int()
    }
    fn get_hash(&self) -> anyhow::Result<u64> {
        self.1.get_hash()
    }
    fn equals(&self, other: Value<'v>) -> anyhow::Result<bool> {
        self.1.equals(other)
    }
    fn compare(&self, other: Value<'v>) -> anyhow::Result<Ordering> {
        self.1.compare(other)
    }
    fn invoke(
        &self,
        me: Value<'v>,
        location: Option<Span>,
        params: Parameters<'v, '_>,
        eval: &mut Evaluator<'v, '_>,
    ) -> anyhow::Result<Value<'v>> {
        self.1.invoke(me, location, params, eval)
    }
    fn at(&self, index: Value<'v>, heap: &'v Heap) -> anyhow::Result<Value<'v>> {
        self.1.at(index, heap)
    }
    fn slice(
        &self,
        start: Option<Value<'v>>,
        stop: Option<Value<'v>>,
        stride: Option<Value<'v>>,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        self.1.slice(start, stop, stride, heap)
    }
    fn iterate(
        &'v self,
        heap: &'v Heap,
    ) -> anyhow::Result<Box<dyn Iterator<Item = Value<'v>> + 'v>> {
        self.1.iterate(heap)
    }
    fn length(&self) -> anyhow::Result<i32> {
        self.1.length()
    }
    fn get_attr(&self, attribute: &str, heap: &'v Heap) -> Option<Value<'v>> {
        self.1.get_attr(attribute, heap)
    }
    fn has_attr(&self, attribute: &str) -> bool {
        self.1.has_attr(attribute)
    }
    fn dir_attr(&self) -> Vec<String> {
        self.1.dir_attr()
    }
    fn is_in(&self, other: Value<'v>) -> anyhow::Result<bool> {
        self.1.is_in(other)
    }
    fn plus(&self, heap: &'v Heap) -> anyhow::Result<Value<'v>> {
        self.1.plus(heap)
    }
    fn minus(&self, heap: &'v Heap) -> anyhow::Result<Value<'v>> {
        self.1.minus(heap)
    }
    fn radd(&self, lhs: Value<'v>, heap: &'v Heap) -> Option<anyhow::Result<Value<'v>>> {
        self.1.radd(lhs, heap)
    }
    fn add(&self, rhs: Value<'v>, heap: &'v Heap) -> anyhow::Result<Value<'v>> {
        self.1.add(rhs, heap)
    }
    fn sub(&self, other: Value<'v>, heap: &'v Heap) -> anyhow::Result<Value<'v>> {
        self.1.sub(other, heap)
    }
    fn mul(&self, other: Value<'v>, heap: &'v Heap) -> anyhow::Result<Value<'v>> {
        self.1.mul(other, heap)
    }
    fn percent(&self, other: Value<'v>, heap: &'v Heap) -> anyhow::Result<Value<'v>> {
        self.1.percent(other, heap)
    }
    fn floor_div(&self, other: Value<'v>, heap: &'v Heap) -> anyhow::Result<Value<'v>> {
        self.1.floor_div(other, heap)
    }
    fn bit_and(&self, other: Value<'v>) -> anyhow::Result<Value<'v>> {
        self.1.bit_and(other)
    }
    fn bit_or(&self, other: Value<'v>) -> anyhow::Result<Value<'v>> {
        self.1.bit_or(other)
    }
    fn bit_xor(&self, other: Value<'v>) -> anyhow::Result<Value<'v>> {
        self.1.bit_xor(other)
    }
    fn left_shift(&self, other: Value<'v>) -> anyhow::Result<Value<'v>> {
        self.1.left_shift(other)
    }
    fn right_shift(&self, other: Value<'v>) -> anyhow::Result<Value<'v>> {
        self.1.right_shift(other)
    }
    fn export_as(&self, variable_name: &str, eval: &mut Evaluator<'v, '_>) {
        self.1.export_as(variable_name, eval)
    }
    fn set_at(&self, index: Value<'v>, new_value: Value<'v>) -> anyhow::Result<()> {
        self.1.set_at(index, new_value)
    }
    fn set_attr(&self, attribute: &str, new_value: Value<'v>) -> anyhow::Result<()> {
        self.1.set_attr(attribute, new_value)
    }
}
