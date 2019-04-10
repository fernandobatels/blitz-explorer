///
/// Blitz Archiving Explorer
///
/// Main of the application
///
/// Copyright 2019 Luis Fernando Batels <luisfbatels@gmail.com>
///

#[macro_use]
extern crate json;

use std::fs::File;
use std::path::Path;
use std::io::BufReader;

use flate2::read::GzDecoder;
use tar::Archive;
use sled::Db;

const DB_INDEX: &str = "/var/db/blitzae";

fn main() {

    let db = Db::start_default(DB_INDEX).unwrap();

    let file_tar = "/tmp/testes.tar.gz".to_string();
    let path = Path::new(&file_tar);
    let archive = File::open(path).unwrap();

    let files = db.open_tree(path.file_name().unwrap().to_str().unwrap().to_string())
                .unwrap();

    if !files.is_empty() {

        for idx in files.iter().keys() {
            println!("{:?}", std::str::from_utf8(&idx.unwrap()));
        }

        return;
    }

    let buffer_archive = BufReader::new(archive);

    let decoder = GzDecoder::new(buffer_archive);
    let buffer_decoder = BufReader::new(decoder);

    let mut tar = Archive::new(buffer_decoder);

    for file in tar.entries().unwrap() {
        let header = file.unwrap().header().clone();
        let file_path = header.path().unwrap().to_str().unwrap().to_string();

        let data = object!{
            "size" => header.size().unwrap(),
            "mtime" => header.mtime().unwrap(),
            "file_name" => header.path().unwrap().file_name().unwrap().to_str().unwrap().to_string(),
            "path" => file_path.clone()
        };

        files.set(file_path.as_bytes(), data.dump().as_bytes().to_vec())
            .expect("Error on create index for a file");
    }

    db.flush()
     .expect("Error on flush db");

}
