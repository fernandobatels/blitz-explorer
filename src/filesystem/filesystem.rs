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
use time;
use fuse::{FileType, FileAttr, Filesystem, Request, ReplyEntry, ReplyAttr, ReplyDirectory};

use catalog::catalog::Catalog;

pub struct TarInterface<'a> {
    pub catalog: &'a mut Catalog,
    pub inodes: &'a mut HashMap<(u64, String), u64> // (parent ino, name of file) => ino of file
}

const DELIMITATOR_INOS_TARS: u64 = 1000;

impl<'a> TarInterface<'a> {

    // Build the FileAttr values
    fn file_attr(ino: u64) -> FileAttr {

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
}

impl<'a> Filesystem for TarInterface<'a> {

    fn lookup(&mut self, _req: &Request, parent: u64, name_osstr: &OsStr, reply: ReplyEntry) {

        let name = name_osstr.to_str()
            .expect("Error on OsStr to String")
            .to_string();

        info!("lookup: {} {}", parent, name);

        let ino = self.inodes.get(&(parent, name))
            .expect("Inode not found!");

        // For list the tar.gz files
        if parent < DELIMITATOR_INOS_TARS {

            let attr = TarInterface::file_attr(*ino);

            reply.entry(&time::now().to_timespec(), &attr, 0);

            return;
        }

        reply.error(ENOENT);
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        info!("getattr: {}", ino);

        // Root path of the mounted dir
        if ino == 1 {

            reply.attr(&time::now().to_timespec(), &TarInterface::file_attr(ino));

            return;
        }

        reply.error(ENOENT);
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {

        // TODO: Paginate this!
        if offset == 0 {

            reply.add(ino + 1, 1, FileType::Directory, ".");
            self.inodes.insert((ino, ".".to_string()), ino + 1);
            reply.add(ino + 1, 1, FileType::Directory, "..");
            self.inodes.insert((ino, "..".to_string()), ino + 1);

            let mut i = 2;

            for entry in self.catalog.get_catalogs() {
                reply.add(ino + i, i as i64, FileType::Directory, entry.file_name.clone());
                self.inodes.insert((ino, entry.file_name), ino + 1);
                i = i + 1;
            }
        }

        reply.ok();
    }
}
