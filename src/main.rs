use drivel::produce;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mode = args.get(1).expect("No mode provided. Usage: drivel [mode] <array_length>, where mode is in (describe, produce)");
    let array_size: usize = args.get(2).and_then(|size| size.parse().ok()).unwrap_or(1);
    let stdin = std::io::stdin();

    let parsed: serde_json::Value =
        serde_json::from_reader(stdin).expect("Unable to parse input JSON");
    let schema = drivel::infer_schema(&parsed);

    match mode.as_str() {
        "produce" => {
            let result = produce(&schema, array_size);
            let stdout = std::io::stdout();
            serde_json::to_writer_pretty(stdout, &result).unwrap();
        },
        "describe" => {
            println!("{}", schema.to_string_pretty(0));
        }
        _ => println!("Invalid mode provided. Usage: drive [mode] <array_length>, where mode is in (describe, produce)")
    }
}
