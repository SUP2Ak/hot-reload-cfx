use std::collections::HashMap;
use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    English,
    French,
}

impl Default for Language {
    fn default() -> Self {
        Language::English
    }
}

static TRANSLATIONS: Lazy<HashMap<Language, HashMap<String, String>>> = 
    Lazy::new(|| {
        let mut map = HashMap::new();
        let en_json = include_str!("../../../../locales/en.json");
        let fr_json = include_str!("../../../../locales/fr.json");

        if let Ok(en_trans) = serde_json::from_str(en_json) {
            map.insert(Language::English, en_trans);
        }
        if let Ok(fr_trans) = serde_json::from_str(fr_json) {
            map.insert(Language::French, fr_trans);
        }
        
        map
    });

#[derive(Debug, Clone)]
pub struct Translator {
    current_language: Language,
}

impl Default for Translator {
    fn default() -> Self {
        Self {
            current_language: Language::English,
        }
    }
}

impl Translator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_language(&mut self, language: Language) -> Result<(), Box<dyn std::error::Error>> {
        if TRANSLATIONS.contains_key(&language) {
            self.current_language = language;
            Ok(())
        } else {
            Err("Langue non disponible".into())
        }
    }

    pub fn get_language(&self) -> Language {
        self.current_language
    }

    pub fn translate(&self, key: &str) -> String {
        if let Some(trans) = TRANSLATIONS.get(&self.current_language) {
            if let Some(text) = trans.get(key) {
                return text.clone();
            }
        }

        if self.current_language != Language::English {
            if let Some(en_trans) = TRANSLATIONS.get(&Language::English) {
                if let Some(text) = en_trans.get(key) {
                    return text.clone();
                }
            }
        }

        key.to_string()
    }

    pub fn t(&self, key: &str) -> String {
        self.translate(key)
    }

    pub fn t_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut message = self.t(key).to_string();
        for (arg_key, arg_value) in args {
            message = message.replace(&format!("{{{}}}", arg_key), arg_value);
        }
        message
    }
    
    pub fn available_languages() -> Vec<Language> {
        vec![Language::English, Language::French]
    }
}

#[macro_export]
macro_rules! t {
    ($translator:expr, $key:expr) => {
        $translator.translate($key)
    };
}