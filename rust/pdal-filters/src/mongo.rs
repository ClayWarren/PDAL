//! `filters.mongo` -- pass points that satisfy a MongoDB-style query.
//!
//! Port of `filters/MongoExpressionFilter.cpp` and the private evaluator in
//! `filters/private/mongoexpression/`. The expression is a JSON document of
//! nested logical gates (`$and`, `$or`, `$nor`, `$not`) and per-dimension
//! comparisons (`$eq`, `$ne`, `$gt`, `$gte`, `$lt`, `$lte`, `$in`, `$nin`).
//!
//! The tree is built when the option is parsed (a structurally invalid
//! document is rejected there); dimension references are resolved against the
//! point layout on the first run.

use pdal_core::point::{DimId, PointId, PointLayout, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use serde_json::{Map, Value};

/// The eight MongoDB comparison operators.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CompOp {
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
    Ne,
    In,
    Nin,
}

impl CompOp {
    /// Parse a `$`-prefixed operator key (PDAL's `toComparisonType`).
    fn parse(s: &str) -> Result<CompOp, String> {
        Ok(match s {
            "$eq" => CompOp::Eq,
            "$gt" => CompOp::Gt,
            "$gte" => CompOp::Gte,
            "$lt" => CompOp::Lt,
            "$lte" => CompOp::Lte,
            "$ne" => CompOp::Ne,
            "$in" => CompOp::In,
            "$nin" => CompOp::Nin,
            _ => return Err(format!("Invalid comparison type: {s}")),
        })
    }

    /// Whether the operator takes a list of operands (`$in` / `$nin`).
    fn is_multi(self) -> bool {
        matches!(self, CompOp::In | CompOp::Nin)
    }
}

/// The right-hand side of a comparison: a constant or a dimension reference
/// (PDAL's `Operand`).
enum Operand {
    Value(f64),
    Dim(DimId),
}

impl Operand {
    fn from_json(json: &Value) -> Result<Operand, String> {
        if let Some(name) = json.as_str() {
            Ok(Operand::Dim(DimId::from_name(name)))
        } else if let Some(value) = json.as_f64() {
            Ok(Operand::Value(value))
        } else {
            Err(format!("Invalid comparison operand: {json}"))
        }
    }

    fn get(&self, view: &PointView, idx: PointId) -> f64 {
        match self {
            Operand::Value(v) => *v,
            Operand::Dim(d) => view.get_f64(idx, d),
        }
    }
}

/// One per-dimension comparison leaf (PDAL's `Comparison` hierarchy).
enum Comparison {
    Single {
        dim: DimId,
        op: CompOp,
        operand: Operand,
    },
    Multi {
        dim: DimId,
        op: CompOp,
        operands: Vec<Operand>,
    },
}

impl Comparison {
    /// Build a comparison for `dim` from `json` (PDAL's `Comparison::create`).
    fn create(dim: &str, json: &Value) -> Result<Comparison, String> {
        // A bare value is shorthand for an `$eq` comparison.
        if !json.is_object() {
            let mut converted = Map::new();
            converted.insert("$eq".to_string(), json.clone());
            return Comparison::create(dim, &Value::Object(converted));
        }
        let obj = json.as_object().unwrap();
        if obj.len() != 1 {
            return Err(format!("Invalid comparison object: {json}"));
        }
        let (key, val) = obj.iter().next().unwrap();
        let op = CompOp::parse(key)?;
        let dim = DimId::from_name(dim);
        if op.is_multi() {
            let arr = val
                .as_array()
                .ok_or_else(|| format!("Invalid comparisons: {val}"))?;
            let mut operands = Vec::with_capacity(arr.len());
            for v in arr {
                operands.push(Operand::from_json(v)?);
            }
            Ok(Comparison::Multi { dim, op, operands })
        } else {
            let operand = Operand::from_json(val)?;
            Ok(Comparison::Single { dim, op, operand })
        }
    }

