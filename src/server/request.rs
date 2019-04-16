///
/// Blitz Archiving Explorer
///
/// Request on the tcp server
///
/// Copyright 2019 Luis Fernando Batels <luisfbatels@gmail.com>
///

use std::net::{TcpStream, SocketAddr};
use std::io::{BufReader, BufRead, Write};

pub struct Request {
}

impl Request {

    // Handle the client connection
    pub fn handle(mut conn: TcpStream) {
        let pa = conn.peer_addr();
        if pa.is_err() {
            error!("Error on get the remote addr {:?}", conn);
            return;
        }

        let client = pa.unwrap();

        info!("Handling {}...", client);

        if !Request::response(&mut conn, client, "Welcome to Blitz Archive Explorer.\nFor search the files send: /search/SEARCH HERE\n".to_string()) {
            return;
        }

        let mut command = String::new();
        let mut buf_reader = BufReader::new(&mut conn);

        if buf_reader.read_line(&mut command).is_err() {
            error!("Error on read the command from the client {}", client);
            return;
        }

        let mut command_ok = false;

        if command.starts_with("/search/") {
            let mut search = command.replace("/search/", "");
            search = search.trim().to_string();
            if !search.is_empty() {
                command_ok = true;
            }
        }

        if !command_ok {
            warn!("Invalid command {}", client);
            Request::response(&mut conn, client, "Invalid command\n".to_string());
            return;
        }

        info!("Handling {}...OK", client);
    }

    // Create and flush the response to the client
    fn response(conn: &mut TcpStream, client: SocketAddr, text: String) -> bool {

        if conn.write_all(text.as_bytes()).is_err() {
            error!("Error on send '{}' message to client {}", text, client);
            return false;
        }

        return true;
    }
}
