///
/// Blitz Archiving Explorer
///
/// Representation of a indexed file and the tar file
///
/// Copyright 2019 Luis Fernando Batels <luisfbatels@gmail.com>
///

use std::path::Path;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct File {
    pub size: u64,
    pub mtime: u64,
    pub file_name: String,
    pub full_path: String,
    pub is_file: bool,
    pub level_path: usize,
    pub ino: u64
}

pub struct FileTar {
    pub file_name: String,
    pub full_path: String
}

impl FileTar {

    // Create the object of FileTar from a Path object
    pub fn from_path(path: &Path) -> FileTar {
        FileTar {
            full_path: FileTar::path_to_string(path, true),
            file_name: FileTar::path_to_string(path, false)
        }
    }

    // Simplify the path -> string
    pub fn path_to_string(path: &Path, full: bool) -> String {

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
