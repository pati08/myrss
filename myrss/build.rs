use toml::Value;

fn main() {
    #[cfg(not(feature = "shuttle"))]
    {
        println!("cargo::rerun-if-changed=Secrets.toml");
        let Ok(secrets_file) = std::fs::read_to_string("./Secrets.toml") else {
            return;
        };
        let secrets: toml::Table = toml::from_str(&secrets_file).unwrap();
        for secret in secrets.iter().filter_map(|(key, val)| {
            if let Value::String(string) = val {
                Some((key.to_owned(), string.to_owned()))
            } else {
                println!("cargo::warning=Invalid Secrets.toml assignment: `{key}` should be of type `String` but is of type `{}`", toml_value_type(val));
                None
            }
        }) {
            println!("cargo::rustc-env={}={}", secret.0, secret.1);
        }
    }
}

fn toml_value_type(value: &Value) -> &'static str {
    match value {
        Value::Float(_) => "Float",
        Value::Array(_) => "Array",
        Value::Table(_) => "Table",
        Value::String(_) => "String",
        Value::Integer(_) => "Integer",
        Value::Boolean(_) => "Boolean",
        Value::Datetime(_) => "Datetime",
    }
}
