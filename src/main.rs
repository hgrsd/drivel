use drivel::SchemaState;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mode = args.get(1).expect(
        "No mode provided. Usage: drivel [mode] <n repeat>, where mode is in (describe, produce)",
    );
    let repeat_n: usize = args.get(2).and_then(|size| size.parse().ok()).unwrap_or(1);
    let input = std::io::read_to_string(std::io::stdin()).expect("Unable to read from stdin");

    let schema = if let Ok(json) = serde_json::from_str(&input) {
        drivel::infer_schema(&json)
    } else {
        // unable to parse input as JSON; try JSON lines format as fallback
        let values = input.lines().map(|line| serde_json::from_str(line).expect("Unable to parse input; format is neither JSON nor JSON lines"));
        drivel::infer_schema_from_iter(values)
    };
    

    match mode.as_str() {
        "produce" => {
            let schema = match schema {
                SchemaState::Array { .. } => schema,
                _ => {
                    // if the user wants to repeat the data more than once and we aren't dealing
                    // with an array at the root, then we wrap the state in an array before we 
                    // produce our values
                    if repeat_n > 1 {
                        SchemaState::Array { min_length: 1, max_length: 1, schema: Box::new(schema) }
                    } else {
                        schema
                    }
                }
            };
                
            let result = drivel::produce(&schema, repeat_n);
            let stdout = std::io::stdout();
            serde_json::to_writer_pretty(stdout, &result).unwrap();
        },
        "describe" => {
            println!("{}", schema.to_string_pretty());
        }
        _ => println!("Invalid mode provided. Usage: drivel [mode] <array_length>, where mode is in (describe, produce)")
    }
}
