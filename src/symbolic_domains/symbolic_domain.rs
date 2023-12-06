use biodivine_lib_bdd::{Bdd, BddVariableSet, BddVariableSetBuilder};

pub trait SymbolicDomain<T> {
    // todo in general, no need to enforce there should be a `max_value` -> ordering
    // todo vs want to somehow specify which values are allowed in this domain
    fn new(builder: &mut BddVariableSetBuilder, name: &str, max_value: T) -> Self;

    fn encode_one(&self, bdd_variable_set: &BddVariableSet, value: &T) -> Bdd;
    // todo here because the `and(unit_set)` is often forgotten
    fn encode_one_not(&self, bdd_variable_set: &BddVariableSet, value: &T) -> Bdd {
        self.encode_one(bdd_variable_set, value)
            .not()
            .and(&self.unit_collection(bdd_variable_set))
    }
    fn empty_collection(&self, bdd_variable_set: &BddVariableSet) -> Bdd;
    fn unit_collection(&self, bdd_variable_set: &BddVariableSet) -> Bdd;
}

pub trait SymbolicDomainOrd<T>: SymbolicDomain<T> {
    /// Encodes the set of values that are strictly less than the given value.
    fn encode_lt(&self, bdd_variable_set: &BddVariableSet, value: &T) -> Bdd;
    /// Encodes the set of values that are less than or equal to the given value.
    fn encode_le(&self, bdd_variable_set: &BddVariableSet, value: &T) -> Bdd {
        self.encode_lt(bdd_variable_set, value)
            .or(&self.encode_one(bdd_variable_set, value))
    }
    /// Encodes the set of values that are strictly greater than the given value.
    fn encode_gt(&self, bdd_variable_set: &BddVariableSet, value: &T) -> Bdd {
        self.encode_le(bdd_variable_set, value)
            .not()
            // not(), in general, might produce a set containing invalid values
            .and(&self.unit_collection(bdd_variable_set))
    }
    /// Encodes the set of values that are greater than or equal to the given value.
    fn encode_ge(&self, bdd_variable_set: &BddVariableSet, value: &T) -> Bdd {
        self.encode_gt(bdd_variable_set, value)
            .or(&self.encode_one(bdd_variable_set, value))
    }
}

// todo maybe split into: SymbolicEncoding, SymbolicDomain;
