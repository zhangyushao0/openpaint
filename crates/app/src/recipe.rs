//! Brush files: one brush as TOML, read into a `Brush` with typed errors (DECISIONS D07: fixed
//! stages with typed fields, no graph). Every key is checked; an unknown one is an error, so a
//! typo never passes silently.
//!
//! ```toml
//! name = "Pencil"                      # required, 1 to 40 characters
//!
//! [size]                               # required; diameters in canvas pixels
//! default = 5                          # 0 < min <= default <= max <= MAX_BRUSH_SIZE
//! min = 1
//! max = 40
//! pressure = [[0.0, 0.45], [1.0, 1.0]] # optional curves of [x, y] points, y in 0..=1;
//! speed = [[0.0, 1.0], [6.0, 0.7]]     # x is pressure 0..=1 or pixels per millisecond
//!
//! [opacity]                            # required
//! default = 1.0                        # in (0, 1]
//! pressure = [[0.0, 0.15], [1.0, 1.0]] # optional, as for size
//!
//! [shape]                              # optional
//! softness = 0.0                       # 0 hard edge .. 1 fades from the centre line
//!
//! [grain]                              # optional
//! strength = 0.65                      # 0 none .. 1
//! scale = 3                            # coarse cells in canvas pixels, 1 to 64
//!
//! [accumulate]                         # optional: density builds up along the stroke
//! tip = "flat"                         # "flat" (1 - d^8) or "bell" ((1 - d^2)^2)
//! flow = 3.1                           # density per radius travelled, times the opacity
//! max_density = 1.0                    # optional, in (0, 1], default 1
//!
//! [paper]                              # optional, under an accumulating flat tip
//! texture = "sketch"
//! contact = [[0.0, 0.4], [1.0, 0.95]]  # share of the paper touched, over pressure
//! scale = 4.5                          # texels per canvas pixel
//! depth = 0.3                          # how fast a ridge takes graphite above the contact
//! dust = 0.3                           # the share the valleys take, 0..=1
//! ```
//!
//! A missing curve is flat at 1; a missing `[shape]` is a hard edge; a missing `[grain]` none;
//! without `[accumulate]` the brush covers (crossings never darken).

use std::fmt;
use std::sync::Arc;

use openpaint_core::{
    Accumulation, BrushRecipe, CoreError, Curve, Deposit, Dynamics, Grain, Paper, PaperTexture, Tip,
};
use toml::{Table, Value};

use crate::brushes::{Brush, BrushKey, SizeRange};

/// Longest brush name, in characters.
const MAX_BRUSH_NAME: usize = 40;

/// Why a brush file was refused. `field` is the dotted path of the value, such as `size.max`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecipeError {
    /// Not TOML: reading stopped at this line and column, counted from 1.
    Syntax { line: usize, column: usize },
    /// A required value is missing.
    Missing { field: String },
    /// A value has the wrong kind; `expected` names the right one.
    WrongType {
        field: String,
        expected: &'static str,
    },
    /// A key the format does not have.
    Unknown { field: String },
    /// A value of the right kind outside its range, or curve points out of order.
    OutOfRange { field: String },
}

impl fmt::Display for RecipeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax { line, column } => write!(f, "not TOML at line {line}, column {column}"),
            Self::Missing { field } => write!(f, "{field} is missing"),
            Self::WrongType { field, expected } => write!(f, "{field} must be {expected}"),
            Self::Unknown { field } => write!(f, "{field} is not a brush setting"),
            Self::OutOfRange { field } => write!(f, "{field} is out of range"),
        }
    }
}

impl std::error::Error for RecipeError {}

/// Reads the brush `key` from its file `text`.
pub(crate) fn parse(key: BrushKey, text: &str) -> Result<Brush, RecipeError> {
    let table: Table = text
        .parse()
        .map_err(|error: toml::de::Error| syntax(text, error.span()))?;
    let root = Section {
        table: &table,
        path: "",
    };
    root.only(&[
        "name",
        "size",
        "opacity",
        "shape",
        "grain",
        "accumulate",
        "paper",
    ])?;
    let name = root.string("name")?.ok_or_else(|| missing(&root, "name"))?;
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_BRUSH_NAME {
        return Err(out_of_range(&root, "name"));
    }
    let size = root.required_section("size")?;
    size.only(&["default", "min", "max", "pressure", "speed"])?;
    let range = SizeRange::new(
        size.required_number("min")?,
        size.required_number("default")?,
        size.required_number("max")?,
    )
    .ok_or_else(|| out_of_range(&root, "size"))?;
    let opacity = root.required_section("opacity")?;
    opacity.only(&["default", "pressure", "speed"])?;
    let start = opacity.required_number("default")?;
    if !(start > 0.0 && start <= 1.0) {
        return Err(out_of_range(&opacity, "default"));
    }
    let recipe = BrushRecipe::new(
        softness(&root)?,
        dynamics(&size)?,
        dynamics(&opacity)?,
        grain(&root)?,
    )
    .map_err(|_| out_of_range(&root, "shape.softness"))?;
    let recipe = match deposit(&root)? {
        Some(deposit) => recipe
            .with_deposit(deposit)
            .map_err(|_| out_of_range(&root, "shape.softness"))?,
        None => recipe,
    };
    Ok(Brush {
        key,
        name: name.to_owned(),
        recipe: Arc::new(recipe),
        size: range,
        opacity: start,
    })
}

fn dynamics(section: &Section<'_>) -> Result<Dynamics, RecipeError> {
    let curve = |key: &str| -> Result<Curve, RecipeError> {
        let points = section.curve(key)?.unwrap_or_else(|| vec![[0.0, 1.0]]);
        Curve::new(&points).map_err(|_: CoreError| out_of_range(section, key))
    };
    let (pressure, speed) = (curve("pressure")?, curve("speed")?);
    let within = pressure.points().iter().all(|[x, _]| *x <= 1.0);
    let culprit = if within { "speed" } else { "pressure" };
    Dynamics::new(pressure, speed).map_err(|_| out_of_range(section, culprit))
}

