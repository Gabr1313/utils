use template_cp::Scanner;
// use template_cp::{file_reader, file_writer};
use std::io;

fn main() {
    let mut scan = Scanner::new(io::stdin().lock());
    let mut out = io::BufWriter::new(io::stdout().lock()); 
    // let mut scan = file_reader("in.txt").unwrap();
    // let mut out = file_writer("out.txt").unwrap();
    solve(&mut scan, &mut out); 
}

fn solve<R: io::BufRead, W: io::Write>(scan: &mut Scanner<R>, out: &mut W) {
    let n: usize = scan.tok();
    writeln!(out, "{}", n).ok();
    let _v: Vec<i32> = (0..n).map(|_| scan.tok()).collect();
}
