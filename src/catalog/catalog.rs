///
/// Blitz Archiving Explorer
///
/// Index/catalog of files content
///
/// Copyright 2019 Luis Fernando Batels <luisfbatels@gmail.com>
///

use std::fs::File;
use std::path::{Path, PathBuf};
use std::io::{BufReader, BufWriter, Write, copy};
use std::sync::Arc;
use std::str;

use flate2::read::GzDecoder;
use tar::Archive;
use sled::{Db, Tree};

use super::file::File as IndexedFile;
use super::file::FileTar;

pub struct Catalog {
   pub db: Db,
   pub cache_extract: String
}

impl Catalog {

    // Index the content of compressed file
    pub fn catalog_file(&mut self, path: &Path) -> Option<FileTar> {

        info!("Indexing {}...", path.display());

        if !path.is_file() {
            warn!("Is not a file {}. Skiping...", path.display());
            return None;
        }

        if !FileTar::path_to_string(path, false).ends_with(".tar.gz") {
            warn!("Is not a tar.gz file {}. Skiping...", path.display());
            return None;
        }

        let ftar = FileTar::from_path(path);
        if self.is_indexed(&ftar) {
            warn!("Already indexed {}. Skiping...", path.display());
            return None;
        }

        let archive = File::open(path);

        if let Err(e) = archive {
            error!("Can't open the file {}: {}. Skiping...", path.display(), e);
            return None;
        }

        let buffer_archive = BufReader::new(archive.unwrap());

        let decoder = GzDecoder::new(buffer_archive);
        let buffer_decoder = BufReader::new(decoder);

        let mut tar = Archive::new(buffer_decoder);

        let entries = tar.entries()
            .expect("Error on get the entries of tar file");

        let tree = self.get_tree(&ftar);

        for file in entries {

            let header = file
                .expect("Erro on get the entrie file header")
                .header()
                .clone();

            let full_path = &header.path()
                .expect("Can't get the full path");

            let indexed_file = IndexedFile {
                full_path: FileTar::path_to_string(full_path, true),
                file_name: FileTar::path_to_string(full_path, false),
                mtime: header.mtime()
                    .expect("Can't determine de mtime"),
                size: header.size()
                    .expect("Can't determine de size")
            };

            let data = serde_json::to_string(&indexed_file)
                .expect("Error on Serialize the file")
                .to_string();

            tree.set(FileTar::path_to_string(full_path, true).as_bytes(), data.as_bytes().to_vec())
                .expect("Error on create index for a file");
        }

        self.db.flush()
         .expect("Error on flush db");

        info!("Indexing {}...OK", path.display());

        return Some(ftar);
    }

    // Return the sled Tree object for access the indexed content
    // of a file
    fn get_tree(&mut self, tar: &FileTar) -> Arc<Tree> {

        let files = self.db.open_tree(tar.full_path.clone())
                .expect("Can't open the file tree");

        return files;
    }

    // Return the indexed files inside of the tar
    pub fn get_catalog(&mut self, tar: &FileTar) -> Vec<IndexedFile> {

        let tree = self.get_tree(tar);
        let mut files: Vec<IndexedFile> = vec![];

        for val in tree.iter().values() {

            let uval = val.expect("Error on get the val of indexed file");

            let file = str::from_utf8(&uval)
                .expect("Error on get string ut8 from indexed file");

            files.push(serde_json::from_str(&file)
                .expect("Error on Deserialize the file"));
        }

        return files;
    }

    // Return the list of indexed files(catalog's)
    pub fn get_catalogs(&mut self) -> Vec<FileTar> {
        let mut cats: Vec<FileTar> = vec![];

        for ucat in self.db.tree_names() {

            let cat = str::from_utf8(&ucat)
                .expect("Error on get string ut8 from catalog key");

            let path_buf = PathBuf::from(cat);

            cats.push(FileTar::from_path(path_buf.as_path()));
        }

        cats
    }

    // Return if the tar is already indexed
    pub fn is_indexed(&mut self, tar: &FileTar) -> bool {

        let tree = self.get_tree(tar);

        for _val in tree.iter().values() {
            return true;
        }

        return false;
    }

    // Burn/remove the indexed content, if exists, of the file tar
    pub fn burn_catalog(&mut self, tar: &FileTar) {

        info!("Burning {}...", tar.full_path);

        if !self.is_indexed(tar) {
            warn!("Not indexed {}. Skiping...", tar.full_path);
            return;
        }

        self.db.drop_tree(tar.full_path.as_bytes())
            .expect("Can't drop the file tree");

        info!("Burning {}...OK", tar.full_path);
    }

    // Extract a file from .tar file
    pub fn extract_file<W: Write>(&self, ftar: &FileTar, ffile: &IndexedFile, copy_to: &mut BufWriter<W>) -> bool {

        let (is_cached, cache) = self.cached_file(ftar, ffile);

        if is_cached {
            return copy(&mut BufReader::new(cache), copy_to).is_ok();
        }

        let path = Path::new(&ftar.full_path);
        let archive = File::open(path);

        if let Err(e) = archive {
            error!("Can't open the file {}: {}. Skiping...", path.display(), e);
            return false;
        }

        let buffer_archive = BufReader::new(archive.unwrap());

        let decoder = GzDecoder::new(buffer_archive);
        let buffer_decoder = BufReader::new(decoder);

        let mut tar = Archive::new(buffer_decoder);

        let entries = tar.entries()
            .expect("Error on get the entries of tar file");

        for entrie in entries {

            let file = entrie
                .expect("Erro on get the entrie file");

            let header = file.header().clone();

            let full_path = &header.path()
                .expect("Can't get the full path");

            if FileTar::path_to_string(full_path, true) == ffile.full_path {

                // Make the cache for use in the next requests
                copy(&mut BufReader::new(file), &mut BufWriter::new(cache))
                    .expect("Error on make the cache");

                // We get the content from cache
                return self.extract_file(ftar, ffile, copy_to);
            }
        }

        false
    }

    // Return the cache of indexed file, if exists
    fn cached_file(&self, ftar: &FileTar, ffile: &IndexedFile) -> (bool, File) {

        let cached_name = format!("{}/{}_{}", self.cache_extract, ftar.file_name, ffile.full_path.replace("/", ""));

        let path = Path::new(&cached_name);

        if path.exists() {
            let file = File::open(path)
                .expect("Cant open the cache file");

            return (true, file);
        }

        let file = File::create(path)
            .expect("Cant create the cache file");

        (false, file)
    }
}
