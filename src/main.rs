use drivel::produce;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mode = args.get(1).expect(
        "No mode provided. Usage: drivel [mode] <n repeat>, where mode is in (describe, produce)",
    );
    let repeat_n: usize = args.get(2).and_then(|size| size.parse().ok()).unwrap_or(1);
    let stdin = std::io::stdin();

    let parsed: serde_json::Value =
        serde_json::from_reader(stdin).expect("Unable to parse input JSON");

    match mode.as_str() {
        "produce" => {
            let parsed = if !parsed.is_array() && repeat_n > 1 {
                // if the user wants to repeat the data more than once and we aren't dealing with an array at the root,
                // then we wrap the root value in an array first so that downstream we can just expand that array.
                serde_json::Value::Array(vec![parsed])
            } else {
                parsed
            };
            let schema = drivel::infer_schema(&parsed);
            let result = produce(&schema, repeat_n);
            let stdout = std::io::stdout();
            serde_json::to_writer_pretty(stdout, &result).unwrap();
        },
        "describe" => {
            let schema = drivel::infer_schema(&parsed);
            println!("{}", schema.to_string_pretty(0));
        }
        _ => println!("Invalid mode provided. Usage: drivel [mode] <array_length>, where mode is in (describe, produce)")
    }
}
