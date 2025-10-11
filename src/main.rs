use anyhow::Context;
use config::Config;
use reqwest::{
    Url,
    cookie::{CookieStore, Jar},
    header::SET_COOKIE,
};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct LoginResponse {
    session: String,
}

#[derive(Deserialize)]
struct UserSettings {
    username: String,
    password: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Config::builder()
        // Add in `./Settings.toml`
        .add_source(config::File::with_name("settings.toml"))
        .build()
        .unwrap();
    let user_settings = settings
        .try_deserialize::<UserSettings>()
        .context("Add settings.toml configuration")?;

    let url = Url::parse("https://webmail.stud.hwr-berlin.de/appsuite/api/login")?;

    // This will POST a body of `foo=bar&baz=quux`
    let params = [
        ("action", "login"),
        ("name", user_settings.username.as_str()),
        ("password", user_settings.password.as_str()),
    ];

    let jar = std::sync::Arc::new(Jar::default());
    let client = reqwest::Client::builder()
        .cookie_provider(jar.clone())
        .build()?;

    let res = client.post(url.clone()).form(&params).send().await?;
    println!("Login request status: {}", res.status());

    let mut res_headers = res.headers().get_all(SET_COOKIE).iter();
    jar.set_cookies(&mut res_headers, &url);

    println!("{}", res.status());

    let res_json = res.json::<LoginResponse>().await?;
    dbg!(&res_json);

    let url = Url::parse("https://webmail.stud.hwr-berlin.de/appsuite/api/mail")?;
    let params = [
        ("action", "get"),
        ("id", "1020"),
        ("folder", "default0/INBOX"),
        ("session", res_json.session.as_str()),
    ];
    let res = client
        .get(url.clone())
        .query(&params)
        .send()
        .await?
        .text()
        .await?;

    dbg!(res);

    Ok(())
}
