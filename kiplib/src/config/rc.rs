use std::{collections::HashMap, fs, path::Path};

#[derive(Debug)]
pub struct Config {
    pub sections: HashMap<String, Section>,
}

impl IntoIterator for Config {
    type Item = (String, Section);

    type IntoIter = std::collections::hash_map::IntoIter<String, Section>;

    fn into_iter(self) -> Self::IntoIter {
        self.sections.into_iter()
    }
}

impl<'a> IntoIterator for &'a Config {
    type Item = (&'a String, &'a Section);

    type IntoIter = std::collections::hash_map::Iter<'a, String, Section>;

    fn into_iter(self) -> Self::IntoIter {
        self.sections.iter()
    }
}

impl<'a> IntoIterator for &'a mut Config {
    type Item = (&'a String, &'a mut Section);

    type IntoIter = std::collections::hash_map::IterMut<'a, String, Section>;

    fn into_iter(self) -> Self::IntoIter {
        self.sections.iter_mut()
    }
}

impl Config {
    pub fn get_sections<T>(&self, section: T) -> Option<&Section>
    where
        T: AsRef<str>,
    {
        self.sections.get(section.as_ref())
    }
}

#[derive(Debug)]
pub struct Section {
    pub entries: HashMap<String, Value>,
}

impl IntoIterator for Section {
    type Item = (String, Value);
    type IntoIter = std::collections::hash_map::IntoIter<String, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl<'a> IntoIterator for &'a Section {
    type Item = (&'a String, &'a Value);
    type IntoIter = std::collections::hash_map::Iter<'a, String, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

impl<'a> IntoIterator for &'a mut Section {
    type Item = (&'a String, &'a mut Value);
    type IntoIter = std::collections::hash_map::IterMut<'a, String, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter_mut()
    }
}

#[derive(Debug)]
pub enum Value {
    Literal(String),
    Bang(String),
    Question(String),
    At(String),
}

impl Value {
    pub fn parse(s: &str) -> Value {
        match s.chars().next() {
            Some('!') => Value::Bang(s[1..].to_string()),
            Some('?') => Value::Question(s[1..].to_string()),
            Some('@') => Value::At(s[1..].to_string()),
            _ => Value::Literal(s.to_string()),
        }
    }
}

pub fn read_config<T>(path: T) -> Result<Config, std::io::Error>
where
    T: AsRef<Path>,
{
    let config_string: String = fs::read_to_string(path).expect("failed to read config");

    Ok(split_sections(config_string))
}

fn split_sections(config_string: String) -> Config {
    let mut current_section: Option<String> = None;
    let mut sections: HashMap<String, Section> = HashMap::new();
    let trim = config_string.trim();

    for line in trim.lines() {
        if line.is_empty() {
            continue;
        }

        if line.starts_with("    ") {
            let line_trimmed = line.trim();
            if let Some((ident, value)) = line_trimmed.split_once(' ') {
                if let Some(s_name) = &current_section {
                    if let Some(section) = sections.get_mut(s_name) {
                        let key = ident.trim_matches('"').to_string();
                        let value = Value::parse(value.trim().trim_matches('"'));

                        section.entries.insert(key, value);
                    }
                }
            } else {
                todo!();
            };
            continue;
        } else if let Some((left, _)) = line.split_once(':') {
            let name = left.to_string();

            sections.entry(name.clone()).or_insert_with(|| Section {
                entries: HashMap::new(),
            });

            current_section = Some(name);

            continue;
        }
    }

    Config { sections }
}
