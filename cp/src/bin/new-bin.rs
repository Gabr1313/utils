use std::{env, fs, io::Write, path};

fn main() {
    let mut args = env::args().into_iter();
    let file_name = args.nth(1).expect("Enter file_path");
    let content = match args.next().unwrap_or("0".to_string()).as_str() {
        "0" => include_bytes!("template.rs").to_vec(),
        "1" => include_bytes!("template_local.rs").to_vec(),
        "2" => include_bytes!("template_safe.rs").to_vec(),
        v => panic!("Invalid version: {}. 0..=2 are available", v),
    };
    let current_dir = env::current_dir()
        .expect("Can't get current directory")
        .into_os_string()
        .into_string()
        .unwrap();
    let path = format!("{}/src/bin/{}.rs", current_dir, file_name);
    let path = path::Path::new(&path);
    if path.is_file() {
        panic!("File {} already exists", path.display());
    }
    let mut file = match fs::File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", path.display(), why),
        Ok(file) => file,
    };
    file.write_all(&content).expect("Unable to write data");
}
