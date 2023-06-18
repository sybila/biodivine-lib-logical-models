use std::marker::PhantomData;

use biodivine_lib_bdd as bdd;

/// represents arity of a multi-valued variable in an MVDD.<br>
/// lowest allowed arity is 2 corresponding to a boolean variable.<br>
/// the domain of a variable with arity `n` is `{0, 1, ..., n-1}`.<br>
#[derive(Clone)]
pub struct Arity(u8);  // todo decide size; ie eg the original Bdd lib uses u16 iirc

impl Arity {
    /// creates a new Arity with the given arity.<br>
    /// Panics if arity is less than 2.<br>
    pub fn new(arity: u8) -> Self {
        if arity < 2 {
            panic!("Arity must be at least 2, but {} was provided.", arity);
        }

        Self(arity)
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

pub trait MvRepr {
    fn encode(arity: &Arity) -> Vec<bdd::BddVariable>;
}

pub struct MvReprVanHam;
impl MvRepr for MvReprVanHam {
    fn encode(arity: &Arity) -> Vec<bdd::BddVariable> {
        unimplemented!("MvReprVanHam::encode; arity: {}", arity.value())
    }
}

pub struct MvReprGrayCode;
impl MvRepr for MvReprGrayCode {
    fn encode(arity: &Arity) -> Vec<bdd::BddVariable> {
        unimplemented!("MvReprGrayCode::encode; arity: {}", arity.value())
    }
}

pub struct MvddVariable<Repr: MvRepr> {
    repr: PhantomData<Repr>, // til abt PhantomData
    arity: Arity,
    phantom: Vec<bdd::BddVariable>
}

impl<Repr: MvRepr> MvddVariable<Repr> {
    pub fn new(arity: Arity) -> Self {
        let bdd_repr = Repr::encode(&arity);

        Self {
            repr: PhantomData,
            arity,
            phantom: bdd_repr,
        }
    }

    pub fn arity(&self) -> &Arity {
        &self.arity
    }

    pub fn bdd_repr(&self) -> &Vec<bdd::BddVariable> {
        &self.phantom
    }
}

fn stuff() {
    let arity = Arity::new(3);
    let value = arity.value();
}
