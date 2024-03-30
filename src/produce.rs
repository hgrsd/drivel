use chrono::{DateTime, NaiveDate, Utc};
use fake::{Fake, Faker};
use rand::random;
use serde_json::Number;

use crate::{NumberType, SchemaState, StringType};

pub fn produce(schema: &SchemaState, array_size: usize) -> serde_json::Value {
    match schema {
        SchemaState::Initial | SchemaState::Null => serde_json::Value::Null,
        SchemaState::Nullable(inner) => {
            let should_return_null: bool = random();
            if should_return_null {
                serde_json::Value::Null
            } else {
                produce(inner, array_size)
            }
        }
        SchemaState::String(string_type) => {
            let value = match *string_type {
                StringType::IsoDate => {
                    let date: NaiveDate = Faker.fake();
                    date.to_string()
                }
                StringType::IsoDateTime => {
                    let date_time: DateTime<Utc> = Faker.fake();
                    date_time.to_string()
                }
                StringType::UUID => {
                    let uuid = uuid::Uuid::new_v4();
                    uuid.to_string()
                }
                StringType::Unknown => Faker.fake(),
            };
            serde_json::Value::String(value)
        }
        SchemaState::Number(number_type) => match *number_type {
            NumberType::Integer => serde_json::Value::Number(Number::from(random::<i64>())),
            NumberType::Float => serde_json::Value::Number(
                Number::from_f64(random::<f64>() * random::<i32>() as f64).unwrap(),
            ),
        },
        SchemaState::Boolean => serde_json::Value::Bool(random()),
        SchemaState::Array(array_type) => {
            if array_type.as_ref() == &SchemaState::Indefinite
                || array_type.as_ref() == &SchemaState::Initial
            {
                return serde_json::Value::Array(vec![]);
            }

            let data: Vec<_> = (0..array_size)
                .map(|_| produce(array_type, array_size))
                .collect();
            serde_json::Value::Array(data)
        }
        SchemaState::Object { required, optional } => {
            let mut map = serde_json::Map::new();
            for (k, v) in required.iter() {
                let value = produce(v, array_size);
                map.insert(k.clone(), value);
            }
            for (k, v) in optional.iter() {
                let should_include: bool = random();
                if should_include {
                    let value = produce(v, array_size);
                    map.insert(k.clone(), value);
                }
            }
            serde_json::Value::Object(map)
        }
        SchemaState::Indefinite => serde_json::Value::Null,
    }
}
