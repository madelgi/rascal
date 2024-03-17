use clap::{Parser, Subcommand};
use crate::executer::{execute, format_output};

mod executer;
mod parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Commands
}

/// Ideas for future subcommands
///   - transform: transform request spec from json to some programming language
///   - swarm: run a bunch of requests in parallel
///   - test: run integration tests against a server, support basic assertions, etc
#[derive(Subcommand, Debug, Clone)]
enum Commands {
    Exec {
        input_file: String,
        #[arg(short, long)]
        output_file: Option<String>,
        #[arg(short, long, value_parser = parse_key_val::<String, String>)]
        kwargs: Vec<(String, String)>,
        #[arg(short, long, action)]
        full_response: bool,
        #[arg(short, long, action)]
        pretty_print: bool,
    }
}

/// Parse a single key-value pair
fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn std::error::Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: std::error::Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}


fn main() {
    let args = Args::parse();
    match args.cmd {
        Commands::Exec { 
            input_file, 
            output_file,
            kwargs,
            full_response,
            pretty_print,
        } => {
            let output = execute(
                &input_file, 
                kwargs.into_iter().collect()
            ).and_then(|r| format_output(r, full_response, pretty_print, output_file));
            match output {
                Ok(s) => {
                    println!("{}", s);
                },
                Err(e) => eprintln!("🤦 {:?}", e),
            }
        }
    }
}
