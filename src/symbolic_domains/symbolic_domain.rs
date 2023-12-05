use biodivine_lib_bdd::{Bdd, BddVariableSet, BddVariableSetBuilder};

pub trait SymbolicDomain<T> {
    // todo in general, no need to enforce there should be a `max_value` -> ordering
    // todo vs want to somehow specify which values are allowed in this domain
    fn new(builder: &mut BddVariableSetBuilder, name: &str, max_value: T) -> Self;

    fn encode_one_todo(&self, bdd_variaable_set: &BddVariableSet, value: &T) -> Bdd;
    fn empty_collection_todo(&self, bdd_variable_set: &BddVariableSet) -> Bdd;
}

// todo maybe split into: SymbolicEncoding, SymbolicDomain;
