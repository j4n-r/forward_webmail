use config::Config;
use serde::Deserialize;

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
