mod cli;

fn main() {
    if let Err(error) = cli::run_from_args() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
