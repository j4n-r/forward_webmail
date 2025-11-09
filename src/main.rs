use std::time::Duration;

use forward_webmail::{settings::UserSettings, *};
use log::{debug, error, info};
use reqwest::{
    Url,
    cookie::{CookieStore, Jar},
    header::SET_COOKIE,
};
use serde_json::json;

async fn retry_function<F, Fut, T>(
    mut f: F,
    max_attempts: i32,
) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    let mut attempt = 0;
    loop {
        let res = f().await;
        if res.is_ok() || attempt >= max_attempts {
            break res;
        }
        attempt += 1;
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn send_discord_webhook(
    settings: &UserSettings,
    client: &reqwest::Client,
    msg: String,
) -> anyhow::Result<reqwest::StatusCode> {
    let msg = json![{"content": msg}];
    let res = client
        .post(settings.webhook.as_str())
        .json(&msg)
        .send()
        .await?;
    Ok(res.status())
}

async fn forward_mails(
    settings: &UserSettings,
    mut last_mail: i32,
    client: &reqwest::Client,
    session_key: &str,
) -> anyhow::Result<i32> {
    // for some reason these two mostly work on the second try ????
    let total_mails =
        retry_function(|| webmail::get_total_emails(client, session_key), 3)
            .await?;

    if total_mails > last_mail {
        for id in last_mail + 1..=total_mails {
            tokio::time::sleep(Duration::from_secs(3)).await;
            let webmail = retry_function(
                || webmail::get_email_by_id(client, session_key, id),
                3,
            )
            .await?;
            let email = mail_client::Email::from_webmail(webmail, settings)?;
            mail_client::send_mail(settings, email)?;
            info!("Mail: {id} forwarded");
            last_mail = id;
        }
    }
    Ok(last_mail)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let user_settings = settings::parse_from_file();

    let jar = std::sync::Arc::new(Jar::default());
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36")
        .cookie_provider(jar.clone())
        .build()?;

    let res =
        retry_function(|| webmail::login(&client, &user_settings), 3).await?;

    let mut res_headers = res.headers().get_all(SET_COOKIE).iter();
    jar.set_cookies(
        &mut res_headers,
        &Url::parse("https://webmail.stud.hwr-berlin.de")?,
    );

    let res_json = res.json::<webmail::LoginResponse>().await?;

    // check if there is start value in app_data.json
    let app_data = match settings::get_app_data() {
        Ok(data) => {
            debug!("Found start value: {:?}", &data);
            data
        }
        // if not get the total number of emails
        Err(_) => {
            error!("Error getting app_data file");
            // this panics because there is no sensible default we can use for
            // the start of the email forwarding except the newest one
            let total = retry_function(
                || {
                    webmail::get_total_emails(
                        &client,
                        res_json.session.as_str(),
                    )
                },
                3,
            )
            .await
            // fail early here since it's the start of the program
            .expect("Cannot get total mails");
            debug!("Got total Mails: {total}");
            settings::AppData {
                last_forwarded_mail_id: total,
            }
        }
    };
    // panic because if we can't save stuff we don't want to continue
    settings::save_app_data(&app_data).expect("Was not able to save app_data.");
    debug!("saved app_data");

    // keep track of the last email that was forwarded so we don't have to read from disk all the time
    let mut last_mail = app_data.last_forwarded_mail_id;
    debug!("Last Mail forwarded: {last_mail}");
    // keep track of the current seesion key in case of relogin
    let session_key = res_json.session;
    debug!("session key: {session_key}");

    // main loop to forward mails and if the session expires try to login
    let max_retries = 3;
    debug!("Max retries: {max_retries}");
    let mut attempts = 0;
    loop {
        match forward_mails(
            &user_settings,
            last_mail,
            &client,
            session_key.as_str(),
        )
        .await
        {
            Ok(last_processed_mail) => {
                last_mail = last_processed_mail;
                let new_app_data = settings::AppData {
                    last_forwarded_mail_id: last_mail,
                };
                if let Err(e) = settings::save_app_data(&new_app_data) {
                    error!("Error saving app_data {e}");
                    let _ = retry_function(
                        || {
                            send_discord_webhook(
                                &user_settings,
                                &client,
                                e.to_string(),
                            )
                        },
                        3,
                    )
                    .await;
                }
                attempts = 0
            }
            Err(e) => {
                if let Err(e) = retry_function(
                    || webmail::login(&client, &user_settings),
                    3,
                )
                .await
                {
                    let _ = retry_function(
                        || {
                            send_discord_webhook(
                                &user_settings,
                                &client,
                                e.to_string(),
                            )
                        },
                        3,
                    )
                    .await;
                }
                error!("Error forwarding mails: {e}");
                let _ = retry_function(
                    || {
                        send_discord_webhook(
                            &user_settings,
                            &client,
                            e.to_string(),
                        )
                    },
                    3,
                )
                .await
                .unwrap_or_else(|e| {
                    error!("Sending Discord webhook failed: {e}");
                    reqwest::StatusCode::SERVICE_UNAVAILABLE
                });
                attempts += 1;
            }
        }
        if attempts == max_retries {
            retry_function(
                || {
                    send_discord_webhook(
                        &user_settings,
                        &client,
                        "Max retires reached, shutting down".to_string(),
                    )
                },
                3,
            )
            .await
            .expect("Max retries reached, and discord failed");
            panic!("Max retries reached")
        }
        let five_min = std::time::Duration::from_secs(5 * 60);
        tokio::time::sleep(five_min).await;
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn discord_webhook() -> anyhow::Result<()> {
        let user_settings = settings::parse_from_file();

        let client = reqwest::Client::builder().build()?;
        let msg = "test";
        let res_code =
            send_discord_webhook(&user_settings, &client, msg.to_string())
                .await?;
        debug_assert!(reqwest::StatusCode::is_success(&res_code) == true);
        Ok(())
    }

    #[tokio::test]
    async fn end_to_end() -> anyhow::Result<()> {
        let user_settings = settings::parse_from_file();

        let mut app_data = settings::get_app_data()?;

        let jar = std::sync::Arc::new(Jar::default());
        let client = reqwest::Client::builder()
            .cookie_provider(jar.clone())
            .build()?;

        let url = Url::parse("https://webmail.stud.hwr-berlin.de/")?;
        let res = webmail::login(&client, &user_settings).await?;

        let mut res_headers = res.headers().get_all(SET_COOKIE).iter();
        jar.set_cookies(&mut res_headers, &url);

        let res_json = res.json::<webmail::LoginResponse>().await?;

        let total =
            webmail::get_total_emails(&client, res_json.session.as_str())
                .await?;

        let webmail =
            webmail::get_email_by_id(&client, res_json.session.as_str(), total)
                .await?;
        let email = mail_client::Email::from_webmail(webmail)
            .expect("Error parsing webmail into email");

        let status = mail_client::send_mail(&user_settings, email);
        assert!(status.is_ok());
        Ok(())
    }
}
