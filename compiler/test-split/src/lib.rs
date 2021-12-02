// BEGIN - Legion Labs lints v0.6
// do not change or add/remove here, but one can add exceptions after this section
#![deny(unsafe_code)]
#![warn(future_incompatible, nonstandard_style, rust_2018_idioms)]
// Rustdoc lints
#![warn(
    rustdoc::broken_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::private_intra_doc_links
)]
// Clippy pedantic lints, treat all as warnings by default, add exceptions in allow list
#![warn(clippy::pedantic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::unreadable_literal,
    clippy::unseparated_literal_suffix
)]
// Clippy nursery lints, still under development
#![warn(
    clippy::debug_assert_with_mut_call,
    clippy::disallowed_method,
    clippy::disallowed_type,
    clippy::fallible_impl_from,
    clippy::imprecise_flops,
    clippy::mutex_integer,
    clippy::path_buf_push_overwrite,
    clippy::string_lit_as_bytes,
    clippy::use_self,
    clippy::useless_transmute
)]
// Clippy restriction lints, usually not considered bad, but useful in specific cases
#![warn(
    clippy::dbg_macro,
    clippy::exit,
    clippy::float_cmp_const,
    clippy::map_err_ignore,
    clippy::mem_forget,
    clippy::missing_enforced_import_renames,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::string_to_string,
    clippy::todo,
    clippy::unimplemented,
    clippy::verbose_file_reads
)]
// END - Legion Labs lints v0.6
// crate-specific exceptions:
#![allow()]

use std::env;

use legion_data_compiler::{
    compiler_api::{
        CompilationOutput, CompilerContext, CompilerDescriptor, CompilerError, DATA_BUILD_VERSION,
    },
    compiler_utils::hash_code_and_data,
};
use legion_data_offline::resource::ResourceProcessor;
use legion_data_runtime::Resource;

pub static COMPILER_INFO: CompilerDescriptor = CompilerDescriptor {
    name: env!("CARGO_CRATE_NAME"),
    build_version: DATA_BUILD_VERSION,
    code_version: "1",
    data_version: "1",
    transform: &(
        multitext_resource::MultiTextResource::TYPE,
        text_resource::TextResource::TYPE,
    ),
    compiler_hash_func: hash_code_and_data,
    compile_func: compile,
};

fn compile(mut context: CompilerContext<'_>) -> Result<CompilationOutput, CompilerError> {
    let resources = context
        .take_registry()
        .add_loader::<multitext_resource::MultiTextResource>()
        .add_loader::<text_resource::TextResource>()
        .create();

    let resource =
        resources.load_sync::<multitext_resource::MultiTextResource>(context.source.resource_id());
    let resource = resource.get(&resources).unwrap();

    let source_text_list = resource.text_list.clone();

    let mut output = CompilationOutput {
        compiled_resources: vec![],
        resource_references: vec![],
    };

    let mut proc = text_resource::TextResourceProc {};

    for (index, content) in source_text_list.iter().enumerate() {
        let output_resource = text_resource::TextResource {
            content: content.clone(),
        };

        let mut bytes = vec![];

        let _nbytes = proc
            .write_resource(&output_resource, &mut bytes)
            .map_err(CompilerError::ResourceWriteFailed)?;

        let asset = context.store(
            &bytes,
            context.target_unnamed.new_named(&format!("text_{}", index)),
        )?;

        output.compiled_resources.push(asset);
    }

    Ok(output)
}