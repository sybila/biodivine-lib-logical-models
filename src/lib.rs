pub fn add(x: i32, y: i32) -> i32 {
    x + y
}

// expose the prototype module
mod prototype;
pub use prototype::*;

#[cfg(test)]
mod tests {
    use super::add;

    #[test]
    pub fn test() {
        assert_eq!(5, add(2, 3));
    }

    #[test]
    pub fn test_foo() {
        super::foo();
    }

    #[test]
    pub fn test_tutorial() {
        super::tutorial();
    }

    // #[test]
    // pub fn test_sol() {
    //     super::node_processing();
    // }

    #[test]
    pub fn test_sbml_model() {
        super::trying();
    }
}
