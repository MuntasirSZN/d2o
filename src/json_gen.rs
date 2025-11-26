use crate::types::Command;
use serde_json::json;

pub struct JsonGenerator;

impl JsonGenerator {
    pub fn generate(cmd: &Command) -> String {
        let json = Self::command_to_json(cmd);
        serde_json::to_string_pretty(&json).unwrap_or_default()
    }

    fn command_to_json(cmd: &Command) -> serde_json::Value {
        let mut obj = json!({
            "name": cmd.name,
            "description": cmd.description,
            "usage": cmd.usage,
            "options": cmd.options.iter().map(|opt| {
                json!({
                    "names": opt.names.iter().map(|n| n.raw.clone()).collect::<Vec<_>>(),
                    "argument": opt.argument,
                    "description": opt.description,
                })
            }).collect::<Vec<_>>(),
        });

        if !cmd.subcommands.is_empty() {
            obj["subcommands"] = serde_json::json!(
                cmd.subcommands
                    .iter()
                    .map(|sub| {
                        json!({
                            "name": sub.name,
                            "description": sub.description,
                        })
                    })
                    .collect::<Vec<_>>()
            );
        }

        if !cmd.version.is_empty() {
            obj["version"] = json!(cmd.version);
        }

        obj
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_generator_includes_fields() {
        let cmd = Command {
            name: "test".to_string(),
            description: "Test command".to_string(),
            usage: "test [OPTIONS]".to_string(),
            options: vec![],
            subcommands: vec![Command {
                name: "sub".to_string(),
                description: "Subcommand".to_string(),
                usage: String::new(),
                options: vec![],
                subcommands: vec![],
                version: String::new(),
            }],
            version: "1.0.0".to_string(),
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
            name: "test".to_string(),
            description: "Test command".to_string(),
            usage: "test [OPTIONS]".to_string(),
            options: vec![crate::types::Opt {
                names: vec![
                    crate::types::OptName::new(
                        "-v".to_string(),
                        crate::types::OptNameType::ShortType,
                    ),
                    crate::types::OptName::new(
                        "--verbose".to_string(),
                        crate::types::OptNameType::LongType,
                    ),
                ],
                argument: "FILE".to_string(),
                description: "Enable verbose mode".to_string(),
            }],
            subcommands: vec![],
            version: String::new(),
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
