use anyhow::Result;
use colored::Colorize;

use kb_core::config::{self as core_config, ConfigKey, ConfigValue};

pub fn run_list(json: bool, quiet: bool) -> Result<()> {
    list(json, quiet)
}

pub fn run_get(json: bool, key: &str, quiet: bool) -> Result<()> {
    get(key, json, quiet)
}

pub fn run_set(json: bool, key: &str, value: &str, quiet: bool) -> Result<()> {
    set(key, value, json, quiet)
}

pub fn run_unset(json: bool, key: &str, quiet: bool) -> Result<()> {
    unset(key, json, quiet)
}

fn list(json: bool, quiet: bool) -> Result<()> {
    let config = core_config::load()?;
    let path = core_config::config_path()?;

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "path": path,
                "config": serde_json::to_value(&config)?,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    if !quiet {
        println!();
        println!("  {} ~/.kb config", "Config".bold().green(),);
        println!();
        println!("  {} {}", "file".dimmed(), path.display());
        println!();
        print_key(ConfigKey::KbRoot, &config);
        print_key(ConfigKey::ActiveProjects, &config);

        let mut names: Vec<&String> = config.projects.keys().collect();
        names.sort();
        for name in names {
            print_key(ConfigKey::ProjectRepoPath(name.clone()), &config);
            print_key(ConfigKey::ProjectCloneDepth(name.clone()), &config);
        }
        println!();
    }

    Ok(())
}

fn get(key: &str, json: bool, quiet: bool) -> Result<()> {
    let parsed = core_config::parse_config_key(key)?;
    let config = core_config::load()?;
    let value = core_config::get_key(&config, &parsed);

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "key": key,
                "value": value_to_json(&value),
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    if !quiet {
        println!();
        match &value {
            ConfigValue::Path(Some(p)) => println!("  {} = {}", key.bold(), p.display()),
            ConfigValue::Path(None) => println!("  {} = {}", key.bold(), "(unset)".dimmed()),
            ConfigValue::Strings(items) if items.is_empty() => {
                println!("  {} = {}", key.bold(), "(none)".dimmed())
            }
            ConfigValue::Strings(items) => {
                println!(
                    "  {} = {}",
                    key.bold(),
                    serde_json::to_string(items)?.dimmed()
                )
            }
            ConfigValue::U32(depth) => println!("  {} = {}", key.bold(), depth),
        }
        println!();
    }

    Ok(())
}

fn set(key: &str, value: &str, json: bool, quiet: bool) -> Result<()> {
    let parsed = core_config::parse_config_key(key)?;
    let mut config = core_config::load()?;
    core_config::set_key(&mut config, &parsed, value)?;
    let path = core_config::config_path()?;
    core_config::save(&config)?;

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "key": key,
                "value": serde_json::to_value(value)?,
                "file": path,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    if !quiet {
        let display = match get_key_value(&config, &parsed) {
            ConfigValue::Path(Some(p)) => p.display().to_string(),
            ConfigValue::Strings(items) => serde_json::to_string(&items)?,
            ConfigValue::U32(depth) => depth.to_string(),
            ConfigValue::Path(None) => value.to_string(),
        };
        println!();
        println!("  {} {} {}", "Set".green().bold(), key.bold(), display);
        println!("  └ {}", path.display().to_string().dimmed());
        println!();
    }

    Ok(())
}

fn unset(key: &str, json: bool, quiet: bool) -> Result<()> {
    let parsed = core_config::parse_config_key(key)?;
    let mut config = core_config::load()?;
    core_config::unset_key(&mut config, &parsed);
    let path = core_config::config_path()?;
    core_config::save(&config)?;

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "key": key,
                "file": path,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    if !quiet {
        println!();
        println!("  {} {}", "Unset".yellow().bold(), key.bold());
        println!();
        println!(
            "  {}",
            "key removed from config (or reset to default).".dimmed()
        );
        println!();
    }

    Ok(())
}

fn print_key(key: ConfigKey, config: &core_config::Config) {
    let value = core_config::get_key(config, &key);
    let label = key_label(&key);
    match &value {
        ConfigValue::Path(Some(p)) => println!("  {}  = {}", label.dimmed(), p.display()),
        ConfigValue::Path(None) => println!("  {}  = {}", label.dimmed(), "(unset)".dimmed()),
        ConfigValue::Strings(items) if items.is_empty() => {
            println!("  {}  = {}", label.dimmed(), "(none)".dimmed())
        }
        ConfigValue::Strings(items) => {
            let list = serde_json::to_string(items).unwrap_or_default();
            println!("  {}  = {}", label.dimmed(), list.dimmed())
        }
        ConfigValue::U32(depth) => println!("  {}  = {}", label.dimmed(), depth),
    }
}

fn key_label(key: &ConfigKey) -> String {
    match key {
        ConfigKey::KbRoot => "kb_root".to_string(),
        ConfigKey::ActiveProjects => "active_projects".to_string(),
        ConfigKey::ProjectRepoPath(name) => format!("projects.{}.repo_path", name),
        ConfigKey::ProjectCloneDepth(name) => format!("projects.{}.clone_depth", name),
    }
}

fn get_key_value(config: &core_config::Config, key: &ConfigKey) -> ConfigValue {
    core_config::get_key(config, key)
}

fn value_to_json(value: &ConfigValue) -> serde_json::Value {
    match value {
        ConfigValue::Path(Some(p)) => serde_json::Value::String(p.display().to_string()),
        ConfigValue::Path(None) => serde_json::Value::Null,
        ConfigValue::Strings(items) => serde_json::Value::Array(
            items
                .iter()
                .map(|s| serde_json::Value::String(s.clone()))
                .collect(),
        ),
        ConfigValue::U32(d) => (*d).into(),
    }
}
