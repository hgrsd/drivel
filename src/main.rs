use clap::{Parser, Subcommand};
use drivel::SchemaState;
use jemallocator::Jemalloc;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[derive(Subcommand, Debug)]
enum Mode {
    /// Describe the inferred schema for the input data
    Describe,
    /// Produce synthetic data adhering to the inferred schema
    Produce {
        #[arg(short, long)]
        /// Produce `n` elements. Default = 1.
        n_repeat: Option<usize>,
    },
}

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    mode: Mode,
}

fn main() {
    let args = Args::parse();
    let input = match std::io::read_to_string(std::io::stdin()) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Unable to read from stdin. Error: {}", err.to_string());
            std::process::exit(1)
        }
    };

    let opts = drivel::InferenceOptions {
        enum_inference: None,
    };

    let schema = if let Ok(json) = serde_json::from_str(&input) {
        drivel::infer_schema(json, &opts)
    } else {
        // unable to parse input as JSON; try JSON lines format as fallback
        let values = input
            .lines()
            .map(|line| match serde_json::from_str(line) {
                Ok(v) => v,
                Err(err) => {
                    eprintln!(
                        "Error parsing input; are you sure it is valid JSON? Error: {}",
                        err
                    );
                    std::process::exit(1);
                }
            })
            .collect();
        drivel::infer_schema_from_iter(values, &opts)
    };

    match &args.mode {
        Mode::Produce { n_repeat } => {
            let n_repeat = n_repeat.unwrap_or(1);
            let schema = match schema {
                SchemaState::Array { .. } => schema,
                _ => {
                    // if the user wants to repeat the data more than once and we aren't dealing
                    // with an array at the root, then we wrap the state in an array before we
                    // produce our values
                    if n_repeat > 1 {
                        SchemaState::Array {
                            min_length: 1,
                            max_length: 1,
                            schema: Box::new(schema),
                        }
                    } else {
                        schema
                    }
                }
            };

            let result = drivel::produce(&schema, n_repeat);
            let stdout = std::io::stdout();
            serde_json::to_writer_pretty(stdout, &result).unwrap();
        }
        Mode::Describe => {
            println!("{}", schema.to_string_pretty());
        }
    }
}
