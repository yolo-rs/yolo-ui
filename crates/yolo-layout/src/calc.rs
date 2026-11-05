// SPDX-License-Identifier: Apache-2.0
//! `calc()` and relative-unit evaluation over Lightning CSS values.
//!
//! Every length expression over `+ - * /`, numbers, px (and absolute units), percent, `rem`,
//! `em`, `vw`, `vh`, `vmin` and `vmax` is linear in its percentage basis, so it reduces to
//! `px + percent * basis` ([`Length::from_linear`]). `min()`, `max()`, `clamp()` and the other
//! math functions are rejected: they are not linear.

use lightningcss::values::calc::{Calc, MathFunction};
use lightningcss::values::length::LengthValue;
use lightningcss::values::percentage::DimensionPercentage;

use crate::style::{Length, LengthContext};

type R<T> = Result<T, String>;

/// A calc term: a plain number or a linear length.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Term {
    Num(f32),
    Len { px: f32, pct: f32 },
}

/// One dimension to px against `cx`.
pub(crate) fn dimension_px(d: &LengthValue, cx: &LengthContext) -> R<f32> {
    if let Some(px) = d.to_px() {
        return Ok(px);
    }
    let viewport = |what: &str| {
        cx.viewport.ok_or_else(|| {
            format!("`{what}` needs a viewport size; set LengthContext::viewport before parsing")
        })
    };
    let px = match d {
        LengthValue::Em(v) => v * cx.font_size,
        LengthValue::Rem(v) => v * cx.root_font_size,
        LengthValue::Vw(v) => v * viewport("vw")?.0 / 100.0,
        LengthValue::Vh(v) => v * viewport("vh")?.1 / 100.0,
        LengthValue::Vmin(v) => {
            let (w, h) = viewport("vmin")?;
            v * w.min(h) / 100.0
        }
        LengthValue::Vmax(v) => {
            let (w, h) = viewport("vmax")?;
            v * w.max(h) / 100.0
        }
        other => {
            return Err(format!(
                "unit of `{other:?}` is not supported; use px, %, rem, em, vw, vh, vmin or vmax"
            ));
        }
    };
    Ok(px)
}

/// A `<length-percentage>` to a [`Length`].
pub(crate) fn length_percentage(v: &DimensionPercentage<LengthValue>, cx: &LengthContext) -> R<Length> {
    match v {
        DimensionPercentage::Dimension(d) => finite(dimension_px(d, cx)?, 0.0),
        DimensionPercentage::Percentage(p) => finite(0.0, p.0),
        DimensionPercentage::Calc(c) => match eval(c, cx)? {
            Term::Len { px, pct } => finite(px, pct),
            Term::Num(_) => Err("calc() must produce a length or a percentage, not a number".into()),
        },
    }
}

fn finite(px: f32, pct: f32) -> R<Length> {
    if px.is_finite() && pct.is_finite() {
        Ok(Length::from_linear(px, pct))
    } else {
        Err("calc() result is not finite (division by zero or overflow)".into())
    }
}

fn eval(c: &Calc<DimensionPercentage<LengthValue>>, cx: &LengthContext) -> R<Term> {
    match c {
        Calc::Value(v) => match &**v {
            DimensionPercentage::Dimension(d) => Ok(Term::Len { px: dimension_px(d, cx)?, pct: 0.0 }),
            DimensionPercentage::Percentage(p) => Ok(Term::Len { px: 0.0, pct: p.0 }),
            DimensionPercentage::Calc(inner) => eval(inner, cx),
        },
        Calc::Number(n) => Ok(Term::Num(*n)),
        Calc::Sum(a, b) => match (eval(a, cx)?, eval(b, cx)?) {
            (Term::Num(x), Term::Num(y)) => Ok(Term::Num(x + y)),
            (Term::Len { px: ax, pct: ap }, Term::Len { px: bx, pct: bp }) => {
                Ok(Term::Len { px: ax + bx, pct: ap + bp })
            }
            _ => Err("calc() cannot add a number to a length".into()),
        },
        Calc::Product(k, inner) => Ok(match eval(inner, cx)? {
            Term::Num(x) => Term::Num(x * k),
            Term::Len { px, pct } => Term::Len { px: px * k, pct: pct * k },
        }),
        Calc::Function(f) => match &**f {
            MathFunction::Calc(inner) => eval(inner, cx),
            _ => Err("min(), max(), clamp() and the other math functions are not supported; use calc()".into()),
        },
    }
}
