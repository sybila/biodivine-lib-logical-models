use std::collections::HashSet;

use biodivine_lib_bdd::{
    Bdd, BddPartialValuation, BddVariable, BddVariableSet, BddVariableSetBuilder,
};

pub trait SymbolicDomain<T> {
    /// Encode the given `value` into the provided `BddPartialValuation`.
    ///
    /// *Contract:* This method only modifies the symbolic variables from
    /// `Self::symbolic_variables`. No other parts of the `BddPartialValuation` are affected.
    ///
    /// # Panics
    ///
    /// If and only if the value is not in the domain.
    fn encode_bits(&self, bdd_valuation: &mut BddPartialValuation, value: &T);

    /// Encode a single `value` into a `Bdd` which is satisfied for exactly this value
    /// and no other.
    ///
    /// *Contract:* The resulting BDD only uses variables from `Self::symbolic_variables`.
    fn encode_one(&self, variables: &BddVariableSet, value: &T) -> Bdd {
        let mut valuation = BddPartialValuation::empty();
        self.encode_bits(&mut valuation, value);
        variables.mk_conjunctive_clause(&valuation)
    }

    // todo here because the `and(unit_set)` is often forgotten
    fn encode_one_not(&self, bdd_variable_set: &BddVariableSet, value: &T) -> Bdd {
        self.encode_one(bdd_variable_set, value)
            .not()
            .and(&self.unit_collection(bdd_variable_set))
    }
    fn empty_collection(&self, bdd_variable_set: &BddVariableSet) -> Bdd;
    fn unit_collection(&self, bdd_variable_set: &BddVariableSet) -> Bdd;

    /// Like `encode_bits`, but for inspecting how the bits are encoded.
    ///
    /// The order of the bits is the same as the order of the variables in
    /// `Self::raw_bdd_variables()`.
    ///
    /// The result of this function (for the same `value`) does not change
    /// between different calls (with the same arguments) within a single
    /// run of the program. It can, however, change between different runs
    /// of the program.
    fn raw_bdd_variables_encode(&self, value: &T) -> Vec<bool> {
        let value_encoded_into_partial_valuation = {
            let something_to_create_a_partial_valuation_with = self
                .raw_bdd_variables()
                .into_iter()
                .map(|var| (var, true))
                .collect::<Vec<_>>();

            let mut partial_valuation =
                BddPartialValuation::from_values(&something_to_create_a_partial_valuation_with[..]);

            self.encode_bits(&mut partial_valuation, value);

            partial_valuation
        };

        let bdd_variables_and_their_bit = value_encoded_into_partial_valuation.to_values();

        let sorted = {
            let mut unsorted = bdd_variables_and_their_bit;
            unsorted.sort_unstable_by_key(|(var, _)| *var);
            unsorted
        };

        sorted.into_iter().map(|(_, bit)| bit).collect()
    }

    /// Returns the `BddVariable`s used to encode this domain, ordered by their index.
    ///
    /// In case when ordering is not important, use `raw_bdd_variables_unsorted`.
    fn raw_bdd_variables(&self) -> Vec<BddVariable> {
        let mut unsorted = self.raw_bdd_variables_unsorted();
        unsorted.sort_unstable();
        unsorted
    }
    fn raw_bdd_variables_unsorted(&self) -> Vec<BddVariable>;

    /// Decode a value from the provided `BddPartialValuation`.
    ///
    /// *Contract:* This method only reads the symbolic variables from `Self::symbolic_variables`.
    /// The result is undefined if `bdd_valuation` does not represent a value that is valid in
    /// the encoding implemented by this `SymbolicDomain` (i.e. if the valuation is not valid
    /// within the `Self::unit_collection` BDD object). In particular, the method can return
    /// any value or panic in such a scenario (though panics are preferred).
    fn decode_bits(&self, bdd_valuation: &BddPartialValuation) -> T;

