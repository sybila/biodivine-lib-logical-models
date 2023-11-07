use biodivine_lib_logical_models::{BinaryIntegerDomain, GrayCodeIntegerDomain, PetriNetIntegerDomain, reachability_benchmark, UnaryIntegerDomain};

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let representation = args[1].clone();
    let sbml_path = args[2].clone();

    match representation.as_str() {
        "unary" => reachability_benchmark::<UnaryIntegerDomain>(sbml_path.as_str()),
        "binary" => reachability_benchmark::<BinaryIntegerDomain<u8>>(sbml_path.as_str()),
        "petri_net" => reachability_benchmark::<PetriNetIntegerDomain>(sbml_path.as_str()),
        "gray" | "grey" => reachability_benchmark::<GrayCodeIntegerDomain<u8>>(sbml_path.as_str()),
        _ => panic!("Unknown representation: {}.", representation),
    }
}