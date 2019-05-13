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
use std::collections::{HashMap, LinkedList};

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

        let mut parents_inos: HashMap<String, (u64, LinkedList<u64>)> = HashMap::new();

        { // Root dir of tar file
            let ino = self.get_last_ino() + 1;

            parents_inos.insert("".to_string(), (ino, LinkedList::new()));

            self.set_last_ino(ino);
        }

        for file in entries {

            let header = file
                .expect("Erro on get the entrie file header")
                .header()
                .clone();

            let full_path = &header.path()
                .expect("Can't get the full path");

            let ino = self.get_last_ino() + 1;

            let full_path_str = FileTar::path_to_string(full_path, true);
            let file_name_str = FileTar::path_to_string(full_path, false);

            let mut level = full_path_str.clone().matches("/").count();

            if header.entry_type().is_file() {
                level = level + 1;
            }

            let parent_n = full_path_str.rfind(file_name_str.as_str())
                .expect("Error on get the pos of file name");
            let parent = &full_path_str.clone()[..parent_n];

            if header.entry_type().is_dir() && !parents_inos.contains_key(&full_path_str) {
                parents_inos.insert(full_path_str.clone(), (ino, LinkedList::new()));
            }

            if let Some(parent_list) = parents_inos.get_mut(parent) {
                parent_list.1.push_back(ino);
            }

            let indexed_file = IndexedFile {
                full_path: full_path_str,
                file_name: file_name_str,
                mtime: header.mtime()
                    .expect("Can't determine de mtime"),
                size: header.size()
                    .expect("Can't determine de size"),
                is_file: header.entry_type().is_file(),
                level_path: level,
                ino: ino
            };

            let data = serde_json::to_string(&indexed_file)
                .expect("Error on Serialize the file")
                .to_string();

            tree.set(FileTar::path_to_string(full_path, true).as_bytes(), data.as_bytes().to_vec())
                .expect("Error on create index for a file");

            self.set_last_ino(ino);
        }

        for (_parent, inos) in parents_inos {
            let files = self.get_tree_inos(inos.0);
            for file in inos.1 {
                files.set(file.to_string().as_bytes(), file.to_string().as_bytes().to_vec())
                    .expect("Error on set the index tree ino");
            }
        }

        self.db.flush()
         .expect("Error on flush db");

        info!("Indexing {}...OK", path.display());

        return Some(ftar);
    }

    // Return the sled Tree object for access the indexed content
    // of a file
    fn get_tree(&mut self, tar: &FileTar) -> Arc<Tree> {

        let files = self.db.open_tree(format!("tar::{}", tar.file_name.clone()))
                .expect("Can't open the file tree");

        return files;
    }

    // Return the sled Tree object for access the indexed content
    // of a ino tree cache
    fn get_tree_inos(&mut self, ino: u64) -> Arc<Tree> {

        let internal_files = self.db.open_tree(format!("inotree::{}", ino))
                .expect("Can't open the ino tree");

        return internal_files;
    }

    // Update the last used ino on files
    fn set_last_ino(&mut self, ino: u64) {

        self.db.set("last_ino".to_string(), ino.to_string().as_bytes().to_vec())
            .expect("Error on update the last ino");
    }

    // Return the last used ino on files
    fn get_last_ino(&mut self) -> u64 {

        if let Ok(valop) = self.db.get("last_ino".to_string()) {
            if let Some(val) = valop {
                let inostr = str::from_utf8(&val)
                    .expect("Error on get last ino from db");

                if let Ok(ino) = inostr.parse::<u64>() {
                    return ino;
                }
            }
        }

        return 20000;
    }

    // Return the childs of the ino
    pub fn get_files_inos(&mut self, ino: u64) -> Vec<u64> {

        let sub_inos = self.get_tree_inos(ino);

        let mut files: Vec<u64> = vec![];

        for val in sub_inos.iter().values() {

            let uval = val.expect("Error on get the val of indexed ino");

            let sub_ino_str = str::from_utf8(&uval)
                .expect("Error on get string ut8 from indexed ino");

            let sub_ino = sub_ino_str.parse::<u64>()
                .expect("Error on parse the ino str");

            files.push(sub_ino);
        }

        files
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

            if !cat.starts_with("tar::") {
                continue;
            }

            let catn = cat.replacen("tar::", "", 1);

            let path_buf = PathBuf::from(catn);

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
