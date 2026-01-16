//! Gugalanna - A web browser built from scratch
//!
//! Usage: gugalanna <url>

use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use url::Url;

use gugalanna_dom::{DomTree, Queryable};
use gugalanna_html::HtmlParser;
use gugalanna_net::HttpClient;
use gugalanna_shell::{Browser, BrowserConfig};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        return ExitCode::FAILURE;
    }

    let command = args[1].as_str();

    match command {
        "--help" | "-h" => {
            print_usage(&args[0]);
            ExitCode::SUCCESS
        }
        "--version" | "-V" => {
            println!("Gugalanna {}", VERSION);
            ExitCode::SUCCESS
        }
        "--demo" => {
            // Render a simple "Hello World" demo
            if let Err(e) = run_demo() {
                eprintln!("Error: {}", e);
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        "--render" => {
            // Render a URL in a window
            if args.len() < 3 {
                eprintln!("Usage: {} --render <URL>", args[0]);
                return ExitCode::FAILURE;
            }
            if let Err(e) = run_browser(&args[2]) {
                eprintln!("Error: {}", e);
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        "--file" => {
            // Render a local HTML file
            if args.len() < 3 {
                eprintln!("Usage: {} --file <PATH>", args[0]);
                return ExitCode::FAILURE;
            }
            if let Err(e) = run_file(&args[2]) {
                eprintln!("Error: {}", e);
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        url_str => {
            // Text-only mode: fetch and display DOM tree
            if let Err(e) = fetch_and_display(url_str).await {
                eprintln!("Error: {}", e);
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
    }
}

fn print_usage(program: &str) {
    println!(
        r#"Gugalanna {} - A web browser built from scratch

USAGE:
    {} [OPTIONS] <URL>

OPTIONS:
    -h, --help        Print this help message
    -V, --version     Print version information
    --demo            Run a rendering demo (Hello World)
    --render <URL>    Render a URL in a window
    --file <PATH>     Render a local HTML file

EXAMPLES:
    {} https://example.com
    {} --demo
    {} --render https://example.com
    {} --file test-pages/basic.html

"#,
        VERSION, program, program, program, program, program
    );
}

/// Run a simple "Hello World" rendering demo
fn run_demo() -> Result<(), String> {
    let config = BrowserConfig {
        title: "Gugalanna Demo".to_string(),
        width: 800,
        height: 600,
    };

    let mut browser = Browser::new(config)?;

    // Navigate to a demo HTML page using data URL
    browser.load_html(DEMO_HTML, DEMO_CSS)?;

    browser.run()
}

/// Run browser with a URL
fn run_browser(url_str: &str) -> Result<(), String> {
    let config = BrowserConfig {
        title: "Gugalanna".to_string(),
        width: 1024,
        height: 768,
    };

    let mut browser = Browser::new(config)?;

    // Navigate to the URL
    browser.navigate(url_str)?;

    browser.run()
}

/// Run browser with a local HTML file
fn run_file(path_str: &str) -> Result<(), String> {
    let path = Path::new(path_str);

    // Read HTML file
    let html = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path_str, e))?;

    // Try to find and read associated CSS file
    let css = if let Some(parent) = path.parent() {
        // Look for style.css in the same directory
        let css_path = parent.join("style.css");
        if css_path.exists() {
            fs::read_to_string(&css_path).unwrap_or_default()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let title = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("Gugalanna");

    let config = BrowserConfig {
        title: format!("Gugalanna - {}", title),
        width: 1024,
        height: 768,
    };

    let mut browser = Browser::new(config)?;

    // Load HTML content from file (with proper file:// URL)
    browser.load_html_from_file(path, &html, &css)?;

    browser.run()
}

/// Demo HTML content
const DEMO_HTML: &str = r#"
<html>
<body>
    <h1>Hello World!</h1>
    <p>Welcome to Gugalanna, a browser built from scratch.</p>
</body>
</html>
"#;

/// Demo CSS styling
const DEMO_CSS: &str = r#"
body {
    background-color: white;
    color: black;
    font-size: 16px;
}
h1 {
    display: block;
    font-size: 32px;
    color: #333333;
    margin-top: 20px;
    margin-bottom: 10px;
    margin-left: 20px;
}
p {
    display: block;
    margin-left: 20px;
    margin-top: 10px;
}
"#;

/// Text-only mode: Fetch a URL and display DOM tree
async fn fetch_and_display(url_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Parse URL
    let url = if url_str.contains("://") {
        Url::parse(url_str)?
    } else {
        Url::parse(&format!("https://{}", url_str))?
    };

    println!("Fetching: {}\n", url);

    // Fetch the page
    let client = HttpClient::new()?;
    let response = client.get(&url).await?;

    if !response.is_success() {
        return Err(format!("HTTP error: {}", response.status).into());
    }

    // Get HTML content
    let html = response.text_lossy();

    println!("Received {} bytes\n", html.len());

    // Parse HTML to DOM
    let parser = HtmlParser::new();
    let tree = parser.parse(&html)?;

    // Print DOM tree
    println!("=== DOM Tree ===\n");
    println!("{}", tree.pretty_print());

    // Print some stats
    println!("\n=== Stats ===");
    println!("Total nodes: {}", tree.len());

    // Count element types
    let elements = count_elements(&tree);
    println!("Elements: {:?}", elements);

    Ok(())
}

fn count_elements(tree: &DomTree) -> Vec<(String, usize)> {
    use std::collections::HashMap;

    let mut counts: HashMap<String, usize> = HashMap::new();

    // Get all elements
    for tag in &[
        "html", "head", "body", "div", "p", "span", "a", "script", "style", "link", "meta", "img",
        "input", "button", "form", "ul", "li", "table", "tr", "td", "h1", "h2", "h3",
    ] {
        let elements = tree.get_elements_by_tag_name(tag);
        if !elements.is_empty() {
            counts.insert(tag.to_string(), elements.len());
        }
    }

    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted
}
