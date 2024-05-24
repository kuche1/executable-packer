
// INFO
//
// library preload: https://stackoverflow.com/questions/426230/what-is-the-ld-preload-trick

use std::env;
use std::path::Path;
use std::fs;
use std::process::Command;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

fn main() -> std::io::Result<()>  {

    let args: Vec<String> = env::args().collect();
    // dbg!(&args);

    if args.len() != 2 {
        panic!("you need to specify exactly 1 argument: the path to the executable that you want to pack");
    }

    let executable: &String = &args[1];

    // check if executable exists

    let path = Path::new(executable);

    if !path.exists() {
        panic!("path to executable doesn't exist: {}", path.display());
    }

    // create root folder

    let executable_file_name = path.file_name()
        .expect("could not determine executable file name");

    let folder_root = executable_file_name;

    fs::create_dir(folder_root)
        .expect(&format!("could not create root folder `{}`", folder_root.to_str().unwrap()));
    
    // create bin folder

    let folder_bin = Path::new(folder_root).join("bin");

    fs::create_dir(&folder_bin)
        .expect("could not create bin folder");
    
    // create lib folder

    let folder_lib = Path::new(folder_root).join("lib");

    fs::create_dir(&folder_lib)
        .expect("could not create lib folder");

    // create folder for original executable

    let folder_original_executable = Path::new(folder_root).join("original_executable");

    fs::create_dir(&folder_original_executable)
        .expect("could not create folder for original executable");
    
    // copy original executable

    {
        let destination = folder_original_executable.join(executable_file_name);
        // let destination = folder_original_executable.join("original_executable");

        fs::copy(executable, destination)
            .expect("could not copy original executable");
    }

    // create executable file in bin

    {
        let new_file_path = folder_bin.join(executable_file_name);

        let mut file = File::create(&new_file_path)
            .expect("could not create file in bin folder");

        file.write_all(b"#! /usr/bin/env bash\n")?;
        file.write_all(b"set -euo pipefail\n")?;
        file.write_all(b"HERE=$(dirname $(readlink -f \"$BASH_SOURCE\"))\n")?;
        file.write_all(b"PRELOAD=$(readlink -f \"$HERE/../lib\")\n")?;
        file.write_all( &format!("LD_LIBRARY_PATH=\"$PRELOAD\" \"$HERE/../original_executable/{}\" $@\n", executable_file_name.to_str().unwrap()).as_bytes() )?;

        // make file executable

        Command::new("chmod")
            .arg("+x")
            .arg(new_file_path)
            .output()
            .expect("can't add executable permissions to file in bin folder");
    }

    // copy libs

    copy_dependencies_into_folder(
        &PathBuf::from(executable),
        &folder_lib
    );

    // return

    Ok(())

}

fn copy_dependencies_into_folder(executable: &PathBuf, folder_deps: &PathBuf) {

    // get libs used

    let ldd_info = 
        Command::new("ldd")
        .arg(executable)
        .output()
        .expect("can't run `ldd`");

    // dbg!(&ldd_info);

    let ldd_info = ldd_info.stdout;
    let ldd_info = String::from_utf8(ldd_info).expect("invalid utf-8 in ldd output");
    let ldd_info = ldd_info.replace("\t", "");
    
    // println!("\nldd info:\n{}", ldd_info);

    // println!();

    for line in ldd_info.split("\n") {
        let delimiter = " => ";

        if !line.contains(delimiter) {
            continue;
        }

        // println!("ldd line: {}", line);

        let split = line.split(delimiter);

        assert!(split.clone().count() == 2);

        let right = split.collect::<Vec<_>>()[1];

        // println!("right side: {}", right);

        assert!(right.chars().filter(|c| *c == ' ').count() == 1); // this will fail if there is space in th path

        let lib_source = right.split(" ").collect::<Vec<_>>()[0];

        println!("lib_source: {}", lib_source);

        // copy library

        let file_name = Path::new(lib_source).file_name().unwrap();

        let lib_destination = folder_deps.join(file_name);

        if !lib_destination.exists() {
            println!("file already exists: {}", lib_destination.display());
            if ! files_are_the_same_or_either_is_missing(Path::new(lib_source), lib_destination.as_path()) {
                panic!("libraries `{}` and `{}` differ", lib_source, lib_destination.display());
            }
        }

        // this overwrites files, so we make sure that the files are the same beforehand
        fs::copy(lib_source, &lib_destination)
            .expect("could not copy library");

        // resolve library reps
        // libs SHOULD auto detect libs if they're in the same folder

        copy_dependencies_into_folder(&lib_destination, folder_deps);
    }

}

use std::io::BufReader;
fn files_are_the_same_or_either_is_missing(file1: &Path, file2: &Path) -> bool {

    let f1 =
        match File::open(file1) {
            Ok(value) => value,
            Err(_err) => return true,
        };
    // let f1 = File::open(file1).unwrap();

    let f2 =
        match File::open(file2) {
            Ok(value) => value,
            Err(_err) => return true,
        };
    // let f2 = File::open(file2).unwrap();

    // Check if file sizes are different
    if f1.metadata().unwrap().len() != f2.metadata().unwrap().len() {
        return false;
    }

    // Use buf readers since they are much faster
    let f1 = BufReader::new(f1);
    let f2 = BufReader::new(f2);

    // Do a byte to byte comparison of the two files
    for (b1, b2) in f1.bytes().zip(f2.bytes()) {
        if b1.unwrap() != b2.unwrap() {
            return false;
        }
    }

    return true;
}
