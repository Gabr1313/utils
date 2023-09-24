use std::io;

#[derive(Default)]
struct Scanner {
    buffer: Vec<String>,
    input: String,
}

impl Scanner {
    fn tok<T: std::str::FromStr>(&mut self) -> T {
        loop {
            if let Some(token) = self.buffer.pop() {
                return token.parse().ok().expect("Failed parse");
            }
            self.input.clear();
            io::stdin().read_line(&mut self.input).expect("Failed read");
            self.buffer = self.input.split_whitespace().rev().map(String::from).collect();
        }
    }
}

fn main() {
    let mut scan = Scanner::default();
    solve(&mut scan);
}

fn solve(scan: &mut Scanner) {
    let n: usize = scan.tok();
    println!("{}", n);
}
