#[derive(PartialEq, Eq, Debug)]
pub enum StringType {
    Unknown,
    IsoDate,
    IsoDateTime,
    UUID,
}

#[derive(PartialEq, Eq, Debug)]
pub enum NumberType {
    Integer,
    Float,
}

#[derive(PartialEq, Eq, Debug)]
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
