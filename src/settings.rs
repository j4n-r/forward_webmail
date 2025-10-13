use config::Config;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Deserialize, Serialize, Debug)]
pub struct AppData {
    pub last_forwarded_mail_id: i32
}

#[derive(Deserialize, Debug)]
pub struct UserSettings {
    pub username: String,
    pub password: String,
    pub smtp_username: String,
    pub smtp_token: String,
    pub smtp_server: String,
    pub smtp_port: u16,
    pub forward_address: String,
}

pub fn parse_from_file() -> UserSettings {
    let settings = Config::builder()
        // Add in `./Settings.toml`
        .add_source(config::File::with_name("settings.toml"))
        .build()
        .unwrap();
    settings
        .try_deserialize::<UserSettings>()
        .expect("Error with settings file")
}

pub fn get_app_data() -> anyhow::Result<AppData> {
    let data = fs::read("app_data.json")?;
    let app_data: AppData= serde_json::from_slice(data.as_slice())?;
    Ok(app_data)
}

pub fn save_app_data(data: &AppData) -> anyhow::Result<()> {
    let json_data = serde_json::to_string_pretty(&data)?;
    fs::write("app_data.json", json_data)?;
    Ok(())
}
