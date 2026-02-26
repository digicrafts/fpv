fn main() {
    if let Err(err) = fpv::app::run::run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
