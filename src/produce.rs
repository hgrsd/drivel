use chrono::{DateTime, NaiveDate, SubsecRound, Utc};
use fake::{Fake, Faker};
use rand::{random, thread_rng, Rng};
use serde_json::Number;

use crate::{NumberType, SchemaState, StringType};

fn produce_inner(schema: &SchemaState, repeat_n: usize, current_depth: usize) -> serde_json::Value {
    match schema {
        SchemaState::Initial | SchemaState::Null => serde_json::Value::Null,
        SchemaState::Nullable(inner) => {
            let should_return_null: bool = random();
            if should_return_null {
                serde_json::Value::Null
            } else {
                produce_inner(inner, repeat_n, current_depth + 1)
            }
        }
        SchemaState::String(string_type) => {
            let value = match string_type {
                StringType::IsoDate => {
                    let date: NaiveDate = Faker.fake();
                    date.to_string()
                }
                StringType::DateTimeISO8601 => {
                    let date_time: DateTime<Utc> = Faker.fake();
                    let date_time = date_time.round_subsecs(3);
                    date_time.to_rfc3339()
                }
                StringType::DateTimeRFC2822 => {
                    let date_time: DateTime<Utc> = Faker.fake();
                    let date_time = date_time.round_subsecs(3);
                    date_time.to_rfc2822()
                }
                StringType::UUID => {
                    let uuid = uuid::Uuid::new_v4();
                    uuid.to_string()
                }
                StringType::Unknown {
                    char_distribution: charset,
                    min_length,
                    max_length,
                } => {
                    let min = min_length.unwrap_or(0);
                    let max = max_length.unwrap_or(32);
                    let take_n = if min != max {
                        thread_rng().gen_range(min..=max)
                    } else {
                        min
                    };

                    if charset.is_empty() {
                        take_n.fake()
                    } else {
                        let mut s = String::with_capacity(take_n);
                        for _ in 0..take_n {
                            let idx = thread_rng().gen_range(0..charset.len());
                            s.push(charset[idx]);
                        }
                        s
                    }
                }
            };
            serde_json::Value::String(value)
        }
        SchemaState::Number(number_type) => match *number_type {
            NumberType::Integer { min, max } => {
                let number = if min != max {
                    thread_rng().gen_range(min..=max)
                } else {
                    min
                };
                serde_json::Value::Number(Number::from(number))
            }
            NumberType::Float { min, max } => {
                let number = if min != max {
                    thread_rng().gen_range(min..=max)
                } else {
                    min
                };
                serde_json::Value::Number(Number::from_f64(number).unwrap())
            }
        },
        SchemaState::Boolean => serde_json::Value::Bool(random()),
        SchemaState::Array {
            min_length,
            max_length,
            schema,
        } => {
            if schema.as_ref() == &SchemaState::Indefinite
                || schema.as_ref() == &SchemaState::Initial
            {
                return serde_json::Value::Array(vec![]);
            }

            let n_elements = if current_depth == 0 {
                // if we are dealing with an array at the root, we produce the requested `n` elements
                repeat_n
            } else {
                if min_length != max_length {
                    thread_rng().gen_range(*min_length..=*max_length)
                } else {
                    *min_length
                }
            };

            let data: Vec<_> = (0..n_elements)
                .map(|_| produce_inner(schema, repeat_n, current_depth + 1))
                .collect();
            serde_json::Value::Array(data)
        }
        SchemaState::Object { required, optional } => {
            let mut map = serde_json::Map::new();
            for (k, v) in required.iter() {
                let value = produce_inner(v, repeat_n, current_depth + 1);
                map.insert(k.clone(), value);
            }
            for (k, v) in optional.iter() {
                let should_include: bool = random();
                if should_include {
                    let value = produce_inner(v, repeat_n, current_depth + 1);
                    map.insert(k.clone(), value);
                }
            }
            serde_json::Value::Object(map)
        }
        SchemaState::Indefinite => serde_json::Value::Null,
    }
}

/// Produces a JSON value based on the given schema.
///
/// This function generates a JSON value based on the provided schema state.
///
/// # Arguments
///
/// * `schema` - The schema state to produce JSON values for.
/// * `repeat_n` - The number of times to repeat generation (used for arrays at the JSON root).
///
/// # Returns
///
/// # Examples
///
/// ```
/// use drivel::{SchemaState, NumberType, produce};
///
/// // The inferred schema consists of an array with length = 1
/// let schema = SchemaState::Array {
///     min_length: 1,
///     max_length: 1,
///     schema: Box::new(SchemaState::Number(NumberType::Integer { min: 0, max: 100 })),
/// };
///
/// // Generate three values based on the schema
/// let json_data = produce(&schema, 3);
///
/// // Do something with the generated JSON data
/// println!("{}", json_data);
/// // Output: [23, 58, 12]
/// ```
pub fn produce(schema: &SchemaState, repeat_n: usize) -> serde_json::Value {
    produce_inner(schema, repeat_n, 0)
}
