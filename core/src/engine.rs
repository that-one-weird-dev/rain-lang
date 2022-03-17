use common::ast::types::TypeKind;
use common::errors::LangError;
use common::module::ModuleUID;
use parser::modules::module_importer::ModuleImporter;
use parser::modules::module_loader::ModuleLoader;

use crate::{externals::ExternalType, module::EngineModule};


pub trait Engine
where
    Self: Sized,
{
    type Module: EngineModule<Engine = Self>;

    fn load_module<Importer: ModuleImporter>(&mut self, identifier: impl Into<String>) -> Result<ModuleUID, LangError>;

    fn global_types(&self) -> &Vec<(String, TypeKind)>;
    fn module_loader(&mut self) -> &mut ModuleLoader;
    fn insert_module(&mut self, module: Self::Module);

    fn new() -> Self;
}

pub trait EngineGetFunction<Args, R, Ret: InternalFunction<Args, R>> : Engine {
    fn get_function(&self, uid: ModuleUID, name: &str)
                    -> Option<Ret>;
}

pub trait InternalFunction<Args, R> {
    fn call(&self, args: Args) -> R;
}