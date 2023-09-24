use std::error::Error;
use std::io::Write;
use std::{env, fs, path::PathBuf, process, str};
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

static OUTPUT_DIR: &str = "output";
static INPUT_TAG: &str = "input";
static COL: usize = 80;

#[derive(Copy, Clone)]
enum Os {
    Windows,
    Unix,
}

fn main() -> Result<()> {
    let os = if cfg!(windows) {
        Os::Windows
    } else if cfg!(unix) {
        Os::Unix
    } else {
        panic!("Unsupported OS");
    };
    let (file_name, flags) = process_args(os);
    let current_dir = env::current_dir().expect("Can't get current directory");
    let inputs = get_input_files(&current_dir, os)?;
    let (build_arg, binary) = get_cmd(&current_dir, &file_name, flags[0], os)?;
    let build_status = process::Command::new("cargo").args(build_arg).status()?;
    if build_status.success() {
        if flags[1] {
            create_empty_folder(&current_dir, os)?;
        }
        run_test_cases(inputs, &binary, &flags, os)?;
    }
    Ok(())
}

fn process_args(os: Os) -> (String, Vec<bool>) {
    let mut file_name = String::new();
    let mut flags = vec![false; 3];
    for arg in env::args().into_iter().skip(1) {
        let mut iter = arg.chars().peekable();
        if iter.next() == Some('-') {
            if iter.peek() == Some(&'-') {
                match arg.as_str() {
                    "--release" => flags[0] = true,
                    "--output-file" => flags[1] = true,
                    "--parallel" => flags[2] = true,
                    "--help" => {
                        print_help();
                        std::process::exit(0);
                    }
                    _ => panic!("Invalid arguments {}. Try to use -h flag", arg),
                }
            } else {
                for c in iter {
                    match c {
                        'r' => flags[0] = true,
                        'o' => flags[1] = true,
                        'p' => flags[2] = true,
                        'h' => {
                            print_help();
                            std::process::exit(0);
                        }
                        _ => panic!("Invalid arguments {}. Try to use -h flag", arg),
                    }
                }
            }
        } else {
            match file_name.as_str() {
                "" => match os {
                    Os::Windows => {
                        file_name = arg
                            .split('\\')
                            .last()
                            .unwrap()
                            .split('.')
                            .next()
                            .unwrap()
                            .to_string()
                    }
                    Os::Unix => {
                        file_name = arg
                            .split('/')
                            .last()
                            .unwrap()
                            .split('.')
                            .next()
                            .unwrap()
                            .to_string()
                    }
                },
                _ => panic!("Invalid arguments {}. Try to use -h flag", arg),
            }
        }
    }
    if file_name.is_empty() {
        panic!("No input file specified. Try to use -h flag");
    }
    (file_name, flags)
}

fn print_help() {
    println!("Usage: rust-bin [file_name] [option]");
    println!();
    println!("Options:");
    println!("  -r, --release    \t\tBuild in release mode");
    println!("  -o, --output-file\t\tCreate output folder");
    println!("  -p, --parallel   \t\tRun test cases in parallel using all available cores");
    println!("  -h, --help       \t\tPrint this help message");
    println!("Note: you can also aggregate options like `-rop`");
    println!();
    println!(
        "The name of the input files should contain `{}`.",
        INPUT_TAG
    );
    println!(
        "Otherwise the input files should be placed in a folder whose name contains `{}`.",
        INPUT_TAG
    );
}

fn get_input_files(dir: &PathBuf, os: Os) -> Result<Vec<PathBuf>> {
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
            let name = match os {
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

fn get_cmd<'a>(
    current_dir: &PathBuf,
    file_name: &'a str,
    release: bool,
    os: Os,
) -> Result<(Vec<&'a str>, String)> {
    let cmd_run = match os {
        Os::Windows => format!(
            "{}\\target\\{}\\{}.exe",
            current_dir.to_str().unwrap(),
            if release { "release" } else { "debug" },
            file_name
        ),
        Os::Unix => format!(
            "{}/target/{}/{}",
            current_dir.to_str().unwrap(),
            if release { "release" } else { "debug" },
            file_name
        ),
    };
    let mut cmd_args = vec!["build", "--bin", file_name];
    if release {
        cmd_args.push("--release");
    }
    Ok((cmd_args, cmd_run))
}

fn create_empty_folder(current_dir: &PathBuf, os: Os) -> Result<()> {
    for entry in current_dir.read_dir()? {
        let entry = entry.unwrap().path();
        let name = match os {
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

fn run_test_cases(inputs: Vec<PathBuf>, binary: &str, flag: &[bool], os: Os) -> Result<()> {
    if flag[2] {
        let pool = ThreadPool::default();
        if flag[1] {
            for (i, input_file) in inputs.into_iter().enumerate() {
                let binary = binary.to_string();
                pool.execute(move || {
                    run(&binary, &input_file, Some(i), os).unwrap();
                });
            }
        } else {
            for input_file in inputs.into_iter() {
                let binary = binary.to_string();
                pool.execute(move || {
                    run(&binary, &input_file, None, os).unwrap();
                });
            }
        }
    } else {
        if flag[1] {
            for (i, input_file) in inputs.into_iter().enumerate() {
                run(&binary, &input_file, Some(i), os).unwrap();
            }
        } else {
            for input_file in inputs.into_iter() {
                run(&binary, &input_file, None, os).unwrap();
            }
        }
    }
    Ok(())
}

fn run(binary: &str, input_file: &PathBuf, file_number: Option<usize>, os: Os) -> Result<()> {
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
                "{}: {}ms | output.{}.txt",
                match os {
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
                match os {
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
                match os {
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

use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    fn default() -> ThreadPool {
        let size = thread::available_parallelism().unwrap().get();
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }
        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    _id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();
            match message {
                Ok(job) => job(),
                Err(_) => break,
            }
        });
        Worker {
            _id: id,
            thread: Some(thread),
        }
    }
}
