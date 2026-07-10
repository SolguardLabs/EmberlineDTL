fn main() {
    if let Err(error) = emberline_dtl::runtime::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
