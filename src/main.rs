use forward_webmail::{ settings::UserSettings, *};
use reqwest::{
    Url,
    cookie::{CookieStore, Jar},
    header::SET_COOKIE,
};
use serde_json::json;


async fn send_discord_webhook(settings: &UserSettings, client: &reqwest::Client, msg: String) -> anyhow::Result<reqwest::StatusCode> {
    let msg = json![{"content": msg}];
    let res = client.post(settings.webhook.as_str()).json(&msg).send().await?;
    dbg!(&res);
    Ok(res.status())

}

async fn forward_mails(
    settings: &UserSettings,
    last_mail: &mut i32,
    client: &reqwest::Client,
    session_key: &str,
) -> anyhow::Result<()> {
    let total_mails = webmail::get_total_emails(client, session_key).await?;

    if total_mails > *last_mail {
        for id in *last_mail + 1..=total_mails {
            let webmail =
                webmail::get_email_by_id(client, session_key, id).await?;
            let email = mail_client::Email::from_webmail(webmail)?;
            mail_client::send_mail(settings, email)?;
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
    let user_settings = settings::parse_from_file();

    let jar = std::sync::Arc::new(Jar::default());
    let client = reqwest::Client::builder()
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
        Ok(data) => data,
        // if not get the total number of emails
        Err(_) => {
            // this panics because there is no sensible default we can use for
            // the start of the email forwarding except the newest one
            let total =
                webmail::get_total_emails(&client, res_json.session.as_str())
                    .await
                    .unwrap();
            settings::AppData {
                last_forwarded_mail_id: total,
            }
        }
    };
    // panic because if we can't save stuff we don't want to continue
    settings::save_app_data(&app_data).unwrap();

    // start a loop which checks the total nuber with the number in the app_data.json
    // if it is higher then fetch the new emails and forward them
    // sleep

    let mut last_mail = app_data.last_forwarded_mail_id;
    let mut session_key = res_json.session;
    // relogin if get_email or get_total fails
    loop {
        let max_retries = 3;
        let mut attempts = 0;

        loop {
            //try happy path
            if let Err(_) = forward_mails(
                &user_settings,
                &mut last_mail,
                &client,
                session_key.as_str(),
            )
            .await
            {
                match try_login(&client, &user_settings, &url).await {
                    Ok(new_session_key) => session_key = new_session_key,
                    Err(e) => {
                    if attempts > max_retries {
                        panic!("Max login retires reached: {e}");
                    }
                    attempts += 1;
                    }
                }
            }
            let five_min = std::time::Duration::from_secs(5 * 60);
            std::thread::sleep(five_min);
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn discord_webhook() -> anyhow::Result<()> {
        let user_settings = settings::parse_from_file();

        let client = reqwest::Client::builder()
            .build()?;
        let msg = "test";
        let res_code = send_discord_webhook(&user_settings, &client, msg.to_string()).await?;
        debug_assert!(reqwest::StatusCode::is_success(&res_code) == true);
        Ok(())
    }

    // #[tokio::test]
    // async fn end_to_end() -> anyhow::Result<()> {
    //     let user_settings = settings::parse_from_file();

    //     let mut app_data = settings::get_app_data()?;

    //     let jar = std::sync::Arc::new(Jar::default());
    //     let client = reqwest::Client::builder()
    //         .cookie_provider(jar.clone())
    //         .build()?;

    //     let url = Url::parse(
    //         "https://webmail.stud.hwr-berlin.de/appsuite/api/login",
    //     )?;
    //     let res = webmail::login(&client, &user_settings, &url).await?;

    //     let mut res_headers = res.headers().get_all(SET_COOKIE).iter();
    //     jar.set_cookies(&mut res_headers, &url);

    //     let res_json = res.json::<webmail::LoginResponse>().await?;

    //     let total =
    //         webmail::get_total_emails(&client, res_json.session.as_str())
    //             .await?;

    //     let webmail = webmail::get_email_by_id(
    //         &client,
    //         res_json.session.as_str(),
    //         total - 6,
    //     )
    //     .await?;
    //     let email = Email::from_webmail(webmail)
    //         .expect("Error parsing webmail into email");

    //     let status = mail_client::send_mail(&user_settings, email);
    //     assert!(status.is_ok());
    //     Ok(())
    // }
}
