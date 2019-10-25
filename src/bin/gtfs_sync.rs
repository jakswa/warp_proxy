#![deny(warnings)]
extern crate reqwest;
extern crate zip;
extern crate dirs;

use std::error::Error;
use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use zip::read::ZipArchive;

fn main() {
    match download() {
        Ok(file) => process_zip(file),
        Err(err) => println!("uh oh... {:?}", err)
    }
}

fn process_zip(file: File) {
    match ZipArchive::new(file) {
        Ok(zip_file) => loop_files(zip_file),
        Err(err) => println!("ewww: {:?}", err)
    }
}

fn loop_files(mut archive: ZipArchive<File>) {
    for i in 0..archive.len() {
        let csv = archive.by_index(i).unwrap();
        println!("file: {}", csv.name());
        match csv.name() {
            "stops.txt" => {
                let reader = BufReader::new(csv);
                let first_line = reader.lines().next().unwrap();
                println!("line1: {:?}", first_line);
            },
            _ => {}
        }
    }
}

fn download() -> Result<File, Box<dyn Error>> {
    let dir = dirs::home_dir().unwrap().join("marta_gtfs.zip");
    match File::open(&dir) {
        Ok(file) => return Ok(file),
        Err(_) => {
            let mut file = std::fs::OpenOptions::new()
                .create(true).write(true).read(true)
                .open(dir)?;
            let mut resp = reqwest::get("https://www.itsmarta.com/google_transit_feed/google_transit.zip")?;
            resp.copy_to(&mut file)?;
            Ok(file)
        }
    }
}