    /// Decode a collection of values stored in a `Bdd`.
    ///
    /// *Contract:* The order of returned values can be arbitrary as long as it is deterministic.
    /// Typically, this will follow some kind of implicit order enforced by the encoding.
    fn decode_collection(&self, variables: &BddVariableSet, collection: &Bdd) -> Vec<T> {
        // This cumbersome piece of code eliminates all non-encoding variables from the `collection`
        // BDD and replaces them with a value `false`. These extra `false` entries can be then
        // skipped in the final iterator over all BDD valuations.

        // At the moment, this is likely quite slow. However, if this turns out to be the final
        // design, we should be able to implement this directly in `lib-bdd` much more efficiently.
        // Furthermore, explicitly decoding the whole set of values at once is mostly a "debug"
        // operation, or performed only for very small sets. So in the end, the performance here
        // might not even matter.

        let encoding_variables = self.raw_bdd_variables();
        let mut ignored_variables: HashSet<BddVariable> =
            variables.variables().into_iter().collect();
        ignored_variables.retain(|x| !encoding_variables.contains(x));
        let ignored_variables: Vec<BddVariable> = ignored_variables.into_iter().collect();
        let collection = collection.exists(&ignored_variables);
        let fixed_selection = ignored_variables
            .into_iter()
            .map(|it| (it, false))
            .collect::<Vec<_>>();
        let collection = collection.select(&fixed_selection);

        let mut encoded_bits = BddPartialValuation::empty();
        collection
            .sat_valuations()
            .map(|valuation| {
                for bit in &encoding_variables {
                    encoded_bits.set_value(*bit, valuation.value(*bit))
                }
                self.decode_bits(&encoded_bits)
            })
            .collect()
    }
}

pub trait SymbolicDomainOrd<T>: SymbolicDomain<T> {
    fn new(builder: &mut BddVariableSetBuilder, name: &str, max_value: &T) -> Self;
    /// Encodes the set of values that are strictly less than the given value.
    fn encode_lt(&self, bdd_variable_set: &BddVariableSet, exclusive_upper_bound: &T) -> Bdd;
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

    fn get_all_possible_values(&self) -> Vec<T>;
}

/// Implementation of a `SymbolicDomain` using unary integer encoding, i.e. each integer domain
/// `D = { 0 ... max }` is encoded using `max` symbolic variables.
///
/// In this encoding, to represent value `k \in D`, we set the values of the first `k` symbolic
/// variables to `true` and leave the remaining as `false`.
#[derive(Clone, Debug)]
pub struct UnaryIntegerDomain {
    /// invariant: sorted
    variables: Vec<BddVariable>, // todo maybe Rc<[BddVariable]>
}

// implementation author: Samuel Pastva
impl SymbolicDomain<u8> for UnaryIntegerDomain {
    fn encode_bits(&self, bdd_valuation: &mut BddPartialValuation, value: &u8) {
        if value > &(self.variables.len() as u8) {
            let vars = self
                .variables
                .iter()
                .map(|var| format!("{:?}", var))
                .collect::<Vec<_>>()
                .join(", ");

            panic!(
                "Value is too big for domain {}; value: {}, domain size: {}",
                vars,
                value,
                self.variables.len()
            )
        }

        self.variables.iter().enumerate().for_each(|(i, var)| {
            bdd_valuation.set_value(*var, i < (*value as usize));
        });
    }

    fn empty_collection(&self, bdd_variable_set: &BddVariableSet) -> Bdd {
        bdd_variable_set.mk_false()
    }

    fn unit_collection(&self, variables: &BddVariableSet) -> Bdd {
        // The only values that are correct in this encoding are values where `x_{k}` implies
        // `x_{k-1}` for all valid `k`. Following such condition, once a symbolic variable is
        // `true`, all "smaller" variables must be also `true`.

        // TODO:
        //  We might cache this value in the `SymbolicDomain` object so it does not need
        //  to be recomputed every time and we can just copy it instead.

        (1..self.variables.len()).fold(variables.mk_true(), |acc, k| {
            acc.and(
                &variables
                    .mk_var(self.variables[k])
                    .imp(&variables.mk_var(self.variables[k - 1])),
            )
        })
    }

    fn raw_bdd_variables(&self) -> Vec<BddVariable> {
        self.variables.clone() // already sorted
    }

    fn raw_bdd_variables_unsorted(&self) -> Vec<BddVariable> {
        self.raw_bdd_variables() // already the optimal performance
    }

