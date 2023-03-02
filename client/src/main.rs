use std::error::Error;

use env_logger::Env;
use futures::TryStreamExt;
use log::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(Env::new()
        .default_filter_or("info"));

    let host = "127.0.0.1";
    let port = 13337;
    let login = "mail@example.com";
    let password = "12345";

    let tls = async_native_tls::TlsConnector::new()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .use_sni(false);
    let client = async_imap::connect((host, port), host, tls).await?;

    let mut imap_session = client.login(login, password).await
        .map_err(|e| e.0)?;

    imap_session.select("INBOX").await?;

    for i in 1..=20 {
        info!("fetching {}", i);
        let mut messages_stream = imap_session.fetch(i.to_string(), "(FLAGS INTERNALDATE RFC822.SIZE BODY.PEEK[])").await?;
        while let Some(fetch) = messages_stream.try_next().await? {
            let body = fetch.body().expect("message did not have a body!");
            info!("{}: {} bytes", fetch.message, body.len());
        }
        drop(messages_stream);
        info!("fetching done");
    }

    imap_session.logout().await?;

    Ok(())
}
