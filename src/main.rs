use drivel::produce;

fn main() {
    let stdin = std::io::stdin();

    let parsed: serde_json::Value =
        serde_json::from_reader(stdin).expect("Unable to parse input JSON");
    let schema = drivel::infer_schema(&parsed);

    let args: Vec<_> = std::env::args().collect();
    let array_size: usize = args.get(1).and_then(|size| size.parse().ok()).unwrap_or(1);
    let result = produce(&schema, array_size);

    let stdout = std::io::stdout();
    serde_json::to_writer_pretty(stdout, &result).unwrap();
}
