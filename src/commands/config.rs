use std::io;

use crate::args::ParsedArgs;
use crate::client::{ClockifyClient, UreqTransport};
use crate::commands::login;
use crate::config::{get_config, parse_rounding_mode, resolve_api_key, save_config};
use crate::error::CfdError;
use crate::format::format_resource_id;

pub fn execute(args: &ParsedArgs) -> Result<(), CfdError> {
    match args.action.as_deref() {
        None => show_config(),
        Some("interactive") => interactive_setup(),
        Some("set") => set_value(args),
        Some("get") => get_value(args),
        Some("unset") => unset_value(args),
        _ => Err(CfdError::message(
            "usage: cfd config [interactive|set|get|unset] [key] [value]",
        )),
    }
}

fn show_config() -> Result<(), CfdError> {
    let config = get_config()?;

    println!("apiKey: {}", mask_api_key(config.api_key.as_deref()));
    println!(
        "workspace: {}",
        config.workspace.as_deref().unwrap_or("not set")
    );
    println!(
        "project: {}",
        config.project.as_deref().unwrap_or("not set")
    );
    println!(
        "rounding: {}",
        config.rounding.map(rounding_as_str).unwrap_or("not set")
    );
    if !config.aliases.is_empty() {
        println!("aliases:");
        for (name, alias) in config.aliases {
            println!("  {name}:");
            println!("    project: {}", alias.project);
            println!("    task: {}", alias.task.as_deref().unwrap_or("not set"));
            println!(
                "    description: {}",
                alias.description.as_deref().unwrap_or("not set")
            );
        }
    }

    Ok(())
}

fn interactive_setup() -> Result<(), CfdError> {
    let config = get_config()?;
    let api_key = resolve_api_key(&config)?;
    let client = ClockifyClient::new(api_key.clone(), UreqTransport);
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    login::run_setup_with_io(&mut reader, &mut writer, &client, &api_key, "Saved config.")
}

fn set_value(args: &ParsedArgs) -> Result<(), CfdError> {
    let key = args.positional.first().map(String::as_str).ok_or_else(|| {
        CfdError::message("usage: cfd config set <workspace|project|rounding> <value>")
    })?;
    let value = args.positional.get(1).map(String::as_str).ok_or_else(|| {
        CfdError::message("usage: cfd config set <workspace|project|rounding> <value>")
    })?;

    let mut config = get_config()?;
    match key {
        "workspace" => config.workspace = Some(value.to_owned()),
        "project" => config.project = Some(value.to_owned()),
        "rounding" => config.rounding = Some(parse_rounding_mode(value)?),
        _ => return Err(CfdError::message(format!("unknown config key: {key}"))),
    }

    save_config(&config)
}

fn get_value(args: &ParsedArgs) -> Result<(), CfdError> {
    let key =
        args.positional.first().map(String::as_str).ok_or_else(|| {
            CfdError::message("usage: cfd config get <workspace|project|rounding>")
        })?;
    let config = get_config()?;

    let value = match key {
        "workspace" => config.workspace,
        "project" => config.project,
        "rounding" => config.rounding.map(rounding_as_str).map(str::to_owned),
        _ => return Err(CfdError::message(format!("unknown config key: {key}"))),
    }
    .ok_or_else(|| CfdError::message(format!("config value not set: {key}")))?;

    println!("{}", format_resource_id(&value));
    Ok(())
}

fn unset_value(args: &ParsedArgs) -> Result<(), CfdError> {
    let key =
        args.positional.first().map(String::as_str).ok_or_else(|| {
            CfdError::message("usage: cfd config unset <workspace|project|rounding>")
        })?;
    let mut config = get_config()?;

    match key {
        "workspace" => config.workspace = None,
        "project" => config.project = None,
        "rounding" => config.rounding = None,
        _ => return Err(CfdError::message(format!("unknown config key: {key}"))),
    }

    save_config(&config)
}

fn rounding_as_str(mode: crate::types::RoundingMode) -> &'static str {
    match mode {
        crate::types::RoundingMode::Off => "off",
        crate::types::RoundingMode::OneMinute => "1m",
        crate::types::RoundingMode::FiveMinutes => "5m",
        crate::types::RoundingMode::TenMinutes => "10m",
        crate::types::RoundingMode::FifteenMinutes => "15m",
    }
}

fn mask_api_key(value: Option<&str>) -> String {
    match value {
        Some(value) if !value.is_empty() => {
            let chars: Vec<char> = value.chars().collect();
            if chars.len() <= 6 {
                "*".repeat(chars.len())
            } else {
                format!(
                    "{}{}{}",
                    chars[..3].iter().collect::<String>(),
                    "*".repeat(chars.len() - 6),
                    chars[chars.len() - 3..].iter().collect::<String>()
                )
            }
        }
        _ => "not set".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_api_key_preserves_first_and_last_three_chars() {
        assert_eq!(mask_api_key(Some("abcdefghijk")), "abc*****ijk");
    }

    #[test]
    fn mask_api_key_fully_masks_short_values() {
        assert_eq!(mask_api_key(Some("secret")), "******");
    }
}
