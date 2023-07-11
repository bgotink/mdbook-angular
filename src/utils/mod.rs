mod angular_workspace;
mod codegen;
mod path;

pub(crate) use angular_workspace::AngularWorkspace;
pub(crate) use codegen::{generate_angular_code, generated_rendered_code_block};
pub(crate) use path::path_to_root;
