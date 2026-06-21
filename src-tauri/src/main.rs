fn main() {
    if let Err(error) = judou_lib::run() {
        eprintln!("failed to run Judou: {error}");
        std::process::exit(1);
    }
}
