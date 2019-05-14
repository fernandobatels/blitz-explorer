///
/// Blitz Archiving Explorer
///
/// The interface of the filesystem. With this the user can access
/// the content of tar.gz files
///
/// Copyright 2019 Luis Fernando Batels <luisfbatels@gmail.com>
///

use std::ffi::OsStr;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use libc::ENOENT;
use time::{self, Timespec};
use fuse::{FileType, FileAttr, Filesystem, Request, ReplyEntry, ReplyAttr, ReplyDirectory};

use catalog::catalog::Catalog;
use catalog::file::{File, FileTar};

pub struct TarInterface<'a> {
    pub catalog: &'a mut Arc<Mutex<Catalog>>,
    pub inodes: &'a mut HashMap<(u64, String), (u64, File)>, // (parent ino, name of file) => (ino of file, File)
    pub itars: &'a mut HashMap<u64, String> // ino of file => tar parent
}

impl<'a> TarInterface<'a> {

    // Build the default FileAttr values
    fn def_file_attr(ino: u64) -> FileAttr {

        FileAttr {
            ino: ino,
            size: 0,
            blocks: 0,
            atime: time::now().to_timespec(),
            mtime: time::now().to_timespec(),
            ctime: time::now().to_timespec(),
            crtime: time::now().to_timespec(),
            kind: FileType::Directory,
            perm: 0o444,
            nlink: 2,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
        }
    }

    // Build the default value for File
    fn def_file(name: String, is_file: bool, ino: u64) -> File {

        File {
            full_path: name.clone(),
            file_name: name.clone(),
            mtime: 0,
            size: 0,
            is_file: is_file,
            level_path: 1,
            ino: ino
        }
    }
}

impl<'a> Filesystem for TarInterface<'a> {

    fn lookup(&mut self, _req: &Request, parent: u64, name_osstr: &OsStr, reply: ReplyEntry) {

        let name = name_osstr.to_str()
            .expect("Error on OsStr to String")
            .to_string();

        let ino_file_parent = (parent, TarInterface::def_file("".to_string(), false, 1));

        let (ino, file) = match self.inodes.get(&(parent, name)) {
            Some(aux) => aux,
            None => &ino_file_parent
        };

        let mut attr = TarInterface::def_file_attr(*ino);

        if file.is_file {
            attr.kind = FileType::RegularFile;
        }

        attr.mtime = Timespec::new(file.mtime as i64, 0);
        attr.size = file.size;

        reply.entry(&time::now().to_timespec(), &attr, 0);
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        info!("getattr: {}", ino);

        // Root path of the mounted dir
        if ino == 1 {

            reply.attr(&time::now().to_timespec(), &TarInterface::def_file_attr(ino));

            return;
        }

        reply.error(ENOENT);
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {

        // TODO: Paginate this!
        if offset == 0 {

            let mut files: Vec<(File, String)> = vec![];
            let mut catalog = self.catalog.lock()
                .expect("Error on lock the catalog for fuse");

            if ino == 1 {
                // Root dir
                let mut ino_tar = 2;

                for tar in catalog.get_catalogs() {
                    files.push((TarInterface::def_file(tar.file_name.clone(), false, ino_tar), tar.file_name.to_string()));
                    ino_tar = ino_tar + 1;
                }

            } else if let Some(tar_name) = self.itars.get(&ino) {
                // Inside a tar file or a internal folder

                let tar = FileTar {
                    file_name: tar_name.clone(),
                    full_path: tar_name.clone()
                };

                let catalog_files = catalog.get_catalog(&tar);

                if ino >= 20000 {
                    let inos_filter = catalog.get_files_inos(ino);
                    for file in catalog_files {
                        if inos_filter.contains(&file.ino) {
                            files.push((file, tar_name.to_string()));
                        }
                    }
                } else {
                    for file in catalog_files {
                        if file.level_path == 1 {
                            files.push((file, tar_name.to_string()));
                        }
                    }
                }
            }

            for (entry, tar_name) in files {

                let next_ino = entry.ino;

                if entry.is_file {
                    reply.add(next_ino, next_ino as i64, FileType::Directory, entry.file_name.clone());
                } else {
                    reply.add(next_ino, next_ino as i64, FileType::RegularFile, entry.file_name.clone());
                }

                self.inodes.insert((ino, entry.file_name.clone()), (next_ino, entry));

                if !tar_name.is_empty() {
                    self.itars.insert(next_ino, tar_name);
                }
            }

        }

        reply.ok();
    }
}