fn softness(root: &Section<'_>) -> Result<f32, RecipeError> {
    let Some(shape) = root.section("shape")? else {
        return Ok(0.0);
    };
    shape.only(&["softness"])?;
    Ok(shape.number("softness")?.unwrap_or(0.0))
}

fn grain(root: &Section<'_>) -> Result<Grain, RecipeError> {
    let Some(grain) = root.section("grain")? else {
        return Ok(Grain::NONE);
    };
    grain.only(&["strength", "scale"])?;
    let strength = grain.required_number("strength")?;
    let scale = grain.integer("scale")?.unwrap_or(1);
    let scale = u32::try_from(scale).map_err(|_| out_of_range(&grain, "scale"))?;
    Grain::new(strength, scale).map_err(|_| out_of_range(&grain, "strength"))
}

/// The `[accumulate]` deposit with its `[paper]`, if the brush accumulates.
fn deposit(root: &Section<'_>) -> Result<Option<Deposit>, RecipeError> {
    let paper_section = root.section("paper")?;
    let Some(section) = root.section("accumulate")? else {
        return match paper_section {
            Some(_) => Err(missing(root, "accumulate")),
            None => Ok(None),
        };
    };
    section.only(&["tip", "flow", "max_density"])?;
    let tip = match section.string("tip")? {
        None => return Err(missing(&section, "tip")),
        Some("flat") => Tip::Flat,
        Some("bell") => Tip::Bell,
        Some(_) => return Err(out_of_range(&section, "tip")),
    };
    let flow = section.required_number("flow")?;
    let max_density = section.number("max_density")?.unwrap_or(1.0);
    let paper = paper_section.map(|paper| paper_of(&paper)).transpose()?;
    let accumulation = Accumulation::new(tip, flow, max_density, paper)
        .map_err(|_| out_of_range(&section, "flow"))?;
    Ok(Some(Deposit::Accumulate(accumulation)))
}

fn paper_of(section: &Section<'_>) -> Result<Paper, RecipeError> {
    section.only(&["texture", "contact", "scale", "depth", "dust"])?;
    let texture = match section.string("texture")? {
        None => return Err(missing(section, "texture")),
        Some("sketch") => PaperTexture::Sketch,
        Some(_) => return Err(out_of_range(section, "texture")),
    };
    let points = section
        .curve("contact")?
        .ok_or_else(|| missing(section, "contact"))?;
    let contact = Curve::new(&points).map_err(|_| out_of_range(section, "contact"))?;
    Paper::new(
        texture,
        contact,
        section.required_number("scale")?,
        section.required_number("depth")?,
        section.required_number("dust")?,
    )
    .map_err(|_| out_of_range(section, "scale"))
}

/// The line and column where `span` starts in `text`, from 1.
fn syntax(text: &str, span: Option<std::ops::Range<usize>>) -> RecipeError {
    let at = span.map_or(0, |span| span.start).min(text.len());
    let before = text.get(..at).unwrap_or(text);
    let line = before.matches('\n').count() + 1;
    let column = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
    RecipeError::Syntax { line, column }
}

fn missing(section: &Section<'_>, key: &str) -> RecipeError {
    RecipeError::Missing {
        field: section.field(key),
    }
}

fn out_of_range(section: &Section<'_>, key: &str) -> RecipeError {
    RecipeError::OutOfRange {
        field: section.field(key),
    }
}

/// One table of the file and its dotted path.
struct Section<'a> {
    table: &'a Table,
    path: &'static str,
}

impl<'a> Section<'a> {
    fn field(&self, key: &str) -> String {
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
    fn only(&self, known: &[&str]) -> Result<(), RecipeError> {
        match self.table.keys().find(|key| !known.contains(&key.as_str())) {
            Some(key) => Err(RecipeError::Unknown {
                field: self.field(key),
            }),
            None => Ok(()),
        }
    }

    fn section(&self, key: &'static str) -> Result<Option<Section<'a>>, RecipeError> {
        match self.table.get(key) {
            None => Ok(None),
            Some(Value::Table(table)) => Ok(Some(Section { table, path: key })),
            Some(_) => Err(self.wrong(key, "a table")),
        }
    }

    fn required_section(&self, key: &'static str) -> Result<Section<'a>, RecipeError> {
        self.section(key)?.ok_or_else(|| missing(self, key))
    }

    fn string(&self, key: &str) -> Result<Option<&'a str>, RecipeError> {
        match self.table.get(key) {
            None => Ok(None),
            Some(Value::String(text)) => Ok(Some(text)),
            Some(_) => Err(self.wrong(key, "text")),
        }
    }

    fn number(&self, key: &str) -> Result<Option<f32>, RecipeError> {
        match self.table.get(key) {
            None => Ok(None),
            Some(value) => as_number(value)
                .map(Some)
                .ok_or_else(|| self.wrong(key, "a number")),
        }
    }

    fn required_number(&self, key: &str) -> Result<f32, RecipeError> {
        self.number(key)?.ok_or_else(|| missing(self, key))
    }

    fn integer(&self, key: &str) -> Result<Option<i64>, RecipeError> {
        match self.table.get(key) {
            None => Ok(None),
            Some(Value::Integer(value)) => Ok(Some(*value)),
            Some(_) => Err(self.wrong(key, "a whole number")),
        }
    }

    /// A list of `[x, y]` number pairs.
    fn curve(&self, key: &str) -> Result<Option<Vec<[f32; 2]>>, RecipeError> {
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
