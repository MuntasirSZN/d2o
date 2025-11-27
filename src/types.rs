use ecow::{EcoString, EcoVec};
use foldhash::quality::RandomState;
use scc::{HashMap as SccHashMap, HashSet as SccHashSet};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub type HashMap<K, V> = SccHashMap<K, V, RandomState>;
pub type HashSet<T> = SccHashSet<T, RandomState>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Command {
    pub name: EcoString,
    pub description: EcoString,
    pub usage: EcoString,
    pub options: EcoVec<Opt>,
    #[serde(default)]
    pub subcommands: EcoVec<Command>,
    #[serde(default)]
    pub version: EcoString,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Opt {
    pub names: EcoVec<OptName>,
    pub argument: EcoString,
    pub description: EcoString,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub struct OptName {
    pub raw: EcoString,
    #[serde(rename = "type")]
    pub opt_type: OptNameType,
}

impl<'de> Deserialize<'de> for OptName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum OptNameCompat {
            Legacy(String),
            Structured {
                raw: EcoString,
                #[serde(rename = "type")]
                opt_type: OptNameType,
            },
        }

        match OptNameCompat::deserialize(deserializer)? {
            OptNameCompat::Legacy(s) => {
                let opt_type = OptName::determine_type(&s)
                    .ok_or_else(|| serde::de::Error::custom("invalid option name"))?;
                Ok(OptName {
                    raw: EcoString::from(s),
                    opt_type,
                })
            }
            OptNameCompat::Structured { raw, opt_type } => Ok(OptName { raw, opt_type }),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "UPPERCASE")]
pub enum OptNameType {
    LongType,
    ShortType,
    OldType,
    DoubleDashAlone,
    SingleDashAlone,
}

impl PartialOrd for OptName {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OptName {
    fn cmp(&self, other: &Self) -> Ordering {
        (&self.raw, &self.opt_type).cmp(&(&other.raw, &other.opt_type))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Subcommand {
    pub cmd: EcoString,
    pub desc: EcoString,
}

impl OptName {
    pub fn new(raw: EcoString, opt_type: OptNameType) -> Self {
        Self { raw, opt_type }
    }

    pub fn from_text(s: &str) -> Option<Self> {
        let opt_type = Self::determine_type(s)?;
        Some(Self {
            raw: EcoString::from(s),
            opt_type,
        })
    }

    fn determine_type(s: &str) -> Option<OptNameType> {
        match s {
            "-" => Some(OptNameType::SingleDashAlone),
            "--" => Some(OptNameType::DoubleDashAlone),
            s if s.starts_with("--") => Some(OptNameType::LongType),
            s if s.starts_with('-') && s.len() == 2 => Some(OptNameType::ShortType),
            s if s.starts_with('-') => Some(OptNameType::OldType),
            _ => None,
        }
    }
}

impl std::fmt::Display for OptName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl std::fmt::Display for Opt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names = self
            .names
            .iter()
            .map(|n| n.raw.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        write!(
            f,
            "{}  ::  {}\n{}\n",
            names, self.argument, self.description
        )
    }
}

impl std::fmt::Display for Subcommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:<25} ({})", self.cmd, self.desc)
    }
}

impl Command {
    pub fn new(name: EcoString) -> Self {
        Self {
            name,
            description: EcoString::new(),
            usage: EcoString::new(),
            options: EcoVec::new(),
            subcommands: EcoVec::new(),
            version: EcoString::new(),
        }
    }

    pub fn as_subcommand(&self) -> Subcommand {
        Subcommand {
            cmd: self.name.clone(),
            desc: self.description.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_new_and_as_subcommand() {
        let mut cmd = Command::new(EcoString::from("test"));
        assert_eq!(cmd.name.as_str(), "test");
        assert!(cmd.description.is_empty());

        cmd.description = EcoString::from("Test command");
        let sub = cmd.as_subcommand();
        assert_eq!(sub.cmd.as_str(), "test");
        assert_eq!(sub.desc.as_str(), "Test command");
    }
}
