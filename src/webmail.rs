use anyhow::{ anyhow};
use reqwest::Url;
use serde::Deserialize;
use crate::settings;

#[derive(Deserialize, Debug)]

pub struct LoginResponse {
    pub session: String,
}

#[derive(Deserialize, Debug)]
pub struct EmailAttachment {
    pub content_type: String,
    pub size: i32,
    pub content: String,
}

#[derive(Deserialize, Debug)]
pub struct EmailData {
    pub from: Vec<Vec<Option<String>>>,
    pub to: Vec<Vec<Option<String>>>,
    pub attachment: bool,
    pub subject: String,
    pub date: i64,
    pub attachments: Vec<EmailAttachment>,
}

#[derive(Deserialize, Debug)]
pub struct Email {
    pub data: EmailData,
}

pub async fn login(
    client: &reqwest::Client,
    settings: &settings::UserSettings,
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

pub async fn get_email_by_id(
    client: &reqwest::Client,
    session_key: &str,
    id: i32,
) -> anyhow::Result<Email> {
    let url = Url::parse("https://webmail.stud.hwr-berlin.de/appsuite/api/mail")?;
    let params = [
        ("action", "get"),
        ("id", &id.to_string()),
        ("folder", "default0/INBOX"),
        ("session", session_key),
    ];
    let res_text = client.get(url).query(&params).send().await?.text().await?;
    println!("Email: {res_text:?}");
    let email: Email = serde_json::from_str(&res_text)?;
    println!("{email:?}");
    Ok(email)
}

pub async fn get_total_emails(client: &reqwest::Client, session_key: &str) -> anyhow::Result<i32> {
    let url = Url::parse("https://webmail.stud.hwr-berlin.de/appsuite/api/mail")?;
    let params = [
        ("action", "all"),
        ("folder", "default0/INBOX"),
        ("session", session_key),
        ("columns", "600"),
        ("order", "desc"),
        ("limit", "1"),
    ];
    let res_text = client.get(url).query(&params).send().await?.text().await?;
    let res_values: serde_json::Value = serde_json::from_str(&res_text)?;
    match res_values["data"][0][0].as_str() {
        Some(total) => {
            let total = total.parse::<i32>()?;
            println!("Total Emails: {total}");
            Ok(total)
        }
        None => Err(anyhow!("Something went wrong while getting total")),
    }
    // let email: Email = serde_json::from_str(&res_text)?;
    // println!("{email:?}");
}
