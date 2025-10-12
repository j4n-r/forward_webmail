use crate::settings;
use lettre::message::{Mailbox, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

pub fn send_mail(settings: &settings::UserSettings) {
    let email = Message::builder()
        .from(Mailbox::new(
            Some("".to_owned()),
            settings.smtp_username.parse().unwrap(),
        ))
        .to(Mailbox::new(
            Some("Me".to_owned()),
            settings.forward_address.parse().unwrap(),
        ))
        .subject("TEst")
        .header(ContentType::TEXT_PLAIN)
        .body(String::from("Test"))
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
