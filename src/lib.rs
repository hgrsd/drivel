#[macro_use]
extern crate lazy_static;

mod infer;
mod infer_string;
mod produce;
mod schema;

pub use infer::*;
pub use produce::produce;
pub use schema::*;
