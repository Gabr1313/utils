use cp::threadpool::ThreadPool;
use std::{env, error::Error, fs, io::Write, path::PathBuf, process, str, sync::Arc};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

static OUTPUT_DIR: &str = "output";
static INPUT_TAG: &str = "input";
static COL: usize = 80;

static OS: Os = if cfg!(unix) {
    Os::Unix
} else if cfg!(windows) {
    Os::Windows
} else {
    panic!("Unsupported OS");
};

#[derive(Copy, Clone)]
enum Os {
    Windows,
    Unix,
}

fn main() -> Result<()> {
    let (file_name, flags, n_threads) = process_args();
    let current_dir = env::current_dir().expect("Can't get current directory");
    let inputs = get_input_files(&current_dir)?;
    let build_arg = cmd_args(&file_name, flags[0], flags[2]);
    let build_status = process::Command::new("g++").args(build_arg).status()?;
    if build_status.success() {
        if flags[1] {
            create_empty_folder(&current_dir)?;
        }
        let binary = match OS {
            Os::Windows => ".\\a.exe",
            Os::Unix => "./a.out",
        };
        run_test_cases(binary, inputs, flags[1], n_threads)?;
        fs::remove_file(binary)?;
    }
    Ok(())
}

fn process_args() -> (String, Vec<bool>, usize) {
    let mut file_name = String::new();
    let mut flags = vec![false; 3];
    let mut n_threads = 1;
    for arg in env::args().into_iter().skip(1) {
        let bytes = arg.as_bytes();
        if bytes[0] == b'-' {
            if bytes[1] == b'-' {
                match arg.as_str() {
                    "--release" => flags[0] = true,
                    "--output-file" => flags[1] = true,
                    "--parallel" => {
                        let start = "--parallel".len();
                        let end = change_n_threads(&mut n_threads, bytes, start);
                        if end < arg.len() {
                            panic!("Invalid arguments {arg}. Try to use -h flag");
                        }
                    }
                    "--warning" => flags[2] = true,
                    "--help" => {
                        print_help();
                        std::process::exit(0);
                    }
                    _ => panic!("Invalid arguments {arg}. Try to use -h flag"),
                }
            } else {
                let mut i = 1;
                while i < bytes.len() {
                    match bytes[i] {
                        b'r' => flags[0] = true,
                        b'o' => flags[1] = true,
                        b'p' => {
                            i = change_n_threads(&mut n_threads, bytes, i + 1) - 1;
                        }
                        b'w' => flags[2] = true,
                        b'h' => {
                            print_help();
                            std::process::exit(0);
                        }
                        _ => panic!("Invalid arguments {arg}. Try to use -h flag"),
                    }
                    i += 1;
                }
            }
        } else {
            match file_name.as_str() {
                "" => file_name = arg,
                _ => panic!("Invalid arguments {arg}. Try to use -h flag"),
            }
        }
    }
    if file_name.is_empty() {
        panic!("No input file specified. Try to use -h flag");
    }
    (file_name, flags, n_threads)
}

fn change_n_threads(n_threads: &mut usize, bytes: &[u8], start: usize) -> usize {
    let mut j = start;
    while j < bytes.len() && bytes[j] >= b'0' && bytes[j] <= b'9' {
        j += 1;
    }
    *n_threads = std::thread::available_parallelism().unwrap().get();
    if j > start {
        *n_threads = str::from_utf8(&bytes[start..j])
            .unwrap()
            .parse::<usize>()
            .unwrap()
            .min(*n_threads);
    }
    j
}

fn print_help() {
    println!("Usage: cpp-bin [file_name] [option]");
    println!();
    println!("Options:");
    println!("  -r    --release      Build using -Ofast");
    println!("  -o    --output-file  Create output folder");
    println!("  -p[n] --parallel[n]  Run test cases in parallel");
    println!("                       If specified it spawns `n` threads, otherwise it spawns all aviable ones");
    println!("  -w    --warning      Show warning messages");
    println!("  -h    --help         Print this help message");
    println!("Note: you can also aggregate options like `-rp4o`");
    println!();
    println!("The name of the input files should contain `{INPUT_TAG}`.");
    println!(
        "Otherwise the input files should be placed in a folder whose name contains `{INPUT_TAG}`."
    );
}

fn get_input_files(dir: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut inputs = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?.path();
        if entry.is_dir() {
            let folder = entry;
            if folder.to_str().unwrap().to_lowercase().contains(INPUT_TAG) {
                for file in fs::read_dir(&folder)? {
                    let file = file?.path();
                    if file.is_file() {
                        inputs.push(file);
                    }
                }
            }
        } else if entry.is_file() {
            let file = entry;
            let name = match OS {
                Os::Windows => file.to_str().unwrap().split('\\').last().unwrap(),
                Os::Unix => file.to_str().unwrap().split('/').last().unwrap(),
            };
            if name.contains(INPUT_TAG) {
                inputs.push(file);
            }
        }
    }
    inputs.sort_unstable();
    Ok(inputs)
}

