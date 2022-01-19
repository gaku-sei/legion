//! The runtime server is the portion of the Legion Engine that runs off runtime
//! data to simulate a world. It is tied to the lifetime of a runtime client.
//!
//! * Tracking Issue: [legion/crate/#xx](https://github.com/legion-labs/legion/issues/xx)
//! * Design Doc: [legion/book/project-resources](/book/todo.html)

// crate-specific lint exceptions:
//#![allow()]

use runtime_srv::{build_runtime, start_runtime};

fn main() {
    let mut app = build_runtime(
        Some("runtime_srv.project_dir"),
        "test/sample-data",
        // should map to the runtime_entity generated by
        // (1c0ff9e497b0740f,45d44b64-b97c-486d-a0d7-151974c28263)|1d9ddd99aad89045 --
        // check output.index
        "(1d9ddd99aad89045,af7e6ef0-c271-565b-c27a-b8cd93c3546a)",
    );

    start_runtime(&mut app);
}
