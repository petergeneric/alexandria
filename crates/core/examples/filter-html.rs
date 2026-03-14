use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let (input, output, domain) = match args.len() {
        3 => (args[1].as_str(), args[2].as_str(), "news.ycombinator.com"),
        4 => (args[1].as_str(), args[2].as_str(), args[3].as_str()),
        _ => {
            eprintln!("Usage: filter-html <input.html> <output.html> [domain]");
            eprintln!("  domain defaults to news.ycombinator.com");
            std::process::exit(1);
        }
    };

    let html = fs::read_to_string(input).expect("failed to read input file");
    let filtered = alexandria_core::filter::filter_html(&html, domain);
    fs::write(output, &filtered).expect("failed to write output file");

    eprintln!(
        "Input:  {} bytes\nOutput: {} bytes\nReduction: {:.0}%",
        html.len(),
        filtered.len(),
        (1.0 - filtered.len() as f64 / html.len() as f64) * 100.0
    );
}
