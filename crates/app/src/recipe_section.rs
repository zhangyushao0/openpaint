//! One table of a brush file (`recipe.rs`) and how its values are read, each into a typed
//! `RecipeError` naming the value's dotted path.

use toml::{Table, Value};

use crate::recipe::RecipeError;

pub(crate) fn missing(section: &Section<'_>, key: &str) -> RecipeError {
    RecipeError::Missing {
        field: section.field(key),
    }
}

pub(crate) fn out_of_range(section: &Section<'_>, key: &str) -> RecipeError {
    RecipeError::OutOfRange {
        field: section.field(key),
    }
}

/// One table of the file and its dotted path.
pub(crate) struct Section<'a> {
    pub(crate) table: &'a Table,
    pub(crate) path: &'static str,
}

impl<'a> Section<'a> {
    pub(crate) fn field(&self, key: &str) -> String {
        if self.path.is_empty() {
            key.to_owned()
        } else {
            format!("{}.{key}", self.path)
        }
    }

    fn wrong(&self, key: &str, expected: &'static str) -> RecipeError {
        RecipeError::WrongType {
            field: self.field(key),
            expected,
        }
    }

    /// Refuses any key not in `known`.
    pub(crate) fn only(&self, known: &[&str]) -> Result<(), RecipeError> {
        match self.table.keys().find(|key| !known.contains(&key.as_str())) {
            Some(key) => Err(RecipeError::Unknown {
                field: self.field(key),
            }),
            None => Ok(()),
        }
    }

    pub(crate) fn section(&self, key: &'static str) -> Result<Option<Section<'a>>, RecipeError> {
        match self.table.get(key) {
            None => Ok(None),
            Some(Value::Table(table)) => Ok(Some(Section { table, path: key })),
            Some(_) => Err(self.wrong(key, "a table")),
        }
    }

    pub(crate) fn required_section(&self, key: &'static str) -> Result<Section<'a>, RecipeError> {
        self.section(key)?.ok_or_else(|| missing(self, key))
    }

    pub(crate) fn string(&self, key: &str) -> Result<Option<&'a str>, RecipeError> {
        match self.table.get(key) {
            None => Ok(None),
            Some(Value::String(text)) => Ok(Some(text)),
            Some(_) => Err(self.wrong(key, "text")),
        }
    }

    pub(crate) fn number(&self, key: &str) -> Result<Option<f32>, RecipeError> {
        match self.table.get(key) {
            None => Ok(None),
            Some(value) => as_number(value)
                .map(Some)
                .ok_or_else(|| self.wrong(key, "a number")),
        }
    }

    pub(crate) fn required_number(&self, key: &str) -> Result<f32, RecipeError> {
        self.number(key)?.ok_or_else(|| missing(self, key))
    }

    pub(crate) fn integer(&self, key: &str) -> Result<Option<i64>, RecipeError> {
        match self.table.get(key) {
            None => Ok(None),
            Some(Value::Integer(value)) => Ok(Some(*value)),
            Some(_) => Err(self.wrong(key, "a whole number")),
        }
    }

    /// A list of `[x, y]` number pairs.
    pub(crate) fn curve(&self, key: &str) -> Result<Option<Vec<[f32; 2]>>, RecipeError> {
        let Some(value) = self.table.get(key) else {
            return Ok(None);
        };
        let wrong = || self.wrong(key, "a list of [x, y] number pairs");
        let Value::Array(points) = value else {
            return Err(wrong());
        };
        let pair = |point: &Value| match point {
            Value::Array(pair) => match pair.as_slice() {
                [x, y] => Some([as_number(x)?, as_number(y)?]),
                _ => None,
            },
            _ => None,
        };
        points
            .iter()
            .map(pair)
            .collect::<Option<Vec<_>>>()
            .map(Some)
            .ok_or_else(wrong)
    }
}

/// An integer or a float as `f32`; brush values are small, so nothing is lost that matters.
fn as_number(value: &Value) -> Option<f32> {
    match value {
        Value::Integer(value) => Some(*value as f32),
        Value::Float(value) => Some(*value as f32),
        _ => None,
    }
}
