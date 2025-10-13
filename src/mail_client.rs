use crate::settings;
use crate::webmail;

use anyhow::anyhow;
use lettre::message::{Mailbox, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

#[derive(Debug )]
pub struct Email {
    pub id: i32,
    pub from_name: String,
    pub from_email: String,
    pub to_name: String,
    pub to_email: String,
    pub subject: String,
    pub content: String,
    pub has_attachment: bool,
}

pub fn send_mail(settings: &settings::UserSettings, data: Email) {
    let content = format!(
        "<div>Has attachment: {}<br><br></div> {}",
        data.has_attachment, data.content
    );
    let email = Message::builder()
        .from(Mailbox::new(
            Some(data.from_name.to_owned()),
            settings.smtp_username.parse().unwrap(),
        ))
        .to(Mailbox::new(
            Some(data.to_name.to_owned()),
            settings.forward_address.parse().unwrap(),
        ))
        .subject(data.subject)
        .header(ContentType::TEXT_HTML)
        .body(content)
        .unwrap();

    let creds = Credentials::new(
        settings.smtp_username.to_owned(),
        settings.smtp_token.to_owned(),
    );

    let mailer = SmtpTransport::relay(settings.smtp_server.as_str())
        .unwrap()
        .credentials(creds)
        .build();

    // Send the email
    match mailer.send(&email) {
        Ok(_) => println!("Email sent successfully!"),
        Err(e) => panic!("Could not send email: {e:?}"),
    }
}


impl Email {
    pub fn from_webmail(webmail: webmail::Webmail) -> anyhow::Result<Email> {
        let id: i32 = webmail.id.parse().expect("Error parsing webmail ID");
        let from_arr = webmail
            .from
            .first()
            .expect("The from array should always exist");

        let from_email = from_arr
            .get(1)
            .expect("There should always be 2 entries in the from array")
            .to_owned()
            .ok_or_else(|| {
                anyhow!("The email field is empty, this should never happen")
            })?;

        let from_name = from_arr
            .first()
            .expect("There should always be a value here but it might be null")
            .to_owned()
            .unwrap_or(from_email.clone());

        let to_arr = webmail
            .to
            .first()
            .expect("The to array should always exist");

        let to_email = to_arr
            .get(1)
            .expect("There should always be 2 entries in the to array")
            .to_owned()
            .ok_or_else(|| {
                anyhow!("The email field is empty, this should never happen")
            })?;

        let to_name = to_arr
            .first()
            .expect("There should always be a value here but it might be null")
            .to_owned()
            .unwrap_or(to_email.clone());

        let content = webmail
            .attachments
            .first()
            .ok_or_else(|| {
                anyhow!("There should always be a content attachment here")
            })?
            .content
            .to_owned();

        Ok(Email {
            id,
            from_name,
            from_email,
            to_name,
            to_email,
            subject: webmail.subject,
            content: content,
            has_attachment: webmail.attachment,
        })
    }
}
