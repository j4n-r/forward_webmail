use lettre::message::{Mailbox, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

fn main() {
    let email = Message::builder()
        .from(Mailbox::new(Some("j4n-r".to_owned()), "hwr@j4n.me".parse().unwrap()))
        .to(Mailbox::new(Some("Me".to_owned()), "jan.rueggeberg@pm.me".parse().unwrap()))
        .subject("TEst")
        .header(ContentType::TEXT_PLAIN)
        .body(String::from("Test"))
        .unwrap();

    let creds = Credentials::new("smtp_username".to_owned(), "smtp_password".to_owned());

    // Open a remote connection to gmail
    let mailer = SmtpTransport::relay("smtp.gmail.com")
        .unwrap()
        .credentials(creds)
        .build();

    // Send the email
    match mailer.send(&email) {
        Ok(_) => println!("Email sent successfully!"),
        Err(e) => panic!("Could not send email: {e:?}"),
    }
}
