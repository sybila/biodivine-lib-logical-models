use std::collections::HashSet;
use biodivine_lib_bdd::{Bdd, BddPartialValuation, BddVariable, BddVariableSet, BddVariableSetBuilder};

/// Objects implementing `SymbolicDomain` serve as encoders/decoders for their associated type
/// `T` from/into `Bdd` objects.
///
/// TODO:
///     - We might consider swapping `&T` for `T` in a few places, as the `T` type will be usually
///     quite small (maybe even `Copy`). But this might not be that important.
///     - At the moment, methods returning a `Bdd` object take a `BddVariableSet` argument.
///     In some cases, this argument is unused, but it could be needed in some more specialized
///     implementations. Later, we should try to address this in `lib-bdd` (i.e. have a better
///     API for creating BDDs without having access to the `BddVariableSet`).
trait SymbolicDomain<T> {

    /// Encode the given `value` into the provided `BddPartialValuation`.
    fn encode_bits(&self, bdd_valuation: &mut BddPartialValuation, value: &T);

    /// Decode a value from the provided `BddPartialValuation`.
    ///
    /// The behaviour of this method is undefined if `bdd_valuation` does not represent a value
    /// that is valid in this encoding.
    fn decode_bits(&self, bdd_valuation: &BddPartialValuation) -> T;

    /// Returns the exact symbolic variables used in the encoding of this symbolic domain.
    ///
    /// Note that not all valuations of these variables must encode valid values.
    fn symbolic_variables(&self) -> Vec<BddVariable>;

    /// Returns the number of symbolic variables used in the encoding of this symbolic domain.
    fn symbolic_size(&self) -> usize;

    /// Create a `Bdd` which represents the empty set of encoded values.
    ///
    /// Typically, this is just the `false` BDD, but one may need to customize this sometimes.
    fn empty_collection(&self, variables: &BddVariableSet) -> Bdd;

    /// Create a `Bdd` which represents all values that can be encoded by this symbolic domain.
    ///
    /// Often, this is just `true`, but may need to be customized when the size of the symbolic
    /// domain is not such that all possible encodings are used.
    fn unit_collection(&self, variables: &BddVariableSet) -> Bdd;

    /* The rest are default implementations of several utility methods. */

    /// Encode a single `value` into a `Bdd` that is true for exactly this value and no other.
    fn encode_one(&self, variables: &BddVariableSet, value: &T) -> Bdd {
        let mut valuation = BddPartialValuation::empty();
        self.encode_bits(&mut valuation, value);
        variables.mk_conjunctive_clause(&valuation)
    }

    /// Interpret and decode the given `Bdd` as a single value.
    ///
    /// The `Bdd` object actually must be a proper encoding of a *single* value. This method
    /// will panic if the given `Bdd` is satisfied by multiple values from the symbolic domain.
    fn decode_one(&self, _variables: &BddVariableSet, value: &Bdd) -> T {
        assert!(value.is_clause());
        let clause = value.first_clause().unwrap();
        self.decode_bits(&clause)
    }

    /// Encode a collection of values into a `Bdd`.
    fn encode_collection(&self, variables: &BddVariableSet, collection: &[T]) -> Bdd {
        let clauses = collection.iter()
            .map(|v| {
                let mut valuation = BddPartialValuation::empty();
                self.encode_bits(&mut valuation, v);
                valuation
            })
            .collect::<Vec<_>>();

        variables.mk_dnf(&clauses)
    }

    /// Decode a collection of values stored in a `Bdd`.
    fn decode_collection(&self, variables: &BddVariableSet, collection: &Bdd) -> Vec<T> {

        // This cumbersome piece of code eliminates all non-encoding variables from the `collection`
        // BDD and replaces them with a value `false`. These extra `false` entries can be then
        // skipped in the final iterator over all BDD valuations.

        // At the moment, this is likely quite slow. However, if this turns out to be the final
        // design, we should be able to implement this directly in `lib-bdd` much more efficiently.
        // Furthermore, explicitly decoding the whole set of values at once is mostly a "debug"
        // operation, or performed only for very small sets. So in the end, the performance here
        // might not even matter.

        let encoding_variables = self.symbolic_variables();
        let mut ignored_variables: HashSet<BddVariable> = variables.variables().into_iter().collect();
        ignored_variables.retain(|x| !encoding_variables.contains(x));
        let ignored_variables: Vec<BddVariable> = ignored_variables.into_iter().collect();
        let collection = collection.exists(&ignored_variables);
        let fixed_selection = ignored_variables.into_iter().map(|it| (it, false)).collect::<Vec<_>>();
        let collection = collection.select(&fixed_selection);

        let mut encoded_bits = BddPartialValuation::empty();
        collection.sat_valuations()
            .map(|valuation| {
                for bit in &encoding_variables {
                    encoded_bits.set_value(*bit, valuation.value(*bit))
                }
                self.decode_bits(&encoded_bits)
            })
            .collect()
    }

}

