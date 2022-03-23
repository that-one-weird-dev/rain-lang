use core::{Engine, EngineBuildSource, parser::{ModuleLoader, ModuleImporter}, LangError};
use std::sync::Arc;
use common::module::{Module, ModuleUID, ModuleIdentifier, ModuleMetadata};
use crate::{module::WasmModule, external_module::WasmExternalModule, errors::UNEXPECTED_ERROR, build};

pub struct WasmEngine {
    module_loader: ModuleLoader,
}

impl Engine for WasmEngine {
    type Module = WasmModule;
    type ExternalModule = WasmExternalModule;

    fn load_module(&mut self, identifier: impl Into<String>, importer: &impl ModuleImporter) -> Result<ModuleUID, LangError> {
        let (uid, _) = self
            .module_loader()
            .load_module(&ModuleIdentifier(identifier.into()), importer)?;

        Ok(uid)
    }

    fn insert_module(&mut self, _module: Arc<Module>) -> Result<(), LangError> {
        Ok(())
    }

    fn module_loader(&mut self) -> &mut ModuleLoader {
        &mut self.module_loader
    }

    fn insert_external_module(&mut self, module: Self::ExternalModule) {
        self.module_loader
            .insert_module(module.uid, Module {
                uid: module.uid,
                metadata: ModuleMetadata {
                    definitions: module.definitions,
                },
                imports: Vec::new(),
                functions: Vec::new(),
                variables: Vec::new(),
            });
    }

    fn new() -> Self {
        Self {
            module_loader: ModuleLoader::new(),
        }
    }
}

impl EngineBuildSource for WasmEngine {
    fn build_module_source(&self, uid: ModuleUID) -> Result<Vec<u8>, LangError> {
        let module = match self.module_loader.get_module(uid) {
            Some(module) => module,
            None => return Err(LangError::new_runtime(UNEXPECTED_ERROR.to_string()))
        };

        build::build_module(module)
    }
}