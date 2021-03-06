///
/// Blitz Explorer
///
/// Main of the application
///
/// Copyright 2019 Luis Fernando Batels <luisfbatels@gmail.com>
///

use std::path::{Path, PathBuf};
use std::time::Duration;
use std::sync::mpsc::channel;
use std::fs;
use std::ffi::OsStr;
use std::env;
use std::panic;
use std::thread;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

#[macro_use]
extern crate log;
extern crate simplelog;
extern crate notify;
extern crate flate2;
extern crate sled;
extern crate serde;
extern crate tar;
extern crate fuse;
extern crate libc;
extern crate time;

use simplelog::{SimpleLogger, LevelFilter, Config};
use sled::Db;
use notify::{RecommendedWatcher, RecursiveMode, Watcher, DebouncedEvent};
use fuse::mount;

mod catalog;
mod tcp;
mod filesystem;

use catalog::catalog::Catalog;
use catalog::file::FileTar;
use tcp::request::Request;
use filesystem::filesystem::TarInterface;

const DB_INDEX: &str = "/var/db/blitze";
const TCP_BIND: &str = "127.0.0.1:3355";
const CACHE_EXTRACT: &str = "/tmp";

fn main() {

    SimpleLogger::init(LevelFilter::Info, Config::default())
        .expect("Error on start the log");

    panic::set_hook(Box::new(|e| {
        error!("{}", e);
    }));

    let input_folder_str = env::args().nth(1)
        .expect("Argument 1 needs to be the input folder");

    let mountpoint = env::args().nth(2)
        .expect("Argument 2 needs to be the mountpoint of the output content");

    let mut only_run: Option<bool> = None; // true = tcp, false = fuse, none = both
    if let Some(on) = env::args().nth(3) {
        if on == "--only-tcp" {
            only_run = Some(true);
        } else if on == "--only-fuse" {
            only_run = Some(false);
        }
    }

    let input_folder = fs::read_dir(Path::new(&input_folder_str))
        .expect("Error on read the input folder");

    let db = Db::start_default(DB_INDEX)
        .expect("Error on start the index database");

    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(10))
        .expect("Error on start the watch service");

    let tcp_listener = TcpListener::bind(TCP_BIND)
        .expect("Error on bind the tcp port");

    let catalog = Arc::new(Mutex::new(Catalog {
        db: db,
        cache_extract: CACHE_EXTRACT.to_string()
    }));

    // Index all current content
    for entry in input_folder {
        let file_tar = entry
            .expect("Error on get the entry");

        catalog.lock().unwrap().catalog_file(&file_tar.path());
    }

    // Index all new, or changed, content
    watcher.watch(input_folder_str.clone(), RecursiveMode::NonRecursive)
        .expect("Failed to watch for changes on the input folder!");

    let catalog_indx = catalog.clone();
    let thread_indx = thread::spawn(move || {
        info!("Waiting for changes in {}...", input_folder_str);
        loop {
            let change = rx.recv()
                .expect("Error on recv the change event");

            let mut catalog_aux = catalog_indx.lock()
                .expect("Error on lock the catalog for file system");

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
                catalog_aux.burn_catalog(&FileTar::from_path(path_buf.as_path()));
            }

            // Indexing the new content
            if let Some(path_buf) = change_path {
                catalog_aux.catalog_file(&path_buf.as_path());
            }
        }
    });

    let catalog_tcp = catalog.clone();
    let thread_tcp = thread::spawn(move || {
        
        if let Some(on) = only_run {
            if !on {
                return;
            }
        }

        info!("Waiting for tcp connections in {}...", TCP_BIND);
        for stream in tcp_listener.incoming() {

            let client = stream.expect("Error on handle the tcp client");

            let mut catalog_aux = catalog_tcp.lock()
                .expect("Error on lock the catalog for tcp server");

            Request::handle(client, &mut catalog_aux);
        }
    });

    let mut catalog_fs = catalog.clone();
    let thread_fs = thread::spawn(move || {
        
        if let Some(on) = only_run {
            if on {
                return;
            }
        }

        let tar_interface = TarInterface {
            catalog: &mut catalog_fs,
            inodes: &mut HashMap::new(),
            itars: &mut HashMap::new()
        };

        let options: Vec<&OsStr> = vec![OsStr::new("-o"), OsStr::new("ro"), OsStr::new("-o"), OsStr::new("fsname=blitze")];

        mount(tar_interface, &mountpoint, &options)
            .expect("Error on mount the fuse");
    });

    thread_indx.join()
        .expect("Error on indexer thread");
    thread_tcp.join()
        .expect("Error on tcp thread");
    thread_fs.join()
        .expect("Error on fuse thread");
}