/// Implementation of a `SymbolicDomain` using unary integer encoding, i.e. each integer domain `D`
/// is encoded using `max(D)` symbolic variables.
///
/// In this encoding, to represent value `k \in D`, we set the values of the first `k` symbolic
/// variables to `true` and leave the remaining as `false`.
struct UnaryIntegerDomain {
    variables: Vec<BddVariable>
}

impl UnaryIntegerDomain {
    /// Create a new `UnaryIntegerDomain`, such that the symbolic variables are allocated in the
    /// given `BddVariableSetBuilder`.
    pub fn new(builder: &mut BddVariableSetBuilder, name: &str, max_value: u8) -> UnaryIntegerDomain {
        let variables = (0..max_value).map(|it| {
                let name = format!("{name}_v{}", it + 1);
                builder.make_variable(name.as_str())
            })
            .collect::<Vec<_>>();
        UnaryIntegerDomain {
            variables
        }
    }
}

impl SymbolicDomain<u8> for UnaryIntegerDomain {
    fn encode_bits(&self, bdd_valuation: &mut BddPartialValuation, value: &u8) {
        for (i, var) in self.variables.iter().enumerate() {
            bdd_valuation.set_value(*var, i < (*value as usize));
        }
    }

    fn decode_bits(&self, bdd_valuation: &BddPartialValuation) -> u8 {
        // This method does not always check if the valuation is valid in the unary encoding, it
        // just picks the "simplest" interpretation of the given valuation. For increased safety,
        // we should check that that after the last "true" value, only "false" values follow.
        let mut result = 0;
        while result < self.variables.len() {
            let variable = self.variables[result];
            // This will panic if the variable value is not provided, which is reasonable because
            // in that case, the partial valuation is not a correctly encoded value.
            if !bdd_valuation.get_value(variable).unwrap() {
                return result as u8;
            }
            result += 1;
        }
        result as u8
    }

    fn symbolic_variables(&self) -> Vec<BddVariable> {
        self.variables.clone()
    }

    fn symbolic_size(&self) -> usize {
        self.variables.len()
    }

    fn empty_collection(&self, variables: &BddVariableSet) -> Bdd {
        variables.mk_false()
    }

    fn unit_collection(&self, variables: &BddVariableSet) -> Bdd {
        // The only values that are correct in this encoding are values where `x_{k}` implies
        // `x_{k-1}` for all valid `k`. Following such condition, once a variable is `true`,
        // all "smaller" variables must be also `true`.
        // TODO:
        //  We might cache this value in the `SymbolicDomain` object so it does not need
        //  to be recomputed every time.

        let mut true_set = variables.mk_true();
        for k in 1..self.variables.len() {
            let var_k = self.variables[k];
            let var_k_minus_one = self.variables[k - 1];
            let var_k = variables.mk_var(var_k);
            let var_k_minus_one = variables.mk_var(var_k_minus_one);
            let k_implies_k_minus_one = var_k.imp(&var_k_minus_one);
            true_set = true_set.and(&k_implies_k_minus_one);
        }
        true_set
    }
}

/// `GenericSymbolicStateSpace` is a collection of `SymbolicDomain` objects that together encode
/// the state space of a logical model.
///
/// This implementation allows using different encodings, which may lead to minor inefficiencies.
/// Alternatively, in the future we can also provide fully specialized structures relying on a
/// single encoding instead.
struct GenericStateSpaceDomain {

}

#[cfg(test)]
mod tests {
    use biodivine_lib_bdd::BddVariableSetBuilder;
    use crate::symbolic_domain::{SymbolicDomain, UnaryIntegerDomain};

    #[test]
    pub fn test_unary_domain_basic_properties() {
        let mut builder = BddVariableSetBuilder::new();
        let domain_1 = UnaryIntegerDomain::new(&mut builder, "x", 5);
        let domain_2 = UnaryIntegerDomain::new(&mut builder, "y", 14);
        let var_set = builder.build();

        assert_eq!(domain_1.symbolic_size(), 5);
        assert_eq!(domain_2.symbolic_size(), 14);
        assert_eq!(domain_1.symbolic_variables().len(), domain_1.symbolic_size());
        assert_eq!(domain_2.symbolic_variables().len(), domain_2.symbolic_size());

        let unit_set = domain_1.unit_collection(&var_set);
        let decoded_unit_set = domain_1.decode_collection(&var_set, &unit_set);
        assert_eq!(decoded_unit_set.len(), 6);

        let empty_set = domain_1.empty_collection(&var_set);
        let decoded_empty_set = domain_1.decode_collection(&var_set, &empty_set);
        assert_eq!(decoded_empty_set.len(), 0);
    }

}