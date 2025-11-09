use crate::settings;
use crate::webmail;

use anyhow::anyhow;
use anyhow::Context;
use lettre::message::{Mailbox, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use log::info;

#[derive(Debug)]
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

pub fn send_mail(settings: &settings::UserSettings, data: Email)  -> anyhow::Result<()> {
    let content = if data.has_attachment {
        format!(
            "<div>This Email has one or more attachment please check webmail<br><br></div> {}",
            data.content
        )
    } else {
        data.content
    };
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
    mailer.send(&email)?;
    info!("Email sent successfully!");
    Ok(())
}

impl Email {
    pub fn from_webmail(webmail: webmail::Webmail) -> anyhow::Result<Email> {
        let id: i32 = webmail.id.parse()
            .context("Failed to parse webmail ID")?;
        
        let from_arr = webmail.from.first()
            .ok_or_else(|| anyhow!("The from array is empty"))?;
        
        let from_email = from_arr.get(1)
            .ok_or_else(|| anyhow!("The from array should have at least 2 entries"))?
            .to_owned()
            .ok_or_else(|| anyhow!("The from email field is empty"))?;
        
        let from_name = from_arr.first()
            .ok_or_else(|| anyhow!("The from array is unexpectedly empty when accessing first element"))?
            .to_owned()
            .unwrap_or(from_email.clone());
        
        let to_arr = webmail.to.first()
            .ok_or_else(|| anyhow!("The to array is empty"))?;
        
        let to_email = to_arr.get(1)
            .ok_or_else(|| anyhow!("The to array should have at least 2 entries"))?
            .to_owned()
            .ok_or_else(|| anyhow!("The to email field is empty"))?;
        
        let to_name = to_arr.first()
            .ok_or_else(|| anyhow!("The to array is unexpectedly empty when accessing first element"))?
            .to_owned()
            .unwrap_or(to_email.clone());
        
        let content = webmail.attachments.first()
            .ok_or_else(|| anyhow!("There should always be a content attachment here"))?
            .content
            .as_ref()
            .unwrap_or(&String::from("No content available"))
            .to_owned();
        
        Ok(Email {
            id,
            from_name,
            from_email,
            to_name,
            to_email,
            subject: webmail.subject,
            content,
            has_attachment: webmail.attachment,
        })
    }
}
