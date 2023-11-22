use biodivine_lib_bdd::BddVariableSetBuilder;

pub trait SymbolicDomain<T> {
    // todo in general, no need to enforce there should be a `max_value` -> ordering
    // todo vs want to somehow specify which values are allowed in this domain
    fn new(builder: &mut BddVariableSetBuilder, name: &str, max_value: T) -> Self;
}

// todo maybe split into: SymbolicEncoding, SymbolicDomain;
