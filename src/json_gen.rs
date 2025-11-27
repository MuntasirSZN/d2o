use crate::types::Command;
use ecow::EcoString;
use serde_json::json;

pub struct JsonGenerator;

impl JsonGenerator {
    pub fn generate(cmd: &Command) -> EcoString {
        let json = Self::command_to_json(cmd);
        EcoString::from(serde_json::to_string_pretty(&json).unwrap_or_default())
    }

    fn command_to_json(cmd: &Command) -> serde_json::Value {
        let mut obj = json!({
            "name": cmd.name.as_str(),
            "description": cmd.description.as_str(),
            "usage": cmd.usage.as_str(),
            "options": cmd.options.iter().map(|opt| {
                json!({
                    "names": opt.names.iter().map(|n| n.raw.as_str()).collect::<Vec<_>>(),
                    "argument": opt.argument.as_str(),
                    "description": opt.description.as_str(),
                })
            }).collect::<Vec<_>>(),
        });

        if !cmd.subcommands.is_empty() {
            obj["subcommands"] = serde_json::json!(
                cmd.subcommands
                    .iter()
                    .map(|sub| {
                        json!({
                            "name": sub.name.as_str(),
                            "description": sub.description.as_str(),
                        })
                    })
                    .collect::<Vec<_>>()
            );
        }

        if !cmd.version.is_empty() {
            obj["version"] = json!(cmd.version.as_str());
        }

        obj
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecow::{EcoString, EcoVec};

    #[test]
    fn test_json_generator_includes_fields() {
        let cmd = Command {
            name: EcoString::from("test"),
            description: EcoString::from("Test command"),
            usage: EcoString::from("test [OPTIONS]"),
            options: EcoVec::new(),
            subcommands: {
                let mut v = EcoVec::new();
                v.push(Command {
                    name: EcoString::from("sub"),
                    description: EcoString::from("Subcommand"),
                    usage: EcoString::new(),
                    options: EcoVec::new(),
                    subcommands: EcoVec::new(),
                    version: EcoString::new(),
                });
                v
            },
            version: EcoString::from("1.0.0"),
        };

        let json_str = JsonGenerator::generate(&cmd);
        let value: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(value["name"], "test");
        assert_eq!(value["description"], "Test command");
        assert_eq!(value["usage"], "test [OPTIONS]");
        assert_eq!(value["version"], "1.0.0");
        assert_eq!(value["subcommands"][0]["name"], "sub");
        assert_eq!(value["subcommands"][0]["description"], "Subcommand");
    }

    #[test]
    fn test_json_generator_includes_options() {
        let cmd = Command {
            name: EcoString::from("test"),
            description: EcoString::from("Test command"),
            usage: EcoString::from("test [OPTIONS]"),
            options: {
                let mut v = EcoVec::new();
                v.push(crate::types::Opt {
                    names: {
                        let mut names = EcoVec::new();
                        names.push(crate::types::OptName::new(
                            EcoString::from("-v"),
                            crate::types::OptNameType::ShortType,
                        ));
                        names.push(crate::types::OptName::new(
                            EcoString::from("--verbose"),
                            crate::types::OptNameType::LongType,
                        ));
                        names
                    },
                    argument: EcoString::from("FILE"),
                    description: EcoString::from("Enable verbose mode"),
                });
                v
            },
            subcommands: EcoVec::new(),
            version: EcoString::new(),
        };

        let json_str = JsonGenerator::generate(&cmd);
        let value: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(value["options"].as_array().unwrap().len(), 1);
        let opt = &value["options"][0];
        assert_eq!(opt["names"], serde_json::json!(["-v", "--verbose"]));
        assert_eq!(opt["argument"], "FILE");
        assert_eq!(opt["description"], "Enable verbose mode");
    }
}
