use std::env;
use std::fs::{File, read_to_string};
use std::io::Write;
use std::path::PathBuf;
use common::constants::CORE_MODULE_ID;
use common::module::{ModuleIdentifier, ModuleUID};
use wasm::engine::WasmEngine;
use crate::{Args, Engine, EngineBuildSource, ReplImporter};
use crate::config::Config;

pub fn build(args: Args) -> anyhow::Result<()> {
    let config_str = read_to_string(args.module)?;
    let config = serde_json::from_str::<Config>(config_str.as_str())?;

    // Creating the engine
    let mut engine = WasmEngine::new();

    let importer = ReplImporter {
        src_dir: PathBuf::from(&config.src_dir),
    };

    // Loading core lib
    engine.module_loader()
        .load_module_with_source(
            ModuleIdentifier(CORE_MODULE_ID.to_string()),
            ModuleUID::from_string(CORE_MODULE_ID.to_string()),
            &include_str!("../../core_lib/lib.rn").to_string(),
            &importer,
        )?;

    // Creating the module from the source file
    let module = engine
        .load_module(config.main, &importer)?;

    let wasm = engine.build_module_source(module)?;

    let path = env::current_dir()?.join(config.build_path);
    let mut file = File::create(&path)?;
    file.write_all(wasm.as_slice())?;

    println!("Build successful! Output file at {}", path.to_str().unwrap());

    Ok(())
}