    fn decode_bits(&self, bdd_valuation: &BddPartialValuation) -> u8 {
        // This method does not always check if the valuation is valid in the unary encoding, it
        // just picks the "simplest" interpretation of the given valuation. For increased safety,
        // we should check that that after the last "true" value, only "false" values follow.

        self.variables
            .iter()
            .enumerate()
            .find(|(_, var)| {
                !bdd_valuation
                    .get_value(**var)
                    .expect("var should be in the valuation")
            })
            .map(|(idx, _)| idx as u8)
            .unwrap_or(self.variables.len() as u8)
    }
}

impl SymbolicDomainOrd<u8> for UnaryIntegerDomain {
    fn new(builder: &mut BddVariableSetBuilder, name: &str, max_value: &u8) -> Self {
        let variables = (0..*max_value)
            .map(|var_idx| {
                let name = format!("{name}_v{}", var_idx + 1); // todo is there a reason for the +1?
                builder.make_variable(name.as_str())
            })
            .collect();

        Self { variables }
    }

    fn encode_lt(&self, bdd_variable_set: &BddVariableSet, exclusive_upper_bound: &u8) -> Bdd {
        (0..*exclusive_upper_bound).fold(self.empty_collection(bdd_variable_set), |acc, val| {
            acc.or(&self.encode_one(bdd_variable_set, &val))
        })

        // todo or maybe... (test this)
        // let not_upper_bound_bit =
        //     bdd_variable_set.mk_not_var(self.variables[*exclusive_uppper_bound as usize]);

        // self.unit_collection(bdd_variable_set)
        //     .and(&not_upper_bound_bit)
    }

    fn cmp(lhs: &u8, rhs: &u8) -> std::cmp::Ordering {
        lhs.cmp(rhs)
    }

    fn get_all_possible_values(&self) -> Vec<u8> {
        (0..=self.variables.len() as u8).collect() // notice the inclusive range; n values is represented by n-1 bdd variables
    }
}

#[derive(Debug)]
pub struct PetriNetIntegerDomain {
    /// invariant: sorted
    variables: Vec<BddVariable>,
}

impl SymbolicDomain<u8> for PetriNetIntegerDomain {
    fn encode_bits(&self, bdd_valuation: &mut BddPartialValuation, value: &u8) {
        if value > &(self.variables.len() as u8) {
            let vars = self
                .variables
                .iter()
                .map(|var| format!("{:?}", var))
                .collect::<Vec<_>>()
                .join(", ");

            panic!(
                "Value is too big for domain {}; value: {}, domain size: {}",
                vars,
                value,
                self.variables.len()
            )
        }

        self.variables
            .iter()
            .enumerate()
            .for_each(|(var_idx_within_sym_var, var)| {
                bdd_valuation.set_value(*var, var_idx_within_sym_var == (*value as usize));
            });
    }

    fn empty_collection(&self, bdd_variable_set: &BddVariableSet) -> Bdd {
        bdd_variable_set.mk_false()
    }

    fn unit_collection(&self, bdd_variable_set: &BddVariableSet) -> Bdd {
        bdd_variable_set.mk_sat_exactly_k(1, &self.variables)
    }

    fn raw_bdd_variables(&self) -> Vec<BddVariable> {
        self.variables.clone() // already sorted
    }

    fn raw_bdd_variables_unsorted(&self) -> Vec<BddVariable> {
        self.raw_bdd_variables() // already the optimal performance
    }

    fn decode_bits(&self, bdd_valuation: &BddPartialValuation) -> u8 {
        // This method does not always check if the valuation is valid in the unary encoding, it
        // just picks the "simplest" interpretation of the given valuation. For increased safety,
        // we should check that that after the only "true" value, only "false" values follow.

        self.variables
            .iter()
            .enumerate()
            .find(|(_, var)| {
                bdd_valuation
                    .get_value(**var)
                    .expect("var should be in the valuation")
            })
            .map(|(idx, _)| idx as u8)
            .expect("a valid value should be encoded by a \"true\" bit")
    }
}

