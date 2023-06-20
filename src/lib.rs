/// A private module which stores the implementation of the traits/structures relevant for
/// symbolic encoding of logical models.
///
/// TODO:
///     In the final library, we should re-export the relevant types from this module here.
mod symbolic_domain;

pub use symbolic_domain::{
    GenericIntegerDomain, GenericStateSpaceDomain, SymbolicDomain, UnaryIntegerDomain,
};

pub fn add(x: i32, y: i32) -> i32 {
    x + y
}

#[cfg(test)]
mod tests {
    use super::add;

    #[test]
    pub fn test() {
        assert_eq!(5, add(2, 3));
    }
}
