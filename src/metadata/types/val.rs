//! Types for modeling and representing item metadata.

use std::collections::BTreeMap;

use bigdecimal::BigDecimal;

use metadata::types::key::MetaKey;

#[derive(PartialEq, Eq, Debug, Clone, Hash, Deserialize)]
#[serde(untagged)]
pub enum MetaVal {
    Nil,
    Str(String),
    Seq(Vec<MetaVal>),
    Map(BTreeMap<MetaKey, MetaVal>),
    Int(i64),
    Bul(bool),
    Dec(BigDecimal),
}

impl MetaVal {
    pub fn get_key_path<'k>(&self, key_path: &[&'k MetaKey]) -> Option<&MetaVal> {
        let mut curr_val = self;

        for key in key_path {
            // See if the current meta value is indeed a mapping.
            match curr_val {
                MetaVal::Map(map) => {
                    // See if the current key in the key path is found in this mapping.
                    match map.get(key) {
                        None => {
                            // Unable to proceed on the key path, short circuit.
                            return None;
                        }
                        Some(val) => {
                            // The current key was found, set the new current value.
                            curr_val = val;
                        }
                    }
                },
                _ => {
                    // An attempt was made to get the key of a non-mapping, short circuit.
                    return None;
                },
            }
        }

        // The remaining current value is what is needed to return.
        Some(curr_val)
    }

    pub fn resolve_key_path<'k>(self, key_path: &[&'k MetaKey]) -> Option<MetaVal> {
        let mut curr_val = self;

        for key in key_path {
            // See if the current meta value is indeed a mapping.
            match curr_val {
                MetaVal::Map(mut map) => {
                    // See if the current key in the key path is found in this mapping.
                    match map.remove(key) {
                        None => {
                            // Unable to proceed on the key path, short circuit.
                            return None;
                        }
                        Some(val) => {
                            // The current key was found, set the new current value.
                            curr_val = val;
                        }
                    }
                },
                _ => {
                    // An attempt was made to get the key of a non-mapping, short circuit.
                    return None;
                },
            }
        }

        // The remaining current value is what is needed to return.
        Some(curr_val)
    }
}

impl From<String> for MetaVal {
    fn from(s: String) -> Self {
        Self::Str(s)
    }
}

impl From<&str> for MetaVal {
    fn from(s: &str) -> Self {
        Self::Str(s.to_string())
    }
}

impl From<i64> for MetaVal {
    fn from(i: i64) -> Self {
        Self::Int(i)
    }
}

impl From<bool> for MetaVal {
    fn from(b: bool) -> Self {
        Self::Bul(b)
    }
}

impl From<BigDecimal> for MetaVal {
    fn from(d: BigDecimal) -> Self {
        Self::Dec(d)
    }
}

#[cfg(test)]
mod tests {
    use super::MetaVal;

    use bigdecimal::BigDecimal;

    use metadata::types::key::MetaKey;

