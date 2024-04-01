# drivel

`drivel` is a command-line tool written in Rust for inferring a schema from
an example JSON file, and generating synthetic data based on this inferred schema. It offers two main modes of operation: 'describe' and 'produce'.

## Features

- **Schema Inference**: drivel can analyze JSON input and infer its schema, including data types, array lengths, and object structures.
- **Data Generation**: Based on the inferred schema, drivel can generate synthetic data that adheres to the inferred structure.
- **Easy to integrate**: drivel reads JSON input from stdin and writes its output to stdout, allowing for easy integration into pipelines and workflows.

## Installation

To install drivel, ensure you have Rust installed and run:

```sh
cargo install drivel
```

## Usage

### Describe Mode

In 'describe' mode, drivel infers the schema from the input JSON and prints a human-readable description of the schema. This mode is useful for understanding the structure and data types present in JSON data.

```sh
cat input.json | drivel describe
```

### Produce Mode

In 'produce' mode, drivel infers the schema from the input JSON, generates synthetic data based on the inferred schema, and outputs the generated data in JSON format. This is useful for generating test data or sample datasets.

At present, drivel is moderately smart about schema inference. It makes some attempt at inferring semantic meaning for fields, particularly for String types,
but it is fairly limited at doing this. I would welcome any work to improve drivel's ability to understand the data it sees, to have more semantic understanding baked into the generated schema.

```sh
cat input.json | drivel produce
```

You can also specify the number of times to repeat the generated data:

```sh
cat input.json | drivel produce 5
```

## Examples

Consider a JSON file `input.json`:

```json
{
  "name": "John Doe",
  "age": 30,
  "is_student": false,
  "grades": [85, 90, 78],
  "address": {
    "city": "New York",
    "zip_code": "10001"
  }
}
```

Running drivel in 'describe' mode:

```sh
cat input.json | drivel describe
```

Output:

```
{
  "age": int (30),
  "address": {
    "city": string (8),
    "zip_code": string (5)
  },
  "is_student": boolean,
  "grades": [
    int (78-90)
  ] (3),
  "name": string (8)
}
```

Running drivel in 'produce' mode:

```sh
cat input.json | drivel produce 3
```

Output:

```json
[
  {
    "address": {
      "city": "o oowrYN",
      "zip_code": "01110"
    },
    "age": 30,
    "grades": [83, 88, 88],
    "is_student": true,
    "name": "nJ heo D"
  },
  {
    "address": {
      "city": "oro wwNN",
      "zip_code": "11000"
    },
    "age": 30,
    "grades": [83, 88, 89],
    "is_student": false,
    "name": "oeoooeeh"
  },
  {
    "address": {
      "city": "orww ok ",
      "zip_code": "00010"
    },
    "age": 30,
    "grades": [85, 90, 86],
    "is_student": false,
    "name": "ehnDoJDo"
  }
]
```

## Contributing

We welcome contributions from anyone interested in improving or extending drivel! Whether you have ideas for new features, bug fixes, or improvements to the documentation, feel free to open an issue or submit a pull request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
