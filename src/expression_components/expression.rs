#![allow(dead_code)]

use super::proposition::Proposition;

/// Represents a recursive expression. Leaf nodes are propositions. Join `Expression::Terminal`s
/// into more complex expressions using other `Expression` variants.
///
/// Available variants:
///
/// - `Expression::Terminal` - a leaf node, containing a proposition
/// - `Expression::Not` - a negation of the inner expression
/// - `Expression::And` - a conjunction of the inner expressions. The inner expressions are
/// stored inside a `Vec<_>`, to allow for an arbitrary number of conjuncts useful for
/// creating CNF formulas. `Expression::And` with an empty `Vec<_>` is equivalent to
/// constant `true`.
/// - `Expression::Or` - a disjunction of the inner expressions. The inner expressions are
/// stored inside a `Vec<_>`, to allow for an arbitrary number of disjuncts useful for
/// creating DNF formulas. `Expression::Or` with an empty `Vec<_>` is equivalent to
/// constant `false`.
/// - `Expression::Xor` - an exclusive disjunction of the inner expressions.
/// - `Expression::Implies` - an implication of the inner expressions. The order of the
/// operands follows conventional notation, i.e. `Expression::Implies(lhs, rhs)` is
/// equivalent to `lhs => rhs`.
pub enum Expression<T> {
    Terminal(Proposition<T>),
    Not(Box<Expression<T>>),
    And(Vec<Expression<T>>),
    Or(Vec<Expression<T>>),
    Xor(Box<Expression<T>>, Box<Expression<T>>),
    Implies(Box<Expression<T>>, Box<Expression<T>>),
}

// // todo
// //  really torn apart between keeping this here, moving it to a separate file within
// //  this module, and moving it into the `xml_parsing` module
// impl<T: FromStr> Expression<T> {
//     pub fn try_from_xml<XR, BR>(_xml: &mut XR)
//     where
//         XR: XmlReader<BR>,
//         BR: BufRead,
//     {
//         unimplemented!()
//     }
// }
