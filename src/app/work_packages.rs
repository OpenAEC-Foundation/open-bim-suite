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
