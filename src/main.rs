fn main() {
    let stdin = std::io::stdin();
    let parsed: serde_json::Value =
        serde_json::from_reader(stdin).expect("Unable to parse input JSON");
    let schema = drivel::infer_schema(&parsed);

    println!("{:?}", schema);
}
