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

#[derive(Deserialize, Debug)]
struct UserSettings {
    username: String,
    password: String,
}

#[derive(Deserialize, Debug)]
struct EmailAttachment {
    content_type: String,
    size: i32,
    content: String,
}

#[derive(Deserialize, Debug)]
struct EmailData {
    from: Vec<Vec<Option<String>>>,
    to: Vec<Vec<Option<String>>>,
    attachment: bool,
    subject: String,
    date: i64,
    attachments: Vec<EmailAttachment>
}

#[derive(Deserialize, Debug)]
struct Email {
    data: EmailData
}

async fn login(
    client: &reqwest::Client,
    settings: &UserSettings,
    url: Url,
) -> anyhow::Result<reqwest::Response> {
    let params = [
        ("action", "login"),
        ("name", settings.username.as_str()),
        ("password", settings.password.as_str()),
    ];

    let res = client.post(url).form(&params).send().await?;
    println!("Login request status: {:?}", res.status());
    Ok(res)
}

async fn get_email_by_id(client: &reqwest::Client, session_key: &str, id: i32) -> anyhow::Result<Email> {
    let url = Url::parse("https://webmail.stud.hwr-berlin.de/appsuite/api/mail")?;
    let params = [
        ("action", "get"),
        ("id", &id.to_string()),
        ("folder", "default0/INBOX"),
        ("session", session_key),
    ];
    let res_text = client
        .get(url)
        .query(&params)
        .send()
        .await?
        .text()
        .await?;
    println!("Email: {res_text:?}");
    let email: Email = serde_json::from_str(&res_text)?;
    println!("{email:?}");
    Ok(email)
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

    let jar = std::sync::Arc::new(Jar::default());
    let client = reqwest::Client::builder()
        .cookie_provider(jar.clone())
        .build()?;

    let url = Url::parse("https://webmail.stud.hwr-berlin.de/appsuite/api/login")?;
    let res = login(&client, &user_settings, url.clone()).await?;

    let mut res_headers = res.headers().get_all(SET_COOKIE).iter();
    jar.set_cookies(&mut res_headers, &url);

    let res_json = res.json::<LoginResponse>().await?;
    let email = get_email_by_id(&client, res_json.session.as_str(), 1040).await?;

    Ok(())
}
