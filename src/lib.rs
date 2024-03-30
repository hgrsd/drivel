#[macro_use]
extern crate lazy_static;

mod infer;
mod produce;
mod schema;

pub use infer::infer_schema;
pub use produce::produce;
pub use schema::*;
