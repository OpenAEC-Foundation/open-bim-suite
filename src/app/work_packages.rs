use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Member {
    pub source: PathBuf,
    pub identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    #[serde(default)]
    pub steel: bool,
    pub status: Stage,
    pub members: BTreeSet<Member>,
    #[serde(default)]
    pub superseded: BTreeSet<Member>,
    #[serde(default)]
    pub replacements: BTreeSet<Member>,
    pub inputs: Vec<Input>,
    /// The elements it is made of by a filter: what they are, and a property with a value.
    #[serde(default)]
    pub group: Group,
}

/// A filter of elements: what they are (their models, layers, classes and names, as a selector
/// of the Validator takes them), and a property with a value when `property` is set (`*` at an
/// end for anything, `|` between choices).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Group {
    pub models: Vec<String>,
    pub layers: Vec<String>,
    pub classes: Vec<String>,
    pub names: Vec<String>,
    pub property: String,
    pub value: String,
}

impl Group {
    /// Whether it takes every element: nothing set.
    pub fn is_all(&self) -> bool {
        self.models.is_empty() && self.layers.is_empty() && self.classes.is_empty() && self.names.is_empty() && self.property.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Stage {
    #[default]
    Draft,
    Review,
    Final,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Input {
    pub label: String,
    pub reference: String,
    pub status: InputStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum InputStatus {
    #[default]
    Requested,
    Received,
    Accepted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interface {
    pub packages: [usize; 2],
    pub subject: String,
    pub agreed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Register {
    pub packages: Vec<Package>,
    pub interfaces: Vec<Interface>,
}

impl Register {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    pub fn map_sources(&mut self, map: impl Fn(&Path) -> PathBuf) {
        for p in &mut self.packages {
            for members in [&mut p.members, &mut p.superseded, &mut p.replacements] {
                *members = std::mem::take(members)
                    .into_iter()
                    .map(|mut m| {
                        m.source = map(&m.source);
                        m
                    })
                    .collect();
            }
        }
    }
}
