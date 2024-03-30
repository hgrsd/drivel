#[macro_use]
extern crate lazy_static;

mod infer;
mod schema;

pub use infer::infer_schema;
pub use schema::*;
