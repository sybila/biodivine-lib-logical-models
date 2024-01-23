#![allow(dead_code)]

use std::str::FromStr;

#[derive(Debug)]
pub enum ComparisonOperator {
    Eq,
    Neq,
    Lt,
    Gt,
    Leq,
    Geq,
}

impl ComparisonOperator {
    pub fn flip(&self) -> Self {
        match self {
            Self::Eq => Self::Eq,
            Self::Neq => Self::Neq,
            Self::Lt => Self::Gt,
            Self::Gt => Self::Lt,
            Self::Leq => Self::Geq,
            Self::Geq => Self::Leq,
        }
    }
}

impl FromStr for ComparisonOperator {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "eq" => Ok(Self::Eq),
            "neq" => Ok(Self::Neq),
            "lt" => Ok(Self::Lt),
            "gt" => Ok(Self::Gt),
            "leq" => Ok(Self::Leq),
            "geq" => Ok(Self::Geq),
            _ => Err(()),
        }
    }
}

impl ToString for ComparisonOperator {
    fn to_string(&self) -> String {
        match self {
            Self::Eq => "eq",
            Self::Neq => "neq",
            Self::Lt => "lt",
            Self::Gt => "gt",
            Self::Leq => "leq",
            Self::Geq => "geq",
        }
        .to_string()
    }
}

/// Represents a formula in the form of `variable comparison_operator value`.
///
/// This order is fixed. To represent a formula of form `value comparison_operator variable`,
/// use `comparison_operator.flip()`.
#[derive(Debug)]
pub struct Proposition<T> {
    pub comparison_operator: ComparisonOperator,
    pub variable: String,
    pub value: T,
}

impl<T> Proposition<T> {
    pub fn new(comparison_operator: ComparisonOperator, variable: String, value: T) -> Self {
        Self {
            comparison_operator,
            variable,
            value,
        }
    }
}

pub struct Person {
    /// A person must have a name, no matter how much Juliet may hate it
    name: String,
}
