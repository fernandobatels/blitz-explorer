///
/// Blitz Archiving Explorer
///
/// Representation of a indexed file
///
/// Copyright 2019 Luis Fernando Batels <luisfbatels@gmail.com>
///

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct File {
    pub size: u64,
    pub mtime: u64,
    pub file_name: String,
    pub full_path: String
}
