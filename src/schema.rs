#[derive(PartialEq, Debug)]
pub enum StringType {
    Unknown {
        min_length: Option<usize>,
        max_length: Option<usize>,
    },
    IsoDate,
    IsoDateTime,
    UUID,
}

#[derive(PartialEq, Debug)]
pub enum NumberType {
    Integer { min: i64, max: i64 },
    Float { min: f64, max: f64 },
}

#[derive(PartialEq, Debug)]
pub enum SchemaState {
    Initial,
    Null,
    Nullable(Box<SchemaState>),
    String(StringType),
    Number(NumberType),
    Boolean,
    Array(Box<SchemaState>),
    Object {
        required: std::collections::HashMap<String, SchemaState>,
        optional: std::collections::HashMap<String, SchemaState>,
    },
    Indefinite,
}