fn cmd_args<'a>(file_name: &'a str, release: bool, warning: bool) -> Vec<&'a str> {
    let mut args = if release {
        vec!["-Ofast", file_name]
    } else {
        vec!["-O0", "-fsanitize=address", file_name]
    };
    if warning {
        args.append(&mut ["-Wall", "-Wextra"].to_vec());
    }
    args
}

fn create_empty_folder(current_dir: &PathBuf) -> Result<()> {
    for entry in current_dir.read_dir()? {
        let entry = entry.unwrap().path();
        let name = match OS {
            Os::Windows => entry.to_str().unwrap().split('\\').last().unwrap(),
            Os::Unix => entry.to_str().unwrap().split('/').last().unwrap(),
        };
        if name == OUTPUT_DIR {
            if entry.is_dir() {
                fs::remove_dir_all(entry)?;
            } else if entry.is_file() {
                fs::remove_file(entry)?;
            }
            break;
        }
    }
    fs::create_dir(OUTPUT_DIR)?;
    Ok(())
}

fn run_test_cases(
    binary: &str,
    inputs: Vec<PathBuf>,
    output_file: bool,
    n_threads: usize,
) -> Result<()> {
    if n_threads > 1 {
        let binary: Arc<str> = binary.into();
        let pool = ThreadPool::new(n_threads);
        if output_file {
            for (i, input_file) in inputs.into_iter().enumerate() {
                let binary = Arc::clone(&binary);
                pool.execute(move || {
                    run(binary.as_ref(), &input_file, Some(i)).unwrap();
                });
            }
        } else {
            for input_file in inputs.into_iter() {
                let binary = Arc::clone(&binary);
                pool.execute(move || {
                    run(binary.as_ref(), &input_file, None).unwrap();
                });
            }
        }
    } else {
        if output_file {
            for (i, input_file) in inputs.into_iter().enumerate() {
                run(binary, &input_file, Some(i)).unwrap();
            }
        } else {
            for input_file in inputs.into_iter() {
                run(binary, &input_file, None).unwrap();
            }
        }
    }
    Ok(())
}

fn run(binary: &str, input_file: &PathBuf, file_number: Option<usize>) -> Result<()> {
    let mut process = process::Command::new(binary)
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = process.stdin.take() {
        stdin.write_all(&fs::read(&input_file)?)?;
    }
    let start = std::time::Instant::now();
    let output = process.wait_with_output()?;
    let end = std::time::Instant::now();

    let mut stdout = std::io::stdout().lock();
    if let Some(i) = file_number {
        print_cool(
            &format!(
                "{}: {}ms (output.{}.txt)",
                match OS {
                    Os::Windows => input_file.to_str().unwrap().split('\\').last().unwrap(),
                    Os::Unix => input_file.to_str().unwrap().split('/').last().unwrap(),
                },
                (end - start).as_millis(),
                i
            ),
            &mut stdout,
        )?;
        fs::write(
            format!(
                "{}{}output.{}.txt",
                OUTPUT_DIR,
                match OS {
                    Os::Windows => "\\",
                    Os::Unix => "/",
                },
                i
            ),
            unsafe { String::from_utf8_unchecked(output.stdout) },
        )?;
    } else {
        print_cool(
            &format!(
                "{}: {}ms",
                match OS {
                    Os::Windows => input_file.to_str().unwrap().split('\\').last().unwrap(),
                    Os::Unix => input_file.to_str().unwrap().split('/').last().unwrap(),
                },
                (end - start).as_millis()
            ),
            &mut stdout,
        )?;
        stdout.write_fmt(format_args!("{}", unsafe {
            String::from_utf8_unchecked(output.stdout)
        }))?;
    }
    if !output.stderr.is_empty() {
        print_cool("(stderr)", &mut stdout)?;
        stdout.write_fmt(format_args!("{}", unsafe {
            String::from_utf8_unchecked(output.stderr)
        }))?;
    }
    Ok(())
}

fn print_cool<W: Write>(mid: &str, stdout: &mut W) -> Result<()> {
    let occupied = 4 + mid.len();
    let n1 = if occupied >= COL {
        0
    } else {
        (COL - occupied) / 2
    };
    let occupied = 4 + mid.len() + n1;
    let n2 = if occupied >= COL { 0 } else { COL - occupied };
    stdout.write_fmt(format_args!(
        "{}> {} <{}\n",
        "-".repeat(n1),
        mid,
        "-".repeat(n2),
    ))?;
    Ok(())
}
