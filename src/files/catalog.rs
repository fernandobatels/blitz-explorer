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

        println!("Indexing {}...", path.display());

        if !path.is_file() {
            println!("Is not a file {}. Skiping...", path.display());
            return;
        }

        if !Catalog::path_to_string(path, false).ends_with(".tar.gz") {
            println!("Is not a tar.gz file {}. Skiping...", path.display());
            return;
        }

        if self.is_indexed(path) {
            println!("Already indexed {}. Skiping...", path.display());
            return;
        }

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

            let full_path = &header.path()
                .expect("Can't get the full path");

            let indexed_file = IndexedFile {
                full_path: Catalog::path_to_string(full_path, true),
                file_name: Catalog::path_to_string(full_path, false),
                mtime: header.mtime()
                    .expect("Can't determine de mtime"),
                size: header.size()
                    .expect("Can't determine de size")
            };

            let data = serde_json::to_string(&indexed_file)
                .expect("Error on Serialize the file")
                .to_string();

            tree.set(Catalog::path_to_string(full_path, true).as_bytes(), data.as_bytes().to_vec())
                .expect("Error on create index for a file");
        }

        self.db.flush()
         .expect("Error on flush db");

        println!("Indexing {}...OK", path.display());
    }

    // Return the sled Tree object for access the indexed content
    // of a file
    fn get_tree(&mut self, path: &Path) -> Arc<Tree> {

        let files = self.db.open_tree(Catalog::path_to_string(path, false))
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

    // Return if the tar is already indexed
    pub fn is_indexed(&mut self, path: &Path) -> bool {

        let tree = self.get_tree(path);

        for _val in tree.iter().values() {
            return true;
        }

        return false;
    }

    // Burn/remove the indexed content, if exists, of the file tar
    pub fn burn_catalog(&mut self, path: &Path) {

        println!("Burning {}...", path.display());

        if !self.is_indexed(path) {
            println!("Not indexed {}. Skiping...", path.display());
            return;
        }

        self.db.drop_tree(Catalog::path_to_string(path, false).as_bytes())
            .expect("Can't drop the file tree");

        println!("Burning {}...OK", path.display());
    }

    // Simplify the path -> string
    fn path_to_string(path: &Path, full: bool) -> String {

        if full {
            return path.to_str()
                .expect("Can't get the string of full path")
                .to_string();
        }

        return path.file_name()
            .expect("Can't get the file name")
            .to_str()
            .expect("Can't get the string of file name")
            .to_string();
    }
}
