///
/// Blitz Archiving Explorer
///
/// Request on the tcp server
///
/// Copyright 2019 Luis Fernando Batels <luisfbatels@gmail.com>
///

use std::net::{TcpStream, SocketAddr};
use std::io::{BufReader, BufRead, Write, BufWriter};

use catalog::catalog::Catalog;

pub struct Request {
}

impl Request {

    // Handle the client connection
    pub fn handle(conn: TcpStream, catalog: &mut Catalog) {
        let pa = conn.peer_addr();
        if pa.is_err() {
            error!("Error on get the remote addr {:?}", conn);
            return;
        }

        let client = pa.unwrap();

        info!("Handling {}...", client);

        let mut command = String::new();
        let mut buf_reader = BufReader::new(&conn);

        if buf_reader.read_line(&mut command).is_err() {
            error!("Error on read the command from the client {}", client);
            return;
        }

        let mut command_ok = false;

        if command.starts_with("/search/") {

            let mut search = command.replacen("/search/", "", 1);
            search = search.trim().to_string();

            if !search.is_empty() {

                for tar in catalog.get_catalogs() {

                    for file in catalog.get_catalog(&tar) {

                        if file.file_name.contains(search.as_str()) {
                            Request::response(&conn, client, format!("{}:{}\n", tar.file_name.clone(), file.full_path.clone()));
                        }
                    }
                }

                command_ok = true;
            }

        } else if command.starts_with("/download/") && command.contains(":") {

            let mut download = command.replacen("/download/", "", 1);
            download = download.trim().to_string();
            if !download.is_empty() {
                let mut download_slices = download.split(":");

                let tar_file = download_slices.next();
                if tar_file.is_none() {
                    error!("Tar file not setted: {}", download);
                    return;
                }

                let name_file = download_slices.next();
                if name_file.is_none() {
                    error!("Name of file not setted: {}", download);
                    return;
                }

                for tar in catalog.get_catalogs() {

                    if tar.file_name == tar_file.unwrap() {

                        for file in catalog.get_catalog(&tar) {

                            if file.full_path == name_file.unwrap() {

                                if !catalog.extract_file(&tar, &file, &mut BufWriter::new(&conn)) {
                                    error!("Error on extract: {}", download);
                                    return;
                                }
                                break;
                            }
                        }
                        break;
                    }
                }

                command_ok = true;
            }
        }

        if !command_ok {
            warn!("Invalid command {}", client);
            Request::response(&conn, client, "Invalid command\n".to_string());
            return;
        }

        info!("Handling {}...OK", client);
    }

    // Create and flush the response to the client
    fn response(mut conn: &TcpStream, client: SocketAddr, text: String) -> bool {

        if conn.write_all(text.as_bytes()).is_err() {
            error!("Error on send '{}' message to client {}", text, client);
            return false;
        }

        return true;
    }
}
