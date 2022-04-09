//! Content-store protocol library.
//!

#![allow(
    clippy::missing_errors_doc,
    clippy::doc_markdown,
    clippy::wildcard_imports,
    clippy::similar_names,
    clippy::use_self,
    clippy::return_self_not_must_use
)]

mod content_store {
    tonic::include_proto!("content_store");
}
pub use content_store::*;
