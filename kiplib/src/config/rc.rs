use core::fmt;
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
    pub fn get_section<T>(&self, section: T) -> Option<&Section>
    where
        T: AsRef<str>,
    {
        self.sections.get(section.as_ref())
    }
}

#[derive(Debug)]
pub struct Section {
    pub entries: HashMap<String, Value>,
    pub scalars: Vec<Value>,
}

impl Section {
    pub fn empty() -> Self {
        Section {
            entries: HashMap::new(),
            scalars: Vec::new(),
        }
    }
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

#[derive(Debug, Clone)]
pub enum Tagged {
    Literal(String),
    Bang(String),
    Question(String),
    At(String),
}

impl fmt::Display for Tagged {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tagged::Literal(s) => write!(f, "{s}"),
            Tagged::Bang(s) => write!(f, "!{s}"),
            Tagged::Question(s) => write!(f, "?{s}"),
            Tagged::At(s) => write!(f, "@{s}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Value(pub Vec<Tagged>);

impl Value {
    pub fn iter(&self) -> std::slice::Iter<'_, Tagged> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a Value {
    type Item = &'a Tagged;
    type IntoIter = std::slice::Iter<'a, Tagged>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl IntoIterator for Value {
    type Item = Tagged;
    type IntoIter = std::vec::IntoIter<Tagged>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, item) in self.0.iter().enumerate() {
            if i != 0 {
                write!(f, " ")?;
            }
            write!(f, "{}", item)?;
        }
        Ok(())
    }
}

impl FromIterator<Tagged> for Value {
    fn from_iter<I: IntoIterator<Item = Tagged>>(iter: I) -> Self {
        Value(iter.into_iter().collect())
    }
}

impl Tagged {
    fn parse_value(s: &str) -> Value {
        s.split_whitespace()
            .map(|token| match token.chars().next() {
                Some('@') => Tagged::At(token[1..].to_string()),
                Some('!') => Tagged::Bang(token[1..].to_string()),
                Some('?') => Tagged::Question(token[1..].to_string()),
                _ => Tagged::Literal(token.to_string()),
            })
            .collect()
    }

    // pub fn parse(s: &str) -> Tagged {
    //     match s.chars().next() {
    //         Some('!') => Tagged::Bang(s[1..].to_string()),
    //         Some('?') => Tagged::Question(s[1..].to_string()),
    //         Some('@') => Tagged::At(s[1..].to_string()),
    //         _ => Tagged::Literal(s.to_string()),
    //     }
    // }
}

pub fn read_config<T>(path: T) -> Result<Config, std::io::Error>
where
    T: AsRef<Path>,
{
    let config_string: String = fs::read_to_string(path)?;

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

            let Some(s_name) = &current_section else {
                continue;
            };
            let Some(section) = sections.get_mut(s_name) else {
                continue;
            };

            if let Some(value) = line_trimmed.strip_prefix('+') {
                let parsed = Tagged::parse_value(value);
                section.scalars.push(parsed);
                continue;
            }

            if let Some((ident, value)) = line_trimmed.split_once(' ') {
                let key = ident.trim_matches('"').to_string();
                let parsed = Tagged::parse_value(value);
                section.entries.insert(key, parsed);
            }

            continue;
        } else if let Some((left, _)) = line.split_once(':') {
            let name = left.to_string();

            sections.entry(name.clone()).or_insert_with(|| Section {
                entries: HashMap::new(),
                scalars: Vec::new(),
            });

            current_section = Some(name);

            continue;
        }
    }

    Config { sections }
}
