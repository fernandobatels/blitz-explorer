///
/// Blitz Archiving Explorer
///
/// Main of the application
///
/// Copyright 2019 Luis Fernando Batels <luisfbatels@gmail.com>
///

use std::path::Path;
use std::fs;

use sled::Db;

mod files;

use files::catalog::Catalog;

const DB_INDEX: &str = "/var/db/blitzae";
const WATCH_DIR: &str = "/tmp";

fn main() {

    let db = Db::start_default(DB_INDEX)
        .expect("Error on start the index database");

    let mut catalog = Catalog { db: db };

    let watch = fs::read_dir(Path::new(WATCH_DIR))
        .expect("Error on watch the input folder");

    for entry in watch {
        let file_tar = entry
            .expect("Error on get the entry");
        let path = file_tar.path();

        if path.is_file() {

            let file_name = path.file_name()
                .expect("Can't get the file name")
                .to_str()
                .expect("Can't get the string of file name")
                .to_string();

            if file_name.ends_with(".tar.gz") {
                println!("Indexing {}...", file_name);

                if catalog.is_indexed(&path) {
                    println!("Already indexed. Skiping...");
                    continue;
                }

                catalog.catalog_file(&path);
                println!("Indexing {}...OK", file_name);
            }
        }
    }

}
