use legion_data_compiler::{
    compiler_api::{
        compiler_main, CompilationOutput, CompilerContext, CompilerDescriptor, CompilerError,
        DATA_BUILD_VERSION,
    },
    compiler_utils::path_id_to_binary,
    CompilerHash, Locale, Platform, Target,
};
use legion_data_runtime::Resource;
use std::{
    collections::hash_map::DefaultHasher,
    env,
    hash::{Hash, Hasher},
};

static COMPILER_INFO: CompilerDescriptor = CompilerDescriptor {
    name: env!("CARGO_CRATE_NAME"),
    build_version: DATA_BUILD_VERSION,
    code_version: "1",
    data_version: "1",
    transform: &(
        legion_graphics_offline::Material::TYPE,
        legion_graphics_runtime::Material::TYPE,
    ),
    compiler_hash_func: compiler_hash,
    compile_func: compile,
};

fn compiler_hash(
    code: &'static str,
    data: &'static str,
    _target: Target,
    _platform: Platform,
    _locale: &Locale,
) -> CompilerHash {
    let mut hasher = DefaultHasher::new();
    code.hash(&mut hasher);
    data.hash(&mut hasher);
    CompilerHash(hasher.finish())
}

fn compile(mut context: CompilerContext) -> Result<CompilationOutput, CompilerError> {
    let mut resources = context
        .take_registry()
        .add_loader::<legion_graphics_offline::Material>()
        .create();

    let resource =
        resources.load_sync::<legion_graphics_offline::Material>(context.source.content_id());

    let resource = resource.get(&resources).unwrap();

    let compiled_asset = {
        let mut c: Vec<u8> = vec![];
        c.append(&mut path_id_to_binary(&resource.albedo).to_le_bytes().to_vec());
        c.append(&mut path_id_to_binary(&resource.normal).to_le_bytes().to_vec());
        c.append(
            &mut path_id_to_binary(&resource.roughness)
                .to_le_bytes()
                .to_vec(),
        );
        c.append(
            &mut path_id_to_binary(&resource.metalness)
                .to_le_bytes()
                .to_vec(),
        );
        c
    };

    let asset = context.store(&compiled_asset, context.target_unnamed.clone())?;

    Ok(CompilationOutput {
        compiled_resources: vec![asset],
        resource_references: vec![],
    })
}

fn main() {
    std::process::exit(match compiler_main(env::args(), &COMPILER_INFO) {
        Ok(_) => 0,
        Err(_) => 1,
    });
}
