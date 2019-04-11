///
/// Blitz Archiving Explorer
///
/// Index/catalog of files content
///
/// Copyright 2019 Luis Fernando Batels <luisfbatels@gmail.com>
///

use std::fs::File;
use std::path::Path;
use std::io::BufReader;
use std::sync::Arc;

use flate2::read::GzDecoder;
use tar::Archive;
use sled::{Db, Tree};

use super::file::File as IndexedFile;

pub struct Catalog {
   pub db: Db
}

impl Catalog {

    // Index the content of compressed file
    pub fn catalog_file(&mut self, path: &Path) {

        let archive = File::open(path);

        if let Err(e) = archive {
            eprintln!("Can't open the file {}: {}", path.display(), e);
            return;
        }

        let buffer_archive = BufReader::new(archive.unwrap());

        let decoder = GzDecoder::new(buffer_archive);
        let buffer_decoder = BufReader::new(decoder);

        let mut tar = Archive::new(buffer_decoder);

        let entries = tar.entries()
            .expect("Error on get the entries of tar file");

        let tree = self.get_tree(path);

        for file in entries {

            let header = file
                .expect("Erro on get the entrie file header")
                .header()
                .clone();

            let full_path = header.path()
                .expect("Can't get the full path");

            let full_path_str = full_path.to_str()
                .expect("Can't get the string of full path")
                .to_string();

            let file_path = full_path.file_name()
                .expect("Can't get the file name")
                .to_str()
                .expect("Can't get the string of file name")
                .to_string();

            let indexed_file = IndexedFile {
                full_path: full_path_str.clone(),
                file_name: file_path.clone(),
                mtime: header.mtime()
                    .expect("Can't determine de mtime"),
                size: header.size()
                    .expect("Can't determine de size")
            };

            let data = serde_json::to_string(&indexed_file)
                .expect("Error on Serialize the file")
                .to_string();

            tree.set(full_path_str.as_bytes(), data.as_bytes().to_vec())
                .expect("Error on create index for a file");
        }

        self.db.flush()
         .expect("Error on flush db");
    }

    // Return the sled Tree object for access the indexed content
    // of a file
    fn get_tree(&mut self, path: &Path) -> Arc<Tree> {

        let file_name = path.file_name()
            .expect("Can't get the file name")
            .to_str()
            .expect("Can't get the string of file name")
            .to_string();

        let files = self.db.open_tree(file_name)
                .expect("Can't open the file tree");

        return files;
    }

    // Return the indexed files inside of the tar
    pub fn get_catalog(&mut self, path: &Path) -> Vec<IndexedFile> {

        let tree = self.get_tree(path);
        let mut files: Vec<IndexedFile> = vec![];

        for val in tree.iter().values() {

            let uval = val.expect("Error on get the val of indexed file");

            let file = std::str::from_utf8(&uval)
                .expect("Error on get string ut8 from indexed file");

            files.push(serde_json::from_str(&file)
                .expect("Error on Deserialize the file"));
        }

        return files;
    }
}
