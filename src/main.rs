///
/// Blitz Archiving Explorer
///
/// Main of the application
///
/// Copyright 2019 Luis Fernando Batels <luisfbatels@gmail.com>
///

use std::path::Path;

use sled::Db;

mod files;

use files::catalog::Catalog;

const DB_INDEX: &str = "/var/db/blitzae";

fn main() {

    let db = Db::start_default(DB_INDEX)
            .expect("Error on start the index database");

    let mut catalog = Catalog { db: db };

    let file_tar = "/tmp/testes.tar.gz".to_string();
    let path = Path::new(&file_tar);

    let files = catalog.get_catalog(path);

    if !files.is_empty() {

        for file in files {
            println!("{:?}", file);
        }

        return;
    }

    catalog.catalog_file(path);

}
