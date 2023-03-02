use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::sync::Arc;

use env_logger::Env;
use log::{error, info};
use tokio::io::{split, AsyncBufReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;

mod test_body;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(Env::new()
        .default_filter_or("info"));

    let host = "127.0.0.1";
    let port = 13337;

    let certs = rustls_pemfile::certs(&mut BufReader::new(File::open("cert.pem")?))?;
    let cert = rustls::Certificate(certs[0].clone());

    let keys = rustls_pemfile::pkcs8_private_keys(&mut BufReader::new(File::open("key.pem")?))?;
    let key = rustls::PrivateKey(keys[0].clone());

    let config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)?;
    let acceptor = TlsAcceptor::from(Arc::new(config));

    let listener = TcpListener::bind((host, port)).await?;
    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_client(acceptor, stream, peer_addr).await {
                error!("[{}] {:?}", peer_addr, err);
            }
        });
    }
}

async fn handle_client(acceptor: TlsAcceptor, stream: TcpStream, peer_addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    info!("[{}] connected", peer_addr);

    let stream = acceptor.accept(stream).await?;
    let (reader, mut writer) = split(stream);
    let reader = tokio::io::BufReader::new(reader);

    writer.write_all("* OK [CAPABILITY IMAP4 IMAP4rev1 AUTH=PLAIN AUTH=LOGIN AUTH=CRAM-MD5 AUTH=DIGEST-MD5 CHILDREN ENABLE I18NLEVEL=2 ID IDLE MOVE MULTIAPPEND NAMESPACE QUOTA SORT STATUS=SIZE UIDPLUS UNSELECT WITHIN XLIST] IMAP server ready (P18 TLSv1.3:TLS_AES_256_GCM_SHA384)\r\n".as_bytes()).await?;
    writer.flush().await?;

    let mut lines = reader.lines();
    while let Some(line) = lines.next_line().await? {
        info!("[{}] input: {}", peer_addr, line);
        let Some((request_id, request)) = line.split_once(' ') else {
            return Err("bad request".into());
        };

        if request.starts_with("LOGIN ") {
            writer.write_all(format!("{request_id} OK User logged in (30)\r\n").as_bytes()).await?;
            writer.flush().await?;

        } else if request.starts_with("SELECT ") {
            writer.write_all(format!("* FLAGS (\\Answered \\Flagged \\Deleted \\Seen \\Draft \\Forwarded NonJunk $Forwarded $Junk $label1 $label2)\r\n").as_bytes()).await?;
            writer.write_all(format!("* OK [PERMANENTFLAGS (\\Answered \\Flagged \\Deleted \\Seen \\Draft \\Forwarded NonJunk $Forwarded $Junk $label1 $label2 \\*)]\r\n").as_bytes()).await?;
            writer.write_all(format!("* OK [URLMECH INTERNAL]\r\n").as_bytes()).await?;
            writer.write_all(format!("* 1212 EXISTS\r\n").as_bytes()).await?;
            writer.write_all(format!("* 0 RECENT\r\n").as_bytes()).await?;
            writer.write_all(format!("* OK [UIDVALIDITY 1491927899]\r\n").as_bytes()).await?;
            writer.write_all(format!("* OK [UIDNEXT 1250]\r\n").as_bytes()).await?;
            writer.write_all(format!("{request_id} OK [READ-WRITE] SELECT completed\r\n").as_bytes()).await?;
            writer.flush().await?;

        } else if request.starts_with("FETCH ") {
            let Some((id, _)) = request.strip_prefix("FETCH ").unwrap().split_once(' ') else {
                return Err("bad request".into());
            };
            let id = id.parse()?;

            let mut body = test_body::BASE.to_string();
            for _ in 1..id {
                body += "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX\r\n";
            }
            let body_len = body.len();

            writer.write_all(format!(
                "* {id} FETCH (INTERNALDATE \"17-Jun-2017 10:18:07 +0200\" RFC822.SIZE {body_len} BODY[] {{{body_len}}}\r\n{body} FLAGS (\\Seen NonJunk))\r\n"
            ).as_bytes()).await?;
            writer.write_all(format!("{request_id} OK FETCH complete\r\n").as_bytes()).await?;
            writer.flush().await?;

        } else if request.starts_with("LOGOUT") {
            writer.write_all(format!("{request_id} OK LOGOUT completed\r\n").as_bytes()).await?;
            writer.flush().await?;
            break;

        } else {
            return Err("bad request".into());
        }
    }

    info!("[{}] disconnected", peer_addr);
    Ok(())
}