    #[test]
    fn test_deserialize() {
        let inputs_and_expected = vec![
            ("null", MetaVal::Nil),
            (r#""string""#, MetaVal::Str(String::from("string"))),
            ("27", MetaVal::Int(27)),
            ("-27", MetaVal::Int(-27)),
            ("3.1415", MetaVal::Dec(BigDecimal::new(31415.into(), 4))),
            ("-3.1415", MetaVal::Dec(-BigDecimal::new(31415.into(), 4))),
            ("true", MetaVal::Bul(true)),
            ("false", MetaVal::Bul(false)),
            (
                r#"[null, "string", 27, true]"#,
                MetaVal::Seq(vec![
                    MetaVal::Nil,
                    MetaVal::Str(String::from("string")),
                    MetaVal::Int(27),
                    MetaVal::Bul(true),
                ]),
            ),
            (
                r#"{"key_a": "string", "key_b": -27, "key_c": false}"#,
                MetaVal::Map(btreemap![
                    MetaKey::from("key_a") => MetaVal::Str(String::from("string")),
                    MetaKey::from("key_b") => MetaVal::Int(-27),
                    MetaKey::from("key_c") => MetaVal::Bul(false),
                ]),
            ),
        ];

        for (input, expected) in inputs_and_expected {
            let produced = serde_json::from_str::<MetaVal>(&input).unwrap();
            assert_eq!(expected, produced);
        }

        let inputs_and_expected = vec![
            ("null", MetaVal::Nil),
            ("~", MetaVal::Nil),
            (r#""string""#, MetaVal::Str(String::from("string"))),
            ("string", MetaVal::Str(String::from("string"))),
            ("27", MetaVal::Int(27)),
            ("-27", MetaVal::Int(-27)),
            ("3.1415", MetaVal::Dec(BigDecimal::new(31415.into(), 4))),
            ("-3.1415", MetaVal::Dec(-BigDecimal::new(31415.into(), 4))),
            ("true", MetaVal::Bul(true)),
            ("false", MetaVal::Bul(false)),
            (
                r#"[null, "string", 27, true]"#,
                MetaVal::Seq(vec![
                    MetaVal::Nil,
                    MetaVal::Str(String::from("string")),
                    MetaVal::Int(27),
                    MetaVal::Bul(true),
                ]),
            ),
            (
                "- null\n- string\n- 27\n- true",
                MetaVal::Seq(vec![
                    MetaVal::Nil,
                    MetaVal::Str(String::from("string")),
                    MetaVal::Int(27),
                    MetaVal::Bul(true),
                ]),
            ),
            (
                r#"{"key_a": "string", "key_b": -27, "key_c": false}"#,
                MetaVal::Map(btreemap![
                    MetaKey::from("key_a") => MetaVal::Str(String::from("string")),
                    MetaKey::from("key_b") => MetaVal::Int(-27),
                    MetaKey::from("key_c") => MetaVal::Bul(false),
                ]),
            ),
            (
                "key_a: string\nkey_b: -27\nkey_c: false",
                MetaVal::Map(btreemap![
                    MetaKey::from("key_a") => MetaVal::Str(String::from("string")),
                    MetaKey::from("key_b") => MetaVal::Int(-27),
                    MetaKey::from("key_c") => MetaVal::Bul(false),
                ]),
            ),
        ];

        for (input, expected) in inputs_and_expected {
            let produced = serde_yaml::from_str::<MetaVal>(&input).unwrap();
            assert_eq!(expected, produced);
        }
    }

    #[test]
    fn test_get_key_path() {
        let key_str_a = MetaKey::from("key_a");
        let key_str_b = MetaKey::from("key_b");
        let key_str_c = MetaKey::from("key_c");
        let key_str_x = MetaKey::from("key_x");

        let val_nil = MetaVal::Nil;
        let val_str_a = MetaVal::Str(String::from("val_a"));
        let val_str_b = MetaVal::Str(String::from("val_b"));
        let val_str_c = MetaVal::Str(String::from("val_c"));
        let val_seq_a = MetaVal::Seq(vec![
            val_str_a.clone(), val_str_a.clone(), val_str_a.clone(),
        ]);
        let val_seq_b = MetaVal::Seq(vec![
            val_str_b.clone(), val_str_b.clone(), val_str_b.clone(),
        ]);
        let val_seq_c = MetaVal::Seq(vec![
            val_str_c.clone(), val_str_c.clone(), val_str_c.clone(),
        ]);
        let val_map_a = MetaVal::Map(btreemap![
            key_str_a.clone() => val_str_a.clone(),
            key_str_b.clone() => val_str_b.clone(),
            key_str_c.clone() => val_str_c.clone(),
        ]);
        let val_map_b = MetaVal::Map(btreemap![
            key_str_a.clone() => val_seq_a.clone(),
            key_str_b.clone() => val_seq_b.clone(),
            key_str_c.clone() => val_seq_c.clone(),
        ]);
        let val_map_c = MetaVal::Map(btreemap![
            key_str_a.clone() => val_nil.clone(),
            key_str_b.clone() => val_nil.clone(),
            key_str_c.clone() => val_nil.clone(),
        ]);
        let val_map_d = MetaVal::Map(btreemap![
            key_str_a.clone() => val_map_a.clone(),
            key_str_b.clone() => val_map_b.clone(),
            key_str_c.clone() => val_map_c.clone(),
        ]);

        let inputs_and_expected = vec![

            // An empty key path always returns the original value.
            ((&val_nil, vec![]), Some(&val_nil)),
            ((&val_str_a, vec![]), Some(&val_str_a)),
            ((&val_seq_a, vec![]), Some(&val_seq_a)),
            ((&val_map_a, vec![]), Some(&val_map_a)),

            // A non-empty key path returns no value on non-maps.
            ((&val_nil, vec![&key_str_a]), None),
            ((&val_str_a, vec![&key_str_a]), None),
            ((&val_seq_a, vec![&key_str_a]), None),

            // If the key is not found in a mapping, nothing is returned.
            ((&val_map_a, vec![&key_str_x]), None),
            ((&val_map_d, vec![&key_str_a, &key_str_x]), None),

            // Positive test cases.
            ((&val_map_a, vec![&key_str_a]), Some(&val_str_a)),
            ((&val_map_b, vec![&key_str_a]), Some(&val_seq_a)),
            ((&val_map_c, vec![&key_str_a]), Some(&val_nil)),
            ((&val_map_d, vec![&key_str_a]), Some(&val_map_a)),
            ((&val_map_a, vec![&key_str_b]), Some(&val_str_b)),
            ((&val_map_b, vec![&key_str_b]), Some(&val_seq_b)),
            ((&val_map_c, vec![&key_str_b]), Some(&val_nil)),
            ((&val_map_d, vec![&key_str_b]), Some(&val_map_b)),
            ((&val_map_a, vec![&key_str_c]), Some(&val_str_c)),
            ((&val_map_b, vec![&key_str_c]), Some(&val_seq_c)),
            ((&val_map_c, vec![&key_str_c]), Some(&val_nil)),
            ((&val_map_d, vec![&key_str_c]), Some(&val_map_c)),

            // Nested positive test cases.
            ((&val_map_d, vec![&key_str_a, &key_str_a]), Some(&val_str_a)),
            ((&val_map_d, vec![&key_str_b, &key_str_b]), Some(&val_seq_b)),
            ((&val_map_d, vec![&key_str_c, &key_str_c]), Some(&val_nil)),

        ];

        for (input, expected) in inputs_and_expected {
            let (val, key_path) = input;
            let produced = val.get_key_path(&key_path);
            assert_eq!(expected, produced);
        }
    }
}
