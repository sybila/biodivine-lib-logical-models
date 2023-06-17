pub fn add(x: i32, y: i32) -> i32 {
    x + y
}

// expose the prototype module
mod prototype;
pub use prototype::*;

#[cfg(test)]
mod tests {
    use super::add;
    use super::foo;

    #[test]
    pub fn test() {
        assert_eq!(5, add(2, 3));
    }

    #[test]
    pub fn test_foo() {
        foo();
    }
}