    /// Verify every referenced dimension is registered in `layout`.
    fn prepare(&self, layout: &PointLayout) -> Result<(), String> {
        fn check(layout: &PointLayout, dim: &DimId) -> Result<(), String> {
            if layout.dim(dim).is_some() {
                Ok(())
            } else {
                Err(format!("Unknown dimension: {}", dim.name()))
            }
        }
        match self {
            Comparison::Single { dim, operand, .. } => {
                check(layout, dim)?;
                if let Operand::Dim(d) = operand {
                    check(layout, d)?;
                }
            }
            Comparison::Multi { dim, operands, .. } => {
                check(layout, dim)?;
                for o in operands {
                    if let Operand::Dim(d) = o {
                        check(layout, d)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn eval(&self, view: &PointView, idx: PointId) -> bool {
        match self {
            Comparison::Single { dim, op, operand } => {
                let a = view.get_f64(idx, dim);
                let b = operand.get(view, idx);
                match op {
                    CompOp::Eq => a == b,
                    CompOp::Ne => a != b,
                    CompOp::Gt => a > b,
                    CompOp::Gte => a >= b,
                    CompOp::Lt => a < b,
                    CompOp::Lte => a <= b,
                    CompOp::In | CompOp::Nin => unreachable!(),
                }
            }
            Comparison::Multi { dim, op, operands } => {
                let a = view.get_f64(idx, dim);
                let found = operands.iter().any(|o| a == o.get(view, idx));
                match op {
                    CompOp::In => found,
                    CompOp::Nin => !found,
                    _ => unreachable!(),
                }
            }
        }
    }
}

/// A node in the expression tree: a logical gate or a comparison leaf
/// (PDAL's `LogicGate` / `Filterable`).
enum Gate {
    And(Vec<Gate>),
    Or(Vec<Gate>),
    Nor(Vec<Gate>),
    Not(Box<Gate>),
    Compare(Comparison),
}

impl Gate {
    fn prepare(&self, layout: &PointLayout) -> Result<(), String> {
        match self {
            Gate::And(children) | Gate::Or(children) | Gate::Nor(children) => {
                for child in children {
                    child.prepare(layout)?;
                }
                Ok(())
            }
            Gate::Not(child) => child.prepare(layout),
            Gate::Compare(cmp) => cmp.prepare(layout),
        }
    }

    fn eval(&self, view: &PointView, idx: PointId) -> bool {
        match self {
            Gate::And(children) => children.iter().all(|c| c.eval(view, idx)),
            Gate::Or(children) => children.iter().any(|c| c.eval(view, idx)),
            Gate::Nor(children) => !children.iter().any(|c| c.eval(view, idx)),
            Gate::Not(child) => !child.eval(view, idx),
            Gate::Compare(cmp) => cmp.eval(view, idx),
        }
    }
}

fn is_logical_operator(key: &str) -> bool {
    matches!(key, "$and" | "$not" | "$or" | "$nor")
}

/// Recursively translate `json` into gates appended to `out` (PDAL's
/// `Expression::build`). Object levels with several keys are implicitly
/// AND-ed together.
fn build(out: &mut Vec<Gate>, json: &Value) -> Result<(), String> {
    if let Some(array) = json.as_array() {
        for value in array {
            build(out, value)?;
        }
        return Ok(());
    }

    let obj = json
        .as_object()
        .ok_or_else(|| format!("Unexpected expression: {json}"))?;

    let mut collected: Vec<Gate> = Vec::new();
    for (key, val) in obj {
        if is_logical_operator(key) {
            // `$not` negates a single expression; the others take arrays.
            if key != "$not" && !val.is_array() {
                return Err("Logical operator expressions must be arrays".to_string());
            }
            let mut inner: Vec<Gate> = Vec::new();
            build(&mut inner, val)?;
            collected.push(match key.as_str() {
                "$and" => Gate::And(inner),
                "$or" => Gate::Or(inner),
                "$nor" => Gate::Nor(inner),
                "$not" => {
                    if inner.len() > 1 {
                        return Err("Cannot push onto a logical NOT".to_string());
                    }
                    let child = inner.into_iter().next().unwrap_or(Gate::And(Vec::new()));
                    Gate::Not(Box::new(child))
                }
                _ => unreachable!(),
            });
        } else if !val.is_object() || val.as_object().map_or(false, |o| o.len() == 1) {
            // A comparison object (or a bare value shorthand for `$eq`).
            collected.push(Gate::Compare(Comparison::create(key, val)?));
        } else {
            // `key` is a dimension; `val` holds several comparisons for it,
            // each of which is its own (implicitly AND-ed) comparison.
            for (inner_key, inner_val) in val.as_object().unwrap() {
                let mut nest = Map::new();
                nest.insert(inner_key.clone(), inner_val.clone());
                collected.push(Gate::Compare(Comparison::create(key, &Value::Object(nest))?));
            }
        }
    }

    if obj.len() > 1 {
        out.push(Gate::And(collected));
    } else {
        out.extend(collected);
    }
    Ok(())
}

/// The `filters.mongo` stage.
pub struct MongoExpressionFilter {
    root: Gate,
    prepared: bool,
}

impl MongoExpressionFilter {
    /// Parse a JSON query document and build the expression tree. A JSON
    /// syntax error or a structurally invalid document is rejected here,
    /// mirroring PDAL, where the expression is built as the option is
    /// consumed.
    pub fn new(json_text: &str) -> Result<Self, StageError> {
        let value: Value = serde_json::from_str(json_text)
            .map_err(|e| StageError(format!("Invalid JSON expression: {e}")))?;
        if value.is_null() {
             return Err(StageError("No expressions provided".to_string()));
        }
        let mut children: Vec<Gate> = Vec::new();
        build(&mut children, &value).map_err(StageError)?;
        Ok(MongoExpressionFilter {
            root: Gate::And(children),
            prepared: false,
        })
    }

    /// Resolve every dimension reference against `layout`, once.
    fn ensure_prepared(&mut self, layout: &PointLayout) -> Result<(), StageError> {
        if !self.prepared {
            self.root.prepare(layout).map_err(StageError)?;
            self.prepared = true;
        }
        Ok(())
    }
}

impl Filter for MongoExpressionFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.mongo"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.ensure_prepared(input.layout().as_ref())?;

        // `filters.mongo` always emits exactly one view (PDAL's `run`).
        let mut output = input.make_new();
        for idx in 0..input.len() {
            if self.root.eval(input, idx) {
                output.append_point(input, idx);
            }
        }
        Ok(vec![output])
    }
}

impl Streamable for MongoExpressionFilter {
    fn process_one(&mut self, view: &PointView, idx: PointId) -> bool {
        if self.ensure_prepared(view.layout().as_ref()).is_err() {
            return false;
        }
        self.root.eval(view, idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    /// A one-point view over `X`, `Y`, `Z` with the given values.
    fn point(x: f64, y: f64, z: f64) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        let p = view.add_point();
        view.set_f64(p, &DimId::X, x);
        view.set_f64(p, &DimId::Y, y);
        view.set_f64(p, &DimId::Z, z);
        view
    }

    fn check(filter: &mut MongoExpressionFilter, view: &PointView) -> bool {
        filter.process_one(view, 0)
    }

    #[test]
    fn implicit_eq_against_constant() {
        let mut f = MongoExpressionFilter::new(r#"{"X": 0}"#).unwrap();
        assert!(!check(&mut f, &point(-1.0, 0.0, 0.0)));
        assert!(check(&mut f, &point(0.0, 0.0, 0.0)));
        assert!(!check(&mut f, &point(1.0, 0.0, 0.0)));
    }

    #[test]
    fn comparison_across_dimensions() {
        let mut f = MongoExpressionFilter::new(r#"{"X": {"$lt": "Y"}}"#).unwrap();
        assert!(check(&mut f, &point(1.0, 2.0, 0.0)));
        assert!(!check(&mut f, &point(2.0, 2.0, 0.0)));
    }

    #[test]
    fn each_single_operator() {
        for (op, neg, zero, pos) in [
            ("$eq", false, true, false),
            ("$ne", true, false, true),
            ("$gt", false, false, true),
            ("$gte", false, true, true),
            ("$lt", true, false, false),
            ("$lte", true, true, false),
        ] {
            let mut f =
                MongoExpressionFilter::new(&format!(r#"{{"X": {{"{op}": 0}}}}"#)).unwrap();
            assert_eq!(check(&mut f, &point(-1.0, 0.0, 0.0)), neg, "{op} neg");
            assert_eq!(check(&mut f, &point(0.0, 0.0, 0.0)), zero, "{op} zero");
            assert_eq!(check(&mut f, &point(1.0, 0.0, 0.0)), pos, "{op} pos");
        }
    }

    #[test]
    fn in_and_nin() {
        let mut any = MongoExpressionFilter::new(r#"{"X": {"$in": [0, 1, 2]}}"#).unwrap();
        assert!(check(&mut any, &point(2.0, 0.0, 0.0)));
        assert!(!check(&mut any, &point(4.0, 0.0, 0.0)));

        let mut none = MongoExpressionFilter::new(r#"{"X": {"$nin": [0, 1, 2]}}"#).unwrap();
        assert!(!check(&mut none, &point(2.0, 0.0, 0.0)));
        assert!(check(&mut none, &point(4.0, 0.0, 0.0)));
    }

    #[test]
    fn implicit_and_within_a_dimension() {
        let mut f =
            MongoExpressionFilter::new(r#"{"X": {"$gt": 0, "$lt": 2}}"#).unwrap();
        assert!(!check(&mut f, &point(0.0, 0.0, 0.0)));
        assert!(check(&mut f, &point(1.0, 0.0, 0.0)));
        assert!(!check(&mut f, &point(2.0, 0.0, 0.0)));
    }

    #[test]
    fn logical_and_or_nor() {
        let mut and =
            MongoExpressionFilter::new(r#"{"$and": [{"X": 0}, {"Y": 1}, {"Z": 2}]}"#).unwrap();
        assert!(check(&mut and, &point(0.0, 1.0, 2.0)));
        assert!(!check(&mut and, &point(0.0, 1.0, 0.0)));

        let mut or =
            MongoExpressionFilter::new(r#"{"$or": [{"X": 0}, {"Y": 1}, {"Z": 2}]}"#).unwrap();
        assert!(check(&mut or, &point(9.0, 9.0, 2.0)));
        assert!(!check(&mut or, &point(9.0, 9.0, 9.0)));

        let mut nor =
            MongoExpressionFilter::new(r#"{"$nor": [{"X": 0}, {"Y": 1}, {"Z": 2}]}"#).unwrap();
        assert!(!check(&mut nor, &point(0.0, 9.0, 9.0)));
        assert!(check(&mut nor, &point(9.0, 9.0, 9.0)));
    }

    #[test]
    fn logical_not() {
        let mut f =
            MongoExpressionFilter::new(r#"{"$not": {"X": {"$gt": 0}}}"#).unwrap();
        assert!(check(&mut f, &point(-1.0, 0.0, 0.0)));
        assert!(check(&mut f, &point(0.0, 0.0, 0.0)));
        assert!(!check(&mut f, &point(1.0, 0.0, 0.0)));
    }

    #[test]
    fn structurally_invalid_documents_are_rejected() {
        // A top-level array of bare strings is not a valid expression.
        assert!(MongoExpressionFilter::new(r#"["Red", 42]"#).is_err());
        // A logical operator pointing at a scalar.
        assert!(MongoExpressionFilter::new(r#"["$and", 42]"#).is_err());
        // A bare null.
        assert!(MongoExpressionFilter::new("null").is_err());
        // Invalid JSON.
        assert!(MongoExpressionFilter::new("{not json").is_err());
    }

    #[test]
    fn run_keeps_matching_points() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for x in [0.0, 1.0, 2.0, 1.0] {
            let p = view.add_point();
            view.set_f64(p, &DimId::X, x);
        }
        let mut f = MongoExpressionFilter::new(r#"{"X": {"$gte": 1}}"#).unwrap();
        let out = f.run(&view).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].len(), 3);
    }
}