impl SymbolicDomainOrd<u8> for PetriNetIntegerDomain {
    fn new(builder: &mut BddVariableSetBuilder, name: &str, max_value: &u8) -> Self {
        let variables = (0..=*max_value) // notice the inclusive range
            .map(|var_idx| {
                let name = format!("{name}_v{}", var_idx + 1);
                builder.make_variable(name.as_str())
            })
            .collect();

        Self { variables }
    }

    fn encode_lt(&self, bdd_variable_set: &BddVariableSet, exclusive_upper_bound: &u8) -> Bdd {
        (0..*exclusive_upper_bound).fold(self.empty_collection(bdd_variable_set), |acc, val| {
            acc.or(&self.encode_one(bdd_variable_set, &val))
        })
    }

    fn cmp(lhs: &u8, rhs: &u8) -> std::cmp::Ordering {
        lhs.cmp(rhs)
    }

    fn get_all_possible_values(&self) -> Vec<u8> {
        (0..self.variables.len() as u8).collect() // notice the exclusive range; n values is represented by n bdd variables
    }
}

#[derive(Debug)]
pub struct BinaryIntegerDomain<T> {
    /// invariant: sorted
    variables: Vec<BddVariable>,
    /// in older implementations, this used to be the `max_value`
    /// since we no longer require ordering, no `max_value` -> Bdd of all the possible values
    max_value: T, // todo mb, cannot implemnent BinaryIntegerDomain generically -> it must be SymbolicDomainOrd
}

impl SymbolicDomain<u8> for BinaryIntegerDomain<u8> {
    fn encode_bits(&self, bdd_valuation: &mut BddPartialValuation, value: &u8) {
        if value > &(self.max_value) {
            // this breaks the idea of SymbolicDomain being not bound to the ordering
            let vars = self
                .variables
                .iter()
                .map(|var| format!("{:?}", var))
                .collect::<Vec<_>>()
                .join(", ");

            panic!(
                "Value is too big for domain {}; value: {}, domain size: {}",
                vars, value, self.max_value
            )
        }

        self.variables.iter().enumerate().for_each(|(idx, var)| {
            bdd_valuation.set_value(*var, (value & (1 << idx)) != 0);
        })
    }

    fn empty_collection(&self, bdd_variable_set: &BddVariableSet) -> Bdd {
        bdd_variable_set.mk_false()
    }

    fn unit_collection(&self, bdd_variable_set: &BddVariableSet) -> Bdd {
        (0..=self.max_value).fold(bdd_variable_set.mk_false(), |acc, val| {
            acc.or(&self.encode_one(bdd_variable_set, &val))
        })
    }

    fn raw_bdd_variables(&self) -> Vec<BddVariable> {
        self.variables.clone() // already sorted
    }

    fn raw_bdd_variables_unsorted(&self) -> Vec<BddVariable> {
        self.raw_bdd_variables() // already the optimal performance
    }

    fn decode_bits(&self, bdd_valuation: &BddPartialValuation) -> u8 {
        let res = self
            .variables
            .iter()
            .enumerate()
            .fold(0, |acc, (idx, var)| {
                let bit = u8::from(
                    bdd_valuation
                        .get_value(*var)
                        .expect("bits of the value should be in the valuation"),
                );

                acc | (bit << idx)
            });

        if res > self.max_value {
            panic!(
                "invalid encoding; should not contain value greater than {}, but contains {}",
                self.max_value, res
            )
        }

        res
    }
}

impl SymbolicDomainOrd<u8> for BinaryIntegerDomain<u8> {
    fn new(builder: &mut BddVariableSetBuilder, name: &str, max_value: &u8) -> Self {
        let bit_count = 8 - max_value.leading_zeros();

        let variables = (0..bit_count)
            .map(|it| {
                let name = format!("{name}_v{}", it + 1);
                builder.make_variable(name.as_str())
            })
            .collect();

        Self {
            variables,
            max_value: *max_value,
        }
    }

    fn encode_lt(&self, bdd_variable_set: &BddVariableSet, exclusive_upper_bound: &u8) -> Bdd {
        (0..*exclusive_upper_bound).fold(self.empty_collection(bdd_variable_set), |acc, val| {
            acc.or(&self.encode_one(bdd_variable_set, &val))
        })
    }

