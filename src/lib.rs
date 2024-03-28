#[derive(PartialEq, Eq, Debug)]
pub enum SchemaState {
    Unknown,
    Null,
    String,
    Number,
    Boolean,
    Array(Box<SchemaState>),
    Object {
        required: Vec<(String, SchemaState)>,
        optional: Vec<(String, SchemaState)>,
    },
}

pub fn merge_state(current_state: SchemaState, new_state: SchemaState) -> SchemaState {
    SchemaState::Unknown
}

#[cfg(test)]
mod test {
    mod merge {
        use crate::{merge_state, SchemaState};

        #[test]
        fn anything_merged_with_unknown_yields_unknown() {
            assert_eq!(
                merge_state(SchemaState::Unknown, SchemaState::Unknown),
                SchemaState::Unknown
            );
            assert_eq!(
                merge_state(SchemaState::Null, SchemaState::Unknown),
                SchemaState::Unknown
            );
            assert_eq!(
                merge_state(SchemaState::String, SchemaState::Unknown),
                SchemaState::Unknown
            );
            assert_eq!(
                merge_state(SchemaState::Number, SchemaState::Unknown),
                SchemaState::Unknown
            );
            assert_eq!(
                merge_state(SchemaState::Boolean, SchemaState::Unknown),
                SchemaState::Unknown
            );
            assert_eq!(
                merge_state(
                    SchemaState::Array(Box::new(SchemaState::Number)),
                    SchemaState::Unknown
                ),
                SchemaState::Unknown
            );
            assert_eq!(
                merge_state(
                    SchemaState::Object {
                        required: vec![("foo".to_owned(), SchemaState::Boolean)],
                        optional: Vec::new()
                    },
                    SchemaState::Unknown
                ),
                SchemaState::Unknown
            );
        }
    }
}
