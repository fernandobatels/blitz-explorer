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

use libc::ENOENT;
use time::{self, Timespec};
use fuse::{FileType, FileAttr, Filesystem, Request, ReplyEntry, ReplyAttr, ReplyDirectory};

use catalog::catalog::Catalog;
use catalog::file::{File, FileTar};

pub struct TarInterface<'a> {
    pub catalog: &'a mut Catalog,
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
    fn def_file(name: String, is_file: bool) -> File {

        File {
            full_path: name.clone(),
            file_name: name.clone(),
            mtime: 0,
            size: 0,
            is_file: is_file,
            level_path: 1
        }
    }
}

impl<'a> Filesystem for TarInterface<'a> {

    fn lookup(&mut self, _req: &Request, parent: u64, name_osstr: &OsStr, reply: ReplyEntry) {

        let name = name_osstr.to_str()
            .expect("Error on OsStr to String")
            .to_string();

        let ino_file_parent = (parent, TarInterface::def_file("".to_string(), false));

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

            let mut files: Vec<File> = vec![];
            let mut next_ino = 0;
            let mut file_name_tar: Option<String> = None;

            files.push(TarInterface::def_file(".".to_string(), false));
            files.push(TarInterface::def_file("..".to_string(), false));

            if ino == 1 {
                // Root dir

                for tar in self.catalog.get_catalogs() {
                    files.push(TarInterface::def_file(tar.file_name, false));
                }

            } else  {
                // Inside a tar file or a internal folder
                let mut level_filter: usize = 1;
                let mut path_filter = "".to_string();

                if let Some(tar_name) = self.itars.get(&ino) {
                    // Internal folder
                    file_name_tar = Some(tar_name.to_string());

                    for (_key, val) in self.inodes.iter() {
                        if val.0 == ino {
                            level_filter = (val.1.level_path + 1).clone();
                            path_filter = (&val.1).full_path.clone();
                            break;
                        }
                    }

                    if path_filter.ends_with(".") {
                        level_filter = 1;
                        path_filter = "".to_string();
                    }

                } else {
                    // Folder inside the tar

                    for (key, val) in self.inodes.iter() {
                        if val.0 == ino {
                            file_name_tar = Some(key.1.clone());
                            break;
                        }
                    }
                }

                if let Some(tar_name) = file_name_tar.clone() {

                    let tar = FileTar {
                        file_name: tar_name.clone(),
                        full_path: tar_name
                    };

                    // TODO: Make this more fast!
                    for file in self.catalog.get_catalog(&tar) {
                        if file.level_path == level_filter && file.full_path.starts_with(&path_filter) {
                            files.push(file);
                        }
                    }
                }
            }

            for entry in files {

                if entry.is_file {
                    reply.add(ino + next_ino, next_ino as i64, FileType::Directory, entry.file_name.clone());
                } else {
                    reply.add(ino + next_ino, next_ino as i64, FileType::RegularFile, entry.file_name.clone());
                }

                self.inodes.insert((ino, entry.file_name.clone()), (ino + next_ino, entry));

                if let Some(tar_name) = file_name_tar.clone() {
                    self.itars.insert(ino + next_ino, tar_name);
                }

                next_ino = next_ino + 1;
            }

        }

        reply.ok();
    }
}
