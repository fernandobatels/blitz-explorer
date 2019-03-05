///
/// Blitz Archiving Explorer
///
/// Main of the application
///
/// Copyright 2018 Luis Fernando Batels <luisfbatels@gmail.com>
///


extern crate flate2;
extern crate tar;

use flate2::read::GzDecoder;

use std::fs::File;
use std::io::Read;
use tar::Archive;

fn main() {

    let mut buffer : Vec<u8> = Vec::new();
    let mut archive = File::open("/tmp/testes.tar.gz".to_string()).unwrap();
    archive.read_to_end(&mut buffer).unwrap();

    let mut decoder = GzDecoder::new(&buffer[..]);
    let mut decoded_buffer : Vec<u8> = Vec::new();

    decoder.read_to_end(&mut decoded_buffer).unwrap();

    let mut archive = Archive::new(&decoded_buffer[..]);

    for file in archive.entries().unwrap() {
        println!("{:?}", file.unwrap().header());
    }

}