    fn cmp(lhs: &u8, rhs: &u8) -> std::cmp::Ordering {
        lhs.cmp(rhs)
    }

    fn get_all_possible_values(&self) -> Vec<u8> {
        (0..=self.max_value).collect()
    }
}

#[derive(Debug)]
pub struct GrayCodeIntegerDomain<T> {
    /// invariant: sorted
    variables: Vec<BddVariable>,
    /// in older implementations, this used to be the `max_value`
    /// since we no longer require ordering, no `max_value` -> Bdd of all the possible values
    max_value: T, // todo same as in the case of BinaryIntegerDomain
}

impl GrayCodeIntegerDomain<u8> {
    fn binary_to_gray_code(n: u8) -> u8 {
        // magic
        n ^ (n >> 1)
    }

    fn gray_code_to_binary(n: u8) -> u8 {
        // magic II
        let mut n = n;
        let mut mask = n >> 1;
        while mask != 0 {
            n ^= mask;
            mask >>= 1;
        }
        n
    }
}

impl SymbolicDomain<u8> for GrayCodeIntegerDomain<u8> {
    fn encode_bits(&self, bdd_valuation: &mut BddPartialValuation, value: &u8) {
        if value > &(self.max_value) {
            // this breaks the idea of SymbolicDomain being not bound to the ordering
            let vars = self
                .variables
                .iter()
                .map(|var| format!("{:?}", var))
                .collect::<Vec<_>>()
                .join(", ");

            panic!(
                "Value is too big for domain {}; value: {}, domain size: {}",
                vars, value, self.max_value
            )
        }

        let gray_code = Self::binary_to_gray_code(*value);
        self.variables.iter().enumerate().for_each(|(idx, var)| {
            bdd_valuation.set_value(*var, (gray_code & (1 << idx)) != 0);
        })
    }

    fn empty_collection(&self, bdd_variable_set: &BddVariableSet) -> Bdd {
        bdd_variable_set.mk_false()
    }

    fn unit_collection(&self, bdd_variable_set: &BddVariableSet) -> Bdd {
        (0..=self.max_value).fold(bdd_variable_set.mk_false(), |acc, val| {
            acc.or(&self.encode_one(bdd_variable_set, &val))
        })
    }

    fn raw_bdd_variables(&self) -> Vec<BddVariable> {
        self.variables.clone() // already sorted
    }

    fn raw_bdd_variables_unsorted(&self) -> Vec<BddVariable> {
        self.raw_bdd_variables() // already the optimal performance
    }

    fn decode_bits(&self, bdd_valuation: &BddPartialValuation) -> u8 {
        let read_gray_code = self
            .variables
            .iter()
            .enumerate()
            .fold(0, |acc, (idx, var)| {
                let bit = u8::from(
                    bdd_valuation
                        .get_value(*var)
                        .expect("bits of the value should be in the valuation"),
                );

                acc | (bit << idx)
            });

        let res = Self::gray_code_to_binary(read_gray_code);

        if res > self.max_value {
            panic!(
                "invalid encoding; should not contain value greater than {}, but contains {}",
                self.max_value, res
            )
        }

        res
    }
}

impl SymbolicDomainOrd<u8> for GrayCodeIntegerDomain<u8> {
    fn new(builder: &mut BddVariableSetBuilder, name: &str, max_value: &u8) -> Self {
        let bit_count = 8 - max_value.leading_zeros();

        let variables = (0..bit_count)
            .map(|it| {
                let name = format!("{name}_v{}", it + 1);
                builder.make_variable(name.as_str())
            })
            .collect();

        Self {
            variables,
            max_value: *max_value,
        }
    }

    fn encode_lt(&self, bdd_variable_set: &BddVariableSet, exclusive_upper_bound: &u8) -> Bdd {
        (0..*exclusive_upper_bound).fold(self.empty_collection(bdd_variable_set), |acc, val| {
            acc.or(&self.encode_one(bdd_variable_set, &val))
        })
    }

    fn cmp(lhs: &u8, rhs: &u8) -> std::cmp::Ordering {
        lhs.cmp(rhs)
    }

    fn get_all_possible_values(&self) -> Vec<u8> {
        (0..=self.max_value).collect()
    }
}
