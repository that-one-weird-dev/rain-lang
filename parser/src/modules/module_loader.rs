use anyhow::anyhow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use common::ast::types::ClassType;
use common::module::{Module, ModuleUID};
use common::errors::{LoadErrorKind, format_load, LangErrorFormat};
use common::module::ModuleIdentifier;
use tokenizer::tokenizer::Tokenizer;
use crate::modules::module_preparser::ModulePreParser;
use crate::modules::module_importer::ModuleImporter;
use crate::modules::module_parser::ModuleParser;
use common::ast::parsing_types::{ParsableFunctionType, ParsableType};
use common::parsable_types::ParsableModule;

// TODO: Move this to the core crate

/// This handles the loading and dependency loading of modules
pub struct ModuleLoader {
    modules: RefCell<HashMap<ModuleUID, Arc<Module>>>,
}

impl ModuleLoader {
    pub fn new() -> Self {
        Self {
            modules: RefCell::new(HashMap::new()),
        }
    }

    pub fn insert_module(&mut self, uid: ModuleUID, module: Arc<Module>) {
        self.modules
            .borrow_mut()
            .insert(uid, module);
    }

    pub fn load_module_with_source(&mut self, id: ModuleIdentifier, uid: ModuleUID, source: &String, importer: &impl ModuleImporter)
        -> anyhow::Result<(Arc<Module>, Vec<Arc<Module>>)>
    {
        let tokens = match Tokenizer::tokenize(&source) {
            Ok(tokens) => tokens,
            Err(err) => return Err(err.format(&source)),
        };
        let parsable_module = match ModulePreParser::prepare_module(tokens, id, uid) {
            Ok(module) => Arc::new(module),
            Err(err) => return Err(err.format(&source))
        };
        let parser = self.create_parser(parsable_module.clone(), importer)?;

        // Loading the main module
        let module = match parser.parse_module(uid, importer) {
            Ok(module) => Arc::new(module),
            Err(err) => return Err(err.format(&source)),
        };

        self.modules
            .borrow_mut()
            .insert(uid, module.clone());

        // Loading all the dependencies
        let mut dependencies = Vec::new();

        for import_uid in &module.imports {
            let module = match parser.parse_module(*import_uid, importer) {
                Ok(module) => Arc::new(module),
                Err(err) => return Err(err.format(&source)),
            };

            dependencies.push(module.clone());

            self.modules
                .borrow_mut()
                .insert(*import_uid, module);
        }

        Ok((module, dependencies))
    }

    pub fn load_module(&mut self, id: &ModuleIdentifier, importer: &impl ModuleImporter) -> anyhow::Result<(Arc<Module>, Vec<Arc<Module>>)> {
        let uid = match importer.get_unique_identifier(id) {
            Some(uid) => uid,
            None => return Err(anyhow!(format_load(LoadErrorKind::ModuleNotFound(id.0.clone()))))
        };

        // If cached then simply return
        if let Some(module) = self.modules.borrow().get(&uid) {
            return Ok((module.clone(), Vec::new()))
        }

        let source = match importer.load_module(id) {
            Some(source) => source,
            None => return Err(anyhow!(format_load(LoadErrorKind::LoadModuleError(id.0.clone()))))
        };

        self.load_module_with_source(id.clone(), uid, &source, importer)
    }

    // pub fn load_declaration_module(
    //     &mut self,
    //     id: &ModuleIdentifier,
    //     module_id: &ModuleIdentifier,
    //     importer: &impl ModuleImporter
    // ) -> anyhow::Result<(ModuleUID, Option<Arc<Module>>)> {
    //     let module_uid = ModuleUID::from_string(module_id.0.clone());
    //
    //     // If cached then simply return
    //     if self.modules.borrow().contains_key(&module_uid) {
    //         return Ok((module_uid, None))
    //     }
    //
    //     let source = match importer.load_module(id, true) {
    //         Some(source) => source,
    //         None => return Err(anyhow!(format_load(LoadErrorKind::LoadModuleError(id.0.clone()))))
    //     };
    //
    //     let res = self.load_declaration_module_with_source(module_id.clone(), module_uid, &source, importer);
    //
    //     match res {
    //         Ok(res) => Ok((module_uid, Some(res))),
    //         Err(err) => Err(anyhow!(format_error(&source, err))),
    //     }
    // }

    fn create_parser(&self, module: Arc<ParsableModule>, importer: &impl ModuleImporter) -> anyhow::Result<ModuleParser> {
        let mut modules = Vec::new();

        modules.push(module.clone());

        self.load_imports(&mut modules, &module, importer)?;

        Ok(ModuleParser::new(self, modules))
    }

    fn load_imports(
        &self,
        vec: &mut Vec<Arc<ParsableModule>>,
        module: &Arc<ParsableModule>,
        importer: &impl ModuleImporter,
    ) -> anyhow::Result<()> {

        for import in &module.imports {
            let uid = match importer.get_unique_identifier(import) {
                Some(uid) => uid,
                None => return Err(anyhow!(format_load(LoadErrorKind::ModuleNotFound(import.0.clone())))),
            };

            if let Some(module) = self.modules.borrow().get(&uid) {
                vec.push(module.parsable_module.clone());
                continue
            }

            let source = match importer.load_module(&import) {
                Some(source) => source,
                None => return Err(anyhow!(format_load(LoadErrorKind::LoadModuleError(import.0.clone()))))
            };
            let tokens = Tokenizer::tokenize(&source)?;

            let parsable_module = match ModulePreParser::prepare_module(tokens, import.clone(), uid) {
                Ok(module) => Arc::new(module),
                Err(err) => return Err(err.format(&source))
            };

            self.load_imports(vec, &parsable_module, importer)?;

            vec.push(parsable_module);
        }

        Ok(())
    }

    pub fn modules(&self) -> Vec<Arc<Module>> {
        self.modules
            .borrow()
            .iter()
            .map(|(_, module)| module.clone())
            .collect()
    }

    pub fn get_module(&self, uid: ModuleUID) -> Option<Arc<Module>> {
        self.modules
            .borrow()
            .get(&uid)
            .cloned()
    }
}

pub enum GlobalDeclarationKind {
    Var(ParsableType),
    Func(ParsableFunctionType),
    Class(Arc<ClassType>),
}