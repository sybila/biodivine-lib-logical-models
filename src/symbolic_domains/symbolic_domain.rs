use biodivine_lib_bdd::{Bdd, BddVariable, BddVariableSet, BddVariableSetBuilder};

pub trait SymbolicDomain<T> {
    // todo in general, no need to enforce there should be a `max_value` -> ordering
    // todo vs want to somehow specify which values are allowed in this domain
    fn new(builder: &mut BddVariableSetBuilder, name: &str, max_value: &T) -> Self;

    fn encode_one(&self, bdd_variable_set: &BddVariableSet, value: &T) -> Bdd;
    // todo here because the `and(unit_set)` is often forgotten
    fn encode_one_not(&self, bdd_variable_set: &BddVariableSet, value: &T) -> Bdd {
        self.encode_one(bdd_variable_set, value)
            .not()
            .and(&self.unit_collection(bdd_variable_set))
    }
    fn empty_collection(&self, bdd_variable_set: &BddVariableSet) -> Bdd;
    fn unit_collection(&self, bdd_variable_set: &BddVariableSet) -> Bdd;

    /// Like `encode_bits`, but for inspecting how the bits are encoded.
    /// The result of this function (for the same `value`) does not change
    /// between different calls within a single run of the program. It can,
    /// however, change between different runs of the program.
    fn encode_bits_inspect(&self, value: &T) -> Vec<bool>;

    /// For each possible value, returns a vector of bits that encode the value.
    ///
    /// Should not be used - the result may be very large. Is only for testing.
    fn _unit_set_bits_inspect(&self) -> Vec<Vec<bool>>;

    /// todo bind with `encode_bits_inspect`
    fn raw_bdd_variables(&self) -> Vec<BddVariable>;
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

    fn cmp(lhs: &T, rhs: &T) -> std::cmp::Ordering;
}

// todo maybe split into: SymbolicEncoding, SymbolicDomain;
