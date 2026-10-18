use crate::syntax::{Name, Object};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dict {
    entries: Vec<(Name, Object)>,
}

impl Dict {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<&Object> {
        self.entries.iter().find(|(name, _)| name.is(key)).map(|(_, value)| value)
    }

    pub fn get_name(&self, key: &Name) -> Option<&Object> {
        self.entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn insert(&mut self, key: impl Into<Name>, value: Object) {
        let key = key.into();
        match self.entries.iter_mut().find(|(name, _)| *name == key) {
            Some(entry) => entry.1 = value,
            None => self.entries.push((key, value)),
        }
    }

    pub fn insert_missing(&mut self, key: Name, value: Object) {
        if !self.entries.iter().any(|(name, _)| *name == key) {
            self.entries.push((key, value));
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<Object> {
        let index = self.entries.iter().position(|(name, _)| name.is(key))?;
        Some(self.entries.remove(index).1)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Name, &Object)> {
        self.entries.iter().map(|(name, value)| (name, value))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn name(&self, key: &str) -> Option<&Name> {
        self.get(key).and_then(Object::as_name)
    }

    pub fn int(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(Object::as_int)
    }

    pub fn has_type(&self, type_name: &str) -> bool {
        self.name("Type").is_some_and(|name| name.is(type_name))
    }
}

impl FromIterator<(Name, Object)> for Dict {
    fn from_iter<I: IntoIterator<Item = (Name, Object)>>(iter: I) -> Self {
        let mut dict = Dict::new();
        for (key, value) in iter {
            dict.insert(key, value);
        }
        dict
    }
}
