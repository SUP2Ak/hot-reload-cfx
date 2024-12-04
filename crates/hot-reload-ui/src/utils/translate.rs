use std::collections::HashMap;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::sync::RwLock;
use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    English,
    French,
}

impl Language {
    fn as_file_name(&self) -> &str {
        match self {
            Language::English => "en",
            Language::French => "fr",
        }
    }
}

impl Default for Language {
    fn default() -> Self {
        Language::English
    }
}

static TRANSLATIONS: Lazy<RwLock<HashMap<Language, HashMap<String, String>>>> = 
    Lazy::new(|| {
        let mut map = HashMap::new();
        if let Ok(en_trans) = load_language_file(Language::English) {
            map.insert(Language::English, en_trans);
        }
        RwLock::new(map)
    });

fn load_language_file(lang: Language) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let file_path = Path::new("locales").join(format!("{}.json", lang.as_file_name()));
    let content = fs::read_to_string(&file_path)?;
    let json: Value = serde_json::from_str(&content)?;
    
    let mut translations = HashMap::new();
    if let Value::Object(map) = json {
        for (key, value) in map {
            if let Value::String(text) = value {
                translations.insert(key, text);
            }
        }
    }
    
    Ok(translations)
}

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
        if language == Language::English {
            self.current_language = language;
            return Ok(());
        }

        let mut translations = TRANSLATIONS.write().unwrap();
        if !translations.contains_key(&language) {
            if let Ok(trans) = load_language_file(language) {
                translations.insert(language, trans);
            } else {
                return Err("Impossible de charger le fichier de langue".into());
            }
        }

        self.current_language = language;
        Ok(())
    }

    pub fn get_language(&self) -> Language {
        self.current_language
    }

    pub fn translate(&self, key: &str) -> String {
        let translations = TRANSLATIONS.read().unwrap();
        
        if let Some(trans) = translations.get(&self.current_language) {
            if let Some(text) = trans.get(key) {
                return text.clone();
            }
        }

        if self.current_language != Language::English {
            if let Some(en_trans) = translations.get(&Language::English) {
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