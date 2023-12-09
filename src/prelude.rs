pub use crate::update::update_fn;
// pub use crate::prototype::update
pub mod old_update_fn {
    pub use crate::prototype::SmartSystemUpdateFn;
    pub use crate::prototype::SystemUpdateFn;
}

pub use crate::prototype::symbolic_domain as old_symbolic_domain;
pub use crate::symbolic_domains::symbolic_domain;

pub use crate::xml_parsing::utils::find_start_of;
