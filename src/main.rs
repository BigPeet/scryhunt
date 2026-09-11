use clap::Parser;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to the query config file
    queries: PathBuf,

    /// Comma-separated list of sets to filter by
    #[arg(short = 'e', long, value_delimiter = ',')]
    sets: Option<Vec<String>>,

    /// Show reprints
    #[arg(short = 'r', long, default_value_t = false)]
    show_reprints: bool,

    /// Show non-paper cards
    #[arg(long, default_value_t = false)]
    show_non_paper: bool,
}

#[derive(Deserialize)]
struct QueryConfig {
    queries: Vec<Query>,
}

#[derive(Deserialize)]
struct Query {
    name: String,
    needs: Option<Vec<String>>,
    color_identity: Option<String>,
    format: Option<String>,
}

struct Generator {
    default_args: String,
}

impl Generator {
    fn new(ignore_reprints: bool, ignore_non_paper: bool, sets: Option<&Vec<String>>) -> Self {
        let set_string = sets
            // remove empty elements from the vector and trim whitespace
            .map(|s| {
                s.iter()
                    .filter_map(|set| {
                        if set.trim().is_empty() {
                            None
                        } else {
                            Some(set.trim())
                        }
                    })
                    .collect::<Vec<&str>>()
            })
            // if the vector is empty after filtering, return None
            .filter(|s| !s.is_empty())
            // join the remaining elements or return empty string
            .map(|s| format!("(e:{})", s.join(" or e:")))
            .unwrap_or("".to_string());
        let default_args = format!(
            "{} {} {}",
            if ignore_reprints { "-is:reprint" } else { "" },
            if ignore_non_paper { "game:paper" } else { "" },
            set_string,
        )
        .trim()
        .into();
        Self { default_args }
    }

    fn generate_url(&self, query: &Query) -> String {
        let query_args = vec![
            Some(self.default_args.clone()),
            query.color_identity.as_ref().map(|ci| format!("ci:{}", ci)),
            query.format.as_ref().map(|f| format!("f:{}", f)),
            query
                .needs
                .as_ref()
                .map(|n| format!("(({}))", n.join(") or ("))),
        ];
        let query = query_args
            .into_iter()
            .flatten()
            .collect::<Vec<String>>()
            .join(" ");

        // Encode the query for URL
        format!(
            "https://scryfall.com/search?q={}",
            urlencoding::encode(&query)
        )
    }
}

fn main() {
    // Read command line arguments
    let cli = Cli::parse();

    if !cli.queries.exists() {
        eprintln!("Error: file not found at {}", cli.queries.display());
        std::process::exit(1);
    }
    // Read and parse decks.toml to some internal structure
    let config: QueryConfig =
        toml::from_str(&std::fs::read_to_string(&cli.queries).expect("Failed to read file"))
            .expect("Failed to parse file");

    let generator = Generator::new(!cli.show_reprints, !cli.show_non_paper, cli.sets.as_ref());

    // for each deck, create a query URL
    for query in &config.queries {
        println!("URL for {}: {}", query.name, generator.generate_url(query));
    }
}
