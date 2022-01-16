use std::{collections::HashMap, sync::{Arc, Mutex}};

use crate::{lang_value::LangValue, convert_values::ConvertLangValue};


#[derive(Clone)]
pub struct LangObject {
    fields: Arc<Mutex<HashMap<String, LangValue>>>,
}

impl LangObject {
    
    pub fn new() -> Self {
        Self {
            fields: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub fn from_map(map: HashMap<String, LangValue>) -> Self {
        Self {
            fields: Arc::new(Mutex::new(map))
        }
    }

    pub(crate) fn get(&self, name: &String) -> LangValue  {
        match self.fields.lock() {
            Ok(map) => 
                match map.get(name) {
                    Some(value) => value.clone(),
                    None => LangValue::Nothing,
                },
            Err(_) => LangValue::Nothing,
        }
    }
    
    pub(crate) fn set(&mut self, name: String, value: LangValue) {
        match self.fields.lock() {
            Ok(mut map) => { map.insert(name, value); () },
            Err(_) => {},
        };
    }
    
    pub fn get_field<R: ConvertLangValue>(&self, name: &str) -> Option<R> {
        let value = self.get(&name.to_string());
        
        R::into(&value)
    }
    
    pub fn set_field<V: ConvertLangValue>(&mut self, name: &str, value: V) {
        self.set(name.to_string(), V::from(value));
    }
    
    pub fn len(&self) -> usize {
        match self.fields.lock() {
            Ok(value) => value.len(),
            Err(_) => 0,
        }
    }
}