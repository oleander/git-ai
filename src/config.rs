use lazy_static::lazy_static;
use std::sync::RwLock;
use std::str::FromStr;
use anyhow::Result;
use ini::Ini;
use std::env;

lazy_static! {
  static ref CONFIG_PATH: String = {
    let home_dir = env::var("HOME").expect("Failed to find HOME directory");
    let bin_name = env::var("CARGO_PKG_NAME").unwrap_or("git-ai".to_owned());
    let config_dir = format!("{home_dir}/.config/{bin_name}");
    let config_file = format!("{config_dir}/config.ini");

    if !std::path::Path::new(&config_file).exists() {
      std::fs::create_dir_all(config_dir).expect("Failed to create config directory");
      std::fs::write(&config_file, "").expect("Failed to create config file");
    }

    config_file
  };

  static ref CONFIG: RwLock<Ini> = RwLock::new(Ini::load_from_file(CONFIG_PATH.as_str()).expect("Failed to load config file"));
}

pub fn get<T: FromStr>(key: &str) -> Result<T> {
  CONFIG
    .read()
    .unwrap()
    .general_section()
    .get(key)
    .ok_or_else(|| anyhow::anyhow!("Failed to find key: {}", key))
    .and_then(|v| v.parse::<T>().map_err(|_| anyhow::anyhow!("Failed to parse value")))
}

pub fn set(key: &str, value: &str) -> Result<()> {
  let mut config = CONFIG.write().unwrap();
  config.with_section(None::<String>).set(key, value);
  config.write_to_file(CONFIG_PATH.as_str()).expect("Failed to write config file");
  Ok(())
}

#[test]
fn test_config() {
  set("a", "b").unwrap();
  let a: String = get("a").unwrap();

  assert_eq!(a, "b");

  let content = std::fs::read_to_string(&*CONFIG_PATH).unwrap();
  assert!(content.contains("a=b"));
}
