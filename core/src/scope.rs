use crate::ident::Ident;
use crate::result::FResult;
use crate::serialize::{Deserialize, Serialize};
use crate::value::Value;
use crate::{Attrs, Context, Span};
use crate::{ast::Expr, error::Interrupt};
use std::io;
use std::sync::Arc;

#[derive(Debug, Clone)]
enum ScopeValue {
	LazyVariable(Expr, Option<Arc<Scope>>),
}

impl ScopeValue {
	pub(crate) fn compare<I: Interrupt>(
		&self,
		other: &Self,
		ctx: &mut Context,
		int: &I,
	) -> FResult<bool> {
		let Self::LazyVariable(a1, a2) = self;
		let Self::LazyVariable(b1, b2) = other;
		Ok(a1.compare(b1, ctx, int)?
			&& compare_option_arc_scope(a2.as_ref(), b2.as_ref(), ctx, int)?)
	}

	fn eval<I: Interrupt>(
		&self,
		attrs: Attrs,
		spans: &mut Vec<Span>,
		context: &mut crate::Context,
		int: &I,
	) -> FResult<Value> {
		match self {
			Self::LazyVariable(expr, scope) => {
				let value =
					crate::ast::evaluate(expr.clone(), scope.clone(), attrs, spans, context, int)?;
				Ok(value)
			}
		}
	}

	pub(crate) fn serialize(&self, write: &mut impl io::Write) -> FResult<()> {
		match self {
			Self::LazyVariable(e, s) => {
				e.serialize(write)?;
				match s {
					None => false.serialize(write)?,
					Some(s) => {
						true.serialize(write)?;
						s.serialize(write)?;
					}
				}
			}
		}
		Ok(())
	}

	pub(crate) fn deserialize(read: &mut impl io::Read) -> FResult<Self> {
		Ok(Self::LazyVariable(Expr::deserialize(read)?, {
			if bool::deserialize(read)? {
				None
			} else {
				Some(Arc::new(Scope::deserialize(read)?))
			}
		}))
	}
}

#[derive(Debug, Clone)]
pub(crate) struct Scope {
	ident: Ident,
	value: ScopeValue,
	inner: Option<Arc<Scope>>,
}

pub(crate) fn compare_option_arc_scope<I: Interrupt>(
	a: Option<&Arc<Scope>>,
	b: Option<&Arc<Scope>>,
	ctx: &mut Context,
	int: &I,
) -> FResult<bool> {
	Ok(match (a, b) {
		(None, None) => true,
		(Some(a), Some(b)) => a.compare(b, ctx, int)?,
		_ => false,
	})
}

impl Scope {
	pub(crate) fn compare<I: Interrupt>(
		&self,
		other: &Self,
		ctx: &mut Context,
		int: &I,
	) -> FResult<bool> {
		Ok(self.ident == other.ident
			&& self.value.compare(&other.value, ctx, int)?
			&& compare_option_arc_scope(self.inner.as_ref(), other.inner.as_ref(), ctx, int)?)
	}

	pub(crate) fn serialize(&self, write: &mut impl io::Write) -> FResult<()> {
		self.ident.serialize(write)?;
		self.value.serialize(write)?;
		match &self.inner {
			None => false.serialize(write)?,
			Some(s) => {
				true.serialize(write)?;
				s.serialize(write)?;
			}
		}
		Ok(())
	}

	pub(crate) fn deserialize(read: &mut impl io::Read) -> FResult<Self> {
		Ok(Self {
			ident: Ident::deserialize(read)?,
			value: ScopeValue::deserialize(read)?,
			inner: {
				if bool::deserialize(read)? {
					None
				} else {
					Some(Arc::new(Self::deserialize(read)?))
				}
			},
		})
	}

	const fn with_scope_value(ident: Ident, value: ScopeValue, inner: Option<Arc<Self>>) -> Self {
		Self {
			ident,
			value,
			inner,
		}
	}

	pub(crate) fn with_variable(
		name: Ident,
		expr: Expr,
		scope: Option<Arc<Self>>,
		inner: Option<Arc<Self>>,
	) -> Self {
		Self::with_scope_value(name, ScopeValue::LazyVariable(expr, scope), inner)
	}

	pub(crate) fn get<I: Interrupt>(
		&self,
		ident: &Ident,
		attrs: Attrs,
		spans: &mut Vec<Span>,
		context: &mut crate::Context,
		int: &I,
	) -> FResult<Option<Value>> {
		if self.ident.as_str() == ident.as_str() {
			let value = self.value.eval(attrs, spans, context, int)?;
			Ok(Some(value))
		} else {
			self.inner.as_ref().map_or_else(
				|| Ok(None),
				|inner| inner.get(ident, attrs, spans, context, int),
			)
		}
	}
}
