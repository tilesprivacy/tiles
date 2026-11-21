//! `tiles config`

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use tiles::{
    daemon::{ApiResponse, daemon_client, daemon_url},
    utils::config::LlamaSetting,
};

const LLAMA_SETTINGS_PATH: &str = "/v1/tilekit/config/llama";

#[derive(Deserialize)]
struct SettingsResponse {
    settings: Vec<LlamaSetting>,
}

async fn request_settings(request: reqwest::RequestBuilder) -> Result<Vec<LlamaSetting>> {
    let res = request
        .header("content-type", "application/json")
        .send()
        .await
        .context("Could not reach the Tiles daemon")?;
    if res.status().is_success() {
        Ok(res
            .json::<ApiResponse<SettingsResponse>>()
            .await?
            .data
            .settings)
    } else {
        let body: Value = res.json().await.unwrap_or_default();
        Err(anyhow!(
            "{}",
            body["reason"].as_str().unwrap_or("The daemon refused it")
        ))
    }
}

fn value_as_text(setting: &LlamaSetting) -> String {
    match &setting.value {
        Some(Value::String(text)) => text.clone(),
        Some(value) => value.to_string(),
        None => "-".to_owned(),
    }
}

pub async fn list_settings_cli() -> Result<()> {
    let request = daemon_client().await?.get(daemon_url(LLAMA_SETTINGS_PATH));
    let settings = request_settings(request).await?;
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
    println!(
        "{:name$}  {:value$}  {:default$}",
        "SETTING", "VALUE", "DEFAULT"
    );
    for setting in &settings {
        let mut help = setting.help.clone();
        if !setting.choices.is_empty() {
            let values: Vec<_> = setting.choices.iter().map(|c| c.value.as_str()).collect();
            help = format!("{help}: {}", values.join(", "));
        }
        println!(
            "{:name$}  {:value$}  {:default$}  {help}",
            setting.name,
            value_as_text(setting),
            setting.default
        );
    }
    println!("\nChange one with `tiles config set NAME=VALUE`");
    Ok(())
}

pub async fn get_setting_cli(name: &str) -> Result<()> {
    let request = daemon_client().await?.get(daemon_url(LLAMA_SETTINGS_PATH));
    let settings = request_settings(request).await?;
    let setting = settings
        .iter()
        .find(|setting| setting.name == name)
        .ok_or_else(|| anyhow!("No setting named {name}"))?;
    match &setting.value {
        Some(_) => println!("{}", value_as_text(setting)),
        None => println!("{}", setting.default),
    }
    Ok(())
}

pub async fn set_settings_cli(pairs: &[String]) -> Result<()> {
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
    let request = daemon_client()
        .await?
        .post(daemon_url("/v1/tilekit/config/llama/set"))
        .json(&json!({ "values": values }));
    print_changed_settings(&request_settings(request).await?, values.keys());
    Ok(())
}

pub async fn reset_settings_cli(names: Vec<String>, all: bool) -> Result<()> {
    let request = daemon_client()
        .await?
        .post(daemon_url("/v1/tilekit/config/llama/reset"))
        .json(&json!({ "names": names, "all": all }));
    let settings = request_settings(request).await?;
    if all {
        print_changed_settings(&settings, settings.iter().map(|setting| &setting.name));
    } else {
        print_changed_settings(&settings, names.iter());
    }
    Ok(())
}

fn print_changed_settings<'a>(settings: &[LlamaSetting], names: impl Iterator<Item = &'a String>) {
    for name in names {
        if let Some(setting) = settings.iter().find(|setting| &setting.name == name) {
            match &setting.value {
                Some(_) => println!("{name} = {}", value_as_text(setting)),
                None => println!("{name} = {} (default)", setting.default),
            }
        }
    }
    println!("Applies on the next prompt.");
}
