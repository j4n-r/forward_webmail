use std::time::Duration;

use forward_webmail::{settings::UserSettings, *};
use log::{debug, error, info};
use reqwest::{
    Url,
    cookie::{CookieStore, Jar},
    header::SET_COOKIE,
};
use serde_json::json;

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
    last_mail: &mut i32,
    client: &reqwest::Client,
    session_key: &str,
) -> anyhow::Result<()> {
    // for some reason these two mostly work on the second try ????
    // TODO make a nicer retry logic !!!!!
    tokio::time::sleep(Duration::from_secs(5)).await;
    let total_mails = match webmail::get_total_emails(client, session_key).await
    {
        Ok(total) => total,
        Err(_) => {
            debug!("Retry get_total_mails()");
            tokio::time::sleep(Duration::from_secs(5)).await;
            webmail::get_total_emails(client, session_key).await?
        }
    };
    if total_mails > *last_mail {
        for id in *last_mail + 1..=total_mails {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let webmail =
                match webmail::get_email_by_id(client, session_key, id).await {
                    Ok(webmail) => webmail,
                    Err(_) => {
                        debug!("Retry get_email_by_id()");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        webmail::get_email_by_id(client, session_key, id).await?
                    }
                };
            let email = mail_client::Email::from_webmail(webmail)?;
            mail_client::send_mail(settings, email)?;
            info!("Mail: {id} forwarded");
            *last_mail = id;
        }
    }
    Ok(())
}

async fn try_login(
    client: &reqwest::Client,
    settings: &UserSettings,
    url: &Url,
) -> anyhow::Result<String> {
    let res = webmail::login(client, settings, url)
        .await?
        .json::<webmail::LoginResponse>()
        .await?;
    Ok(res.session)
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

    let url =
        Url::parse("https://webmail.stud.hwr-berlin.de/appsuite/api/login")?;
    let res = webmail::login(&client, &user_settings, &url).await?;

    let mut res_headers = res.headers().get_all(SET_COOKIE).iter();
    jar.set_cookies(&mut res_headers, &url);

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
            let total =
                webmail::get_total_emails(&client, res_json.session.as_str())
                    .await
                    .unwrap();
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
    let mut session_key = res_json.session;
    debug!("session key: {session_key}");

    // main loop to forward mails and if the session expires try to login
    let max_retries = 3;
    debug!("Max retries: {max_retries}");
    let mut attempts = 0;
    loop {
        match forward_mails(
            &user_settings,
            &mut last_mail,
            &client,
            session_key.as_str(),
        )
        .await
        {
            Ok(_) => attempts = 0,
            // TODO: handle sending email error differently
            Err(e) => {
                error!("Forwarding Mails failed: {e}");
                if let Err(e) =
                    send_discord_webhook(&user_settings, &client, e.to_string())
                        .await
                {
                    error!("Maybe session gone, maybe rate limited");
                    // TODO: send error email
                    todo!();
                }
                match try_login(&client, &user_settings, &url).await {
                    Ok(new_session_key) => {
                        session_key = new_session_key;
                        info!(
                            "Successfully got now session_key: {session_key}"
                        );
                    }
                    // if login fails we send a discord notification and increase the attempts
                    Err(e) => {
                        error!("Error logging in {e}");
                        if let Err(e) = send_discord_webhook(
                            &user_settings,
                            &client,
                            e.to_string(),
                        )
                        .await
                        {
                            error!("Error sending Discord webhook: {e}");
                            // TODO: send error email
                            todo!();
                        }
                        if attempts >= max_retries {
                            send_discord_webhook(
                                &user_settings,
                                &client,
                                format!(
                                    "Max login retires reached: \n Error: {e}"
                                ),
                            )
                            .await
                            // we panic anyways so why not also when discord fails
                            .expect("Max login retires reached: \n Error: {e}");
                            panic!("Max retries reached, shutting down");
                        }
                        attempts += 1;
                        debug!("Current Attempt: {attempts}");
                    }
                }
            }
        };
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

        let url = Url::parse(
            "https://webmail.stud.hwr-berlin.de/appsuite/api/login",
        )?;
        let res = webmail::login(&client, &user_settings, &url).await?;

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
