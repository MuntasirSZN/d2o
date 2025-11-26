use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub usage: String,
    pub options: Vec<Opt>,
    #[serde(default)]
    pub subcommands: Vec<Command>,
    #[serde(default)]
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Opt {
    pub names: Vec<OptName>,
    pub argument: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub struct OptName {
    pub raw: String,
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
                raw: String,
                #[serde(rename = "type")]
                opt_type: OptNameType,
            },
        }

        match OptNameCompat::deserialize(deserializer)? {
            OptNameCompat::Legacy(s) => {
                let opt_type = OptName::determine_type(&s)
                    .ok_or_else(|| serde::de::Error::custom("invalid option name"))?;
                Ok(OptName { raw: s, opt_type })
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
    pub cmd: String,
    pub desc: String,
}

impl OptName {
    pub fn new(raw: String, opt_type: OptNameType) -> Self {
        Self { raw, opt_type }
    }

    pub fn from_text(s: &str) -> Option<Self> {
        let opt_type = Self::determine_type(s)?;
        Some(Self {
            raw: s.to_string(),
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
            .map(|n| n.raw.clone())
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
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: String::new(),
            usage: String::new(),
            options: Vec::new(),
            subcommands: Vec::new(),
            version: String::new(),
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
        let mut cmd = Command::new("test".to_string());
        assert_eq!(cmd.name, "test");
        assert!(cmd.description.is_empty());

        cmd.description = "Test command".to_string();
        let sub = cmd.as_subcommand();
        assert_eq!(sub.cmd, "test");
        assert_eq!(sub.desc, "Test command");
    }
}
