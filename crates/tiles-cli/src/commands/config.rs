//! `tiles config`

use anyhow::{Result, anyhow};
use serde_json::{Map, Value, json};

use crate::api::{DaemonApi, LlamaSetting};

fn value_as_text(setting: &LlamaSetting) -> String {
    match &setting.value {
        Some(Value::String(text)) => text.clone(),
        Some(value) => value.to_string(),
        None => "-".to_owned(),
    }
}

/// `tiles config`: lists all llama.cpp settings with current values, defaults, and help text
pub async fn list_settings(api: &DaemonApi) -> Result<()> {
    let settings = api.fetch_llama_settings().await?;
    for line in settings_table_lines(&settings) {
        println!("{line}");
    }
    println!("\nChange one with `tiles config set NAME=VALUE`");
    Ok(())
}

/// `tiles config get NAME`: prints the active value (or default) of a single setting
pub async fn get_setting(api: &DaemonApi, name: &str) -> Result<()> {
    let settings = api.fetch_llama_settings().await?;
    println!("{}", setting_value_line(&settings, name)?);
    Ok(())
}

/// `tiles config set NAME=VALUE ...`: updates one or more llama.cpp settings
pub async fn set_settings(api: &DaemonApi, pairs: &[String]) -> Result<()> {
    let values = parse_setting_pairs(pairs)?;
    let settings = api.set_llama_settings(&values).await?;
    for line in changed_settings_lines(&settings, values.keys()) {
        println!("{line}");
    }
    Ok(())
}

/// `tiles config reset [NAMES... | --all]`: resets selected or all llama.cpp settings
pub async fn reset_settings(api: &DaemonApi, names: Vec<String>, all: bool) -> Result<()> {
    let settings = api.reset_llama_settings(&names, all).await?;
    let lines = if all {
        changed_settings_lines(&settings, settings.iter().map(|setting| &setting.name))
    } else {
        changed_settings_lines(&settings, names.iter())
    };
    for line in lines {
        println!("{line}");
    }
    Ok(())
}

fn parse_setting_pairs(pairs: &[String]) -> Result<Map<String, Value>> {
    let mut values = Map::new();
    for pair in pairs {
        let (name, value) = pair
            .split_once('=')
            .filter(|(name, value)| !name.is_empty() && !value.is_empty())
            .ok_or_else(|| {
                anyhow!("Write each setting as NAME=VALUE, like context_length=64000, not {pair}")
            })?;
        values.insert(name.to_owned(), json!(value));
    }
    Ok(values)
}

fn settings_table_lines(settings: &[LlamaSetting]) -> Vec<String> {
    let width = |column: fn(&LlamaSetting) -> usize, title: &str| {
        settings
            .iter()
            .map(column)
            .max()
            .unwrap_or(0)
            .max(title.len())
    };
    let name = width(|s| s.name.len(), "SETTING");
    let value = width(|s| value_as_text(s).len(), "VALUE");
    let default = width(|s| s.default.len(), "DEFAULT");

    let mut lines = Vec::with_capacity(settings.len() + 1);
    lines.push(format!(
        "{:name$}  {:value$}  {:default$}",
        "SETTING", "VALUE", "DEFAULT"
    ));
    for setting in settings {
        let mut help = setting.help.clone();
        if !setting.choices.is_empty() {
            let values: Vec<_> = setting.choices.iter().map(|c| c.value.as_str()).collect();
            help = format!("{help}: {}", values.join(", "));
        }
        lines.push(format!(
            "{:name$}  {:value$}  {:default$}  {help}",
            setting.name,
            value_as_text(setting),
            setting.default
        ));
    }
    lines
}

fn setting_value_line(settings: &[LlamaSetting], name: &str) -> Result<String> {
    let setting = settings
        .iter()
        .find(|setting| setting.name == name)
        .ok_or_else(|| anyhow!("No setting named {name}"))?;
    Ok(match &setting.value {
        Some(_) => value_as_text(setting),
        None => setting.default.clone(),
    })
}

fn changed_settings_lines<'a>(
    settings: &[LlamaSetting],
    names: impl Iterator<Item = &'a String>,
) -> Vec<String> {
    let mut lines = Vec::new();
    for name in names {
        if let Some(setting) = settings.iter().find(|setting| &setting.name == name) {
            let line = match &setting.value {
                Some(_) => format!("{name} = {}", value_as_text(setting)),
                None => format!("{name} = {} (default)", setting.default),
            };
            lines.push(line);
        }
    }
    lines.push("Applies on the next prompt.".to_owned());
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::SettingChoice;

    fn sample_settings() -> Vec<LlamaSetting> {
        vec![
            LlamaSetting {
                name: "context_length".to_owned(),
                value: Some(json!(32768)),
                default: "65536".to_owned(),
                choices: vec![],
                help: "Context window".to_owned(),
            },
            LlamaSetting {
                name: "load_mode".to_owned(),
                value: None,
                default: "auto".to_owned(),
                choices: vec![
                    SettingChoice {
                        value: "auto".to_owned(),
                    },
                    SettingChoice {
                        value: "mmap".to_owned(),
                    },
                ],
                help: "Load mode".to_owned(),
            },
        ]
    }

    #[test]
    fn pairs_are_split_into_name_and_value() {
        let pairs = [
            "context_length=32768".to_owned(),
            "load_mode=mmap+mlock".to_owned(),
        ];
        let parsed = parse_setting_pairs(&pairs).unwrap();

        assert_eq!(parsed["context_length"], json!("32768"));
        assert_eq!(parsed["load_mode"], json!("mmap+mlock"));
    }

    #[test]
    fn malformed_pairs_are_rejected() {
        for bad in ["context_length", "=32768", "context_length="] {
            let err = parse_setting_pairs(&[bad.to_owned()]).unwrap_err();
            assert!(
                err.to_string().contains("NAME=VALUE"),
                "unexpected error for {bad}: {err}"
            );
        }
    }

    #[test]
    fn settings_table_aligns_columns_and_lists_choices() {
        let lines = settings_table_lines(&sample_settings());

        assert_eq!(
            lines,
            [
                "SETTING         VALUE  DEFAULT",
                "context_length  32768  65536    Context window",
                "load_mode       -      auto     Load mode: auto, mmap",
            ]
        );
    }

    #[test]
    fn get_setting_returns_value_or_default_and_errors_when_missing() {
        let settings = sample_settings();

        assert_eq!(
            setting_value_line(&settings, "context_length").unwrap(),
            "32768"
        );
        assert_eq!(setting_value_line(&settings, "load_mode").unwrap(), "auto");
        assert_eq!(
            setting_value_line(&settings, "unknown")
                .unwrap_err()
                .to_string(),
            "No setting named unknown"
        );
    }

    #[test]
    fn changed_settings_distinguish_set_values_from_defaults() {
        let settings = sample_settings();
        let names = ["context_length".to_owned(), "load_mode".to_owned()];

        assert_eq!(
            changed_settings_lines(&settings, names.iter()),
            [
                "context_length = 32768",
                "load_mode = auto (default)",
                "Applies on the next prompt.",
            ]
        );
    }
}
