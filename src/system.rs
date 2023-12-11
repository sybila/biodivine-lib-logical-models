// todo move where it belongs
pub mod variable_update_function {
    use crate::expression_components::expression::Expression;

    #[derive(Debug)]
    pub struct UnprocessedVariableUpdateFn<T> {
        pub input_vars_names: Vec<String>,
        pub target_var_name: String,
        pub terms: Vec<(T, Expression<T>)>,
        pub default: T,
    }

    impl<T> UnprocessedVariableUpdateFn<T> {
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
