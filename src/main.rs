mod app;
mod cli;
mod config;
mod crop;
mod dither;
mod pipeline;
mod save;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let result = if args.first().map(String::as_str) == Some("process") {
        cli::run(&args[1..])
    } else {
        app::run().map_err(|e| e.to_string())
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
