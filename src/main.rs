///
/// Blitz Archiving Explorer
///
/// Main of the application
///
/// Copyright 2019 Luis Fernando Batels <luisfbatels@gmail.com>
///

use std::path::{Path, PathBuf};
use std::time::Duration;
use std::sync::mpsc::channel;
use std::fs;
use std::env;

extern crate notify;

use sled::Db;
use notify::{RecommendedWatcher, RecursiveMode, Watcher, DebouncedEvent};

mod files;

use files::catalog::Catalog;

const DB_INDEX: &str = "/var/db/blitzae";

fn main() {

    let input_folder_str = env::args().nth(1)
        .expect("Argument 1 needs to be the input folder");

    let input_folder = fs::read_dir(Path::new(&input_folder_str))
        .expect("Error on read the input folder");

    let db = Db::start_default(DB_INDEX)
        .expect("Error on start the index database");

    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(10))
        .expect("Error on start the watch service");

    let mut catalog = Catalog { db: db };

    // Index all current content
    for entry in input_folder {
        let file_tar = entry
            .expect("Error on get the entry");

        catalog.catalog_file(&file_tar.path());
    }

    // Index all new, or changed, content
    watcher.watch(input_folder_str.clone(), RecursiveMode::NonRecursive)
        .expect("Failed to watch for changes on the input folder!");

    println!("Waiting for changes in {}...", input_folder_str);
    loop {
        let change = rx.recv()
            .expect("Error on recv the change event");

        let (change_path, burn_path): (Option<PathBuf>, Option<PathBuf>) = match change {
            // New file
            DebouncedEvent::Create(pb) => (Some(pb.clone()), Some(pb)),
            // File updated
            DebouncedEvent::Write(pb) => (Some(pb.clone()), Some(pb)),
            // File removed
            DebouncedEvent::Remove(pb) => (None, Some(pb)),
            // File renamed
            DebouncedEvent::Rename(pb, pb2) => (Some(pb2), Some(pb)),
            _ => (None, None)
        };

        // In some cases we need remove the indexed content
        if let Some(path_buf) = burn_path {
            catalog.burn_catalog(&path_buf.as_path());
        }

        // Indexing the new content
        if let Some(path_buf) = change_path {
            catalog.catalog_file(&path_buf.as_path());
        }
    }
}
