//! Gugalanna - A web browser built from scratch
//!
//! Usage: gugalanna <url>

use std::env;
use std::process::ExitCode;

use url::Url;

use gugalanna_css::Stylesheet;
use gugalanna_dom::{DomTree, Queryable};
use gugalanna_html::HtmlParser;
use gugalanna_layout::{build_layout_tree, layout_block, ContainingBlock};
use gugalanna_net::HttpClient;
use gugalanna_render::{build_display_list, RenderBackend, RenderColor, SdlBackend};
use gugalanna_style::{Cascade, StyleTree};

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
            if let Err(e) = fetch_and_render(&args[2]).await {
                eprintln!("Error: {}", e);
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        url_str => {
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
    -h, --help       Print this help message
    -V, --version    Print version information
    --demo           Run a rendering demo (Hello World)
    --render <URL>   Render a URL in a window

EXAMPLES:
    {} https://example.com
    {} --demo
    {} --render https://example.com

"#,
        VERSION, program, program, program, program
    );
}

/// Run a simple "Hello World" rendering demo
fn run_demo() -> Result<(), Box<dyn std::error::Error>> {
    let html = r#"
        <html>
        <body>
            <h1>Hello World!</h1>
            <p>Welcome to Gugalanna, a browser built from scratch.</p>
        </body>
        </html>
    "#;

    let css = r#"
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

    render_html(html, css, "Gugalanna Demo", 800, 600)
}

/// Fetch a URL and render it
async fn fetch_and_render(url_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    let url = if url_str.contains("://") {
        Url::parse(url_str)?
    } else {
        Url::parse(&format!("https://{}", url_str))?
    };

    println!("Fetching: {}", url);

    let client = HttpClient::new()?;
    let response = client.get(&url).await?;

    if !response.is_success() {
        return Err(format!("HTTP error: {}", response.status).into());
    }

    let html = response.text_lossy();
    println!("Received {} bytes", html.len());

    // Use minimal styling for external pages
    let css = r#"
        body { background-color: white; color: black; font-size: 16px; }
        h1, h2, h3, h4, h5, h6, p, div { display: block; }
        h1 { font-size: 32px; margin-top: 20px; margin-bottom: 10px; }
        h2 { font-size: 24px; margin-top: 18px; margin-bottom: 8px; }
        h3 { font-size: 18px; margin-top: 16px; margin-bottom: 6px; }
        p { margin-top: 10px; margin-bottom: 10px; }
    "#;

    let title = format!("Gugalanna - {}", url.host_str().unwrap_or("Page"));
    render_html(&html, css, &title, 1024, 768)
}

/// Render HTML content in a window
fn render_html(
    html: &str,
    css: &str,
    title: &str,
    width: u32,
    height: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse HTML
    let dom = HtmlParser::new().parse(html)?;

    // Parse CSS and build cascade
    let mut cascade = Cascade::new();
    if let Ok(stylesheet) = Stylesheet::parse(css) {
        cascade.add_author_stylesheet(stylesheet);
    }

    // Build style tree
    let style_tree = StyleTree::build(&dom, &cascade);

    // Find the body element
    let body_ids = dom.get_elements_by_tag_name("body");
    let root_id = if !body_ids.is_empty() {
        body_ids[0]
    } else {
        dom.document_id()
    };

    // Build layout tree
    let mut layout_tree = match build_layout_tree(&dom, &style_tree, root_id) {
        Some(tree) => tree,
        None => return Err("Failed to build layout tree".into()),
    };

    // Perform layout
    layout_block(&mut layout_tree, ContainingBlock::new(width as f32, height as f32));

    // Build display list
    let display_list = build_display_list(&layout_tree);

    println!("Rendering {} paint commands...", display_list.commands.len());

    // Create SDL window
    let mut backend = SdlBackend::new(title, width, height)?;

    // Main loop using raw SDL to avoid panic on unknown events
    'running: loop {
        // Poll events using raw SDL API to handle unknown event types gracefully
        unsafe {
            let mut raw_event: sdl2::sys::SDL_Event = std::mem::zeroed();
            while sdl2::sys::SDL_PollEvent(&mut raw_event) != 0 {
                // Check event type
                let event_type = raw_event.type_;

                // SDL_QUIT = 0x100 (256)
                if event_type == 0x100 {
                    break 'running;
                }

                // SDL_KEYDOWN = 0x300 (768)
                if event_type == 0x300 {
                    let key_event = raw_event.key;
                    let scancode = key_event.keysym.scancode as i32;
                    // SDL_SCANCODE_ESCAPE = 41, SDL_SCANCODE_Q = 20
                    if scancode == 41 || scancode == 20 {
                        break 'running;
                    }
                }

                // SDL_WINDOWEVENT = 0x200 (512)
                if event_type == 0x200 {
                    let window_event = raw_event.window;
                    // SDL_WINDOWEVENT_CLOSE = 14
                    if window_event.event == 14 {
                        break 'running;
                    }
                }
            }
        }

        // Render
        backend.clear(RenderColor::white());
        backend.render(&display_list);
        backend.present();

        // Small sleep to avoid busy-waiting
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    Ok(())
}

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
