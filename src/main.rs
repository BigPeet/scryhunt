use anyhow::Result;
use clap::{Args, Parser};
use std::{borrow::Borrow, path::PathBuf};

use serde::Deserialize;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to the query config file
    queries: PathBuf,

    #[command(flatten)]
    args: QueryArgs,
}

#[derive(Args)]
struct QueryArgs {
    /// Comma-separated list of sets to filter by
    #[arg(short = 'e', long, value_delimiter = ',')]
    sets: Option<Vec<String>>,

    /// Show reprints
    #[arg(short = 'r', long, default_value_t = false)]
    show_reprints: bool,

    /// Show non-paper cards
    #[arg(long, default_value_t = false)]
    show_non_paper: bool,

    /// Instead generate an URL which only shows cards excluded by the queries
    #[arg(short = 'n', long, default_value_t = false)]
    negate: bool,
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

struct UrlGenerator {
    default_args: String,
    negate: bool,
}

impl From<QueryArgs> for UrlGenerator {
    fn from(value: QueryArgs) -> Self {
        let set_string = value
            .sets
            .as_ref()
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
            if !value.show_reprints {
                "-is:reprint"
            } else {
                ""
            },
            if !value.show_non_paper {
                "game:paper"
            } else {
                ""
            },
            set_string,
        )
        .trim()
        .into();
        Self {
            default_args,
            negate: value.negate,
        }
    }
}

fn negate_query(mut query: String) -> String {
    query.insert_str(0, "-(");
    query.push(')');
    query
}

fn to_url(default_args: &str, query_text: &str) -> String {
    format!(
        "https://scryfall.com/search?q={}{}{}",
        urlencoding::encode(default_args),
        if !default_args.is_empty() { "%20" } else { "" },
        urlencoding::encode(query_text)
    )
}

fn color_identity<T: AsRef<str>>(identity: T) -> String {
    format!("ci:{}", identity.as_ref())
}

fn format_legality<T: AsRef<str>>(format: T) -> String {
    format!("f:{}", format.as_ref())
}

fn concat_needs<T: Borrow<str>>(needs: &[T]) -> String {
    format!("(({}))", needs.join(") or ("))
}

fn build_query_text(query: &Query) -> String {
    let query_args = [
        query.color_identity.as_deref().map(color_identity),
        query.format.as_deref().map(format_legality),
        query.needs.as_deref().map(concat_needs),
    ];

    query_args
        .into_iter()
        .flatten()
        .collect::<Vec<String>>()
        .join(" ")
}

impl UrlGenerator {
    /// Generate a single URL for a given query, including default arguments and optional negation.
    pub fn single_url(&self, query: &Query) -> String {
        let query_text = build_query_text(query);

        // Negate user provided query, if selected
        let query_text = if self.negate {
            negate_query(query_text)
        } else {
            query_text
        };

        to_url(&self.default_args, &query_text)
    }

    /// Generate a vector of URLs for a list of queries, including default arguments.
    /// If negation is selected, a single URL will be generated that negates all queries.
    pub fn urls(&self, queries: &[Query]) -> Vec<String> {
        if self.negate {
            let query_text = queries
                .iter()
                .map(build_query_text)
                .collect::<Vec<String>>()
                .join(") or (");
            vec![to_url(&self.default_args, &format!("-(({}))", query_text))]
        } else {
            queries.iter().map(|q| self.single_url(q)).collect()
        }
    }
}

// Idea scratch pad (in code...for now):
// TODO: some clean up and restructuring
// TODO: improve documentation and README
// TODO: improve error handling/reporting
// TODO: add tests (unit and maybe integration)
// TODO: Use API to create JSON output (see https://scryfall.com/docs/api/lists)
//       -> Might want to distinguish between Web URLs (to be viewed in browser) and API URLs (to request JSON data).
//          E.g., `single_web_url` and `single_api_url`, or single_url<T>?

fn main() -> Result<()> {
    // Read command line arguments
    let cli = Cli::parse();

    // Read and parse decks.toml to some internal structure
    let config: QueryConfig = toml::from_str(&std::fs::read_to_string(&cli.queries)?)?;

    let generator = UrlGenerator::from(cli.args);

    // for each deck, create a query URL
    for query in &config.queries {
        println!("URL for {}: {}", query.name, generator.single_url(query));
    }

    for url in generator.urls(&config.queries) {
        println!("URL: {}", url);
    }

    Ok(())
}
