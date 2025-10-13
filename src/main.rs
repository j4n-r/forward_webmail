use forward_webmail::{mail_client::Email,*};
use reqwest::{
    Url,
    cookie::{CookieStore, Jar},
    header::SET_COOKIE,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let user_settings = settings::parse_from_file();

    let mut app_data = settings::get_app_data()?;

    // let jar = std::sync::Arc::new(Jar::default());
    // let client = reqwest::Client::builder()
    //     .cookie_provider(jar.clone())
    //     .build()?;

    // let url =
    //     Url::parse("https://webmail.stud.hwr-berlin.de/appsuite/api/login")?;
    // let res = webmail::login(&client, &user_settings, url.clone()).await?;

    // let mut res_headers = res.headers().get_all(SET_COOKIE).iter();
    // jar.set_cookies(&mut res_headers, &url);

    // let res_json = res.json::<webmail::LoginResponse>().await?;

    // let total =
    //     webmail::get_total_emails(&client, res_json.session.as_str()).await?;

    // let webmail =
    //     webmail::get_email_by_id(&client, res_json.session.as_str(), total-6)
    //         .await?;
    // let email =
    //     Email::from_webmail(webmail).expect("Error parsing webmail into email");

    // dbg!(&email);

    // mail_client::send_mail(&user_settings, email);

    Ok(())
}
