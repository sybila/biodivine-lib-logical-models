pub mod variable_update_function {
    use crate::expression_components::expression::Expression;

    pub struct VariableUpdateFn<T> {
        pub input_vars_names: Vec<String>,
        pub target_var_name: String,
        pub terms: Vec<(T, Expression<T>)>,
        pub default: T,
    }

    impl<T> VariableUpdateFn<T> {
        pub fn new(
            input_vars_names: Vec<String>,
            target_var_name: String,
            terms: Vec<(T, Expression<T>)>,
            default: T,
        ) -> Self {
            Self {
                input_vars_names,
                target_var_name,
                terms,
                default,
            }
        }
    }
}
