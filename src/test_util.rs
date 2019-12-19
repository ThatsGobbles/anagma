#![cfg(test)]

use std::fs::DirBuilder;
use std::fs::File;
use std::path::Path;
use std::io::Write;
use std::time::Duration;
use std::collections::BTreeMap;

use tempfile::Builder;
use tempfile::TempDir;
use rust_decimal::Decimal;
use rand::seq::SliceRandom;

use crate::config::serialize_format::SerializeFormat;
use crate::metadata::target::Target;
use crate::metadata::value::Value;
use crate::metadata::block::Block;
use crate::metadata::structure::MetaStructure;

enum TEntry<'a> {
    Dir(&'a str, bool, &'a [TEntry<'a>]),
    File(&'a str, bool)
}

impl<'a> TEntry<'a> {
    pub fn name(&self) -> &str {
        match *self {
            TEntry::Dir(ref name, ..) => name,
            TEntry::File(ref name, ..) => name,
        }
    }

    pub fn include_spelunk_str(&self) -> bool {
        match *self {
            TEntry::Dir(_, b, ..) => b,
            TEntry::File(_, b, ..) => b,
        }
    }
}

const TEST_DIR_ENTRIES: &[TEntry] = &[
    // Well-behaved album.
    TEntry::Dir("ALBUM_01", false, &[
        TEntry::Dir("DISC_01", false, &[
            TEntry::File("TRACK_01", false),
            TEntry::File("TRACK_02", true),
            TEntry::File("TRACK_03", false),
        ]),
        TEntry::Dir("DISC_02", true, &[
            TEntry::File("TRACK_01", false),
            TEntry::File("TRACK_02", false),
            TEntry::File("TRACK_03", false),
        ]),
    ]),

    // Album with a disc and tracks, and loose tracks not on a disc.
    TEntry::Dir("ALBUM_02", false, &[
        TEntry::Dir("DISC_01", true, &[
            TEntry::File("TRACK_01", false),
            TEntry::File("TRACK_02", false),
            TEntry::File("TRACK_03", false),
        ]),
        TEntry::File("TRACK_01", false),
        TEntry::File("TRACK_02", true),
        TEntry::File("TRACK_03", false),
    ]),

    // Album with discs and tracks, and subtracks on one disc.
    TEntry::Dir("ALBUM_03", true, &[
        TEntry::Dir("DISC_01", true, &[
            TEntry::File("TRACK_01", true),
            TEntry::File("TRACK_02", true),
            TEntry::File("TRACK_03", true),
        ]),
        TEntry::Dir("DISC_02", true, &[
            TEntry::Dir("TRACK_01", true, &[
                TEntry::File("SUBTRACK_01", true),
                TEntry::File("SUBTRACK_02", true),
            ]),
            TEntry::Dir("TRACK_02", true, &[
                TEntry::File("SUBTRACK_01", true),
                TEntry::File("SUBTRACK_02", true),
            ]),
            TEntry::File("TRACK_03", true),
            TEntry::File("TRACK_04", true),
        ]),
    ]),

    // Album that consists of one file.
    TEntry::File("ALBUM_04", false),

    // A very messed-up album.
    TEntry::Dir("ALBUM_05", false, &[
        TEntry::Dir("DISC_01", true, &[
            TEntry::File("SUBTRACK_01", true),
            TEntry::File("SUBTRACK_02", false),
            TEntry::File("SUBTRACK_03", false),
        ]),
        TEntry::Dir("DISC_02", false, &[
            TEntry::Dir("TRACK_01", false, &[
                TEntry::File("SUBTRACK_01", true),
                TEntry::File("SUBTRACK_02", false),
            ]),
        ]),
        TEntry::File("TRACK_01", true),
        TEntry::File("TRACK_02", false),
        TEntry::File("TRACK_03", false),
    ]),
];

const MEDIA_FILE_EXT: &str = "flac";

// LEARN: Why unable to use IntoIterator<Item = Entry>?
fn create_test_dir_entries<'a, P, S>(identifier: S, target_dir_path: P, subentries: &[TEntry<'a>], db: &DirBuilder, staggered: bool)
where P: AsRef<Path>,
      S: AsRef<str>,
{
    let identifier = identifier.as_ref();
    let target_dir_path = target_dir_path.as_ref();

    // Create self meta file for this directory.
    let mut self_meta_file = File::create(target_dir_path.join("self.yml")).expect("unable to create self meta file");
    writeln!(self_meta_file, "const_key: const_val\nself_key: self_val\n{}_self_key: {}_self_val\noverridden: {}_self", identifier, identifier, identifier).expect("unable to write to self meta file");
    // writeln!(self_meta_file, "const_key: const_val").expect("unable to write to self meta file");
    // writeln!(self_meta_file, "self_key: self_val").expect("unable to write to self meta file");
    // writeln!(self_meta_file, "{}_self_key: {}_self_val", identifier, identifier).expect("unable to write to self meta file");
    // writeln!(self_meta_file, "overridden: {}_self", identifier).expect("unable to write to self meta file");

    // Create all sub-entries, and collect info to create item metadata.
    let mut item_meta_contents = String::new();
    for subentry in subentries.into_iter() {
        // helper(&subentry, &target_dir_path, db /*, imt*/);

        match *subentry {
            TEntry::File(name, ..) => {
                File::create(target_dir_path.join(name).with_extension(MEDIA_FILE_EXT)).expect("unable to create file");
            },
            TEntry::Dir(name, _, new_subentries) => {
                let new_dir_path = target_dir_path.join(name);
                db.create(&new_dir_path).expect("unable to create dir");

                create_test_dir_entries(name, new_dir_path, new_subentries, db, staggered);
            }
        }

        let entry_string = format!("- const_key: const_val\n  item_key: item_val\n  {}_item_key: {}_item_val\n  overridden: {}_item\n", subentry.name(), subentry.name(), subentry.name());
        item_meta_contents.push_str(&entry_string);

        if staggered && subentry.include_spelunk_str() {
            // Add unique meta keys that are intended for child aggregating tests.
            item_meta_contents.push_str(&format!("  staggered_key:\n"));
            item_meta_contents.push_str(&format!("    sub_key_a: {}_sub_val_a\n", subentry.name()));
            item_meta_contents.push_str(&format!("    sub_key_b: {}_sub_val_b\n", subentry.name()));
            item_meta_contents.push_str(&format!("    sub_key_c:\n"));
            item_meta_contents.push_str(&format!("      sub_sub_key_a: {}_sub_sub_val_a\n", subentry.name()));
            item_meta_contents.push_str(&format!("      sub_sub_key_b: {}_sub_sub_val_b\n", subentry.name()));
        }
    }

    // Create item meta file for all items in this directory.
    let mut item_meta_file = File::create(target_dir_path.join("item.yml")).expect("unable to create item meta file");
    item_meta_file.write_all(item_meta_contents.as_bytes()).expect("unable to write to item meta file");
}

fn create_temp_media_test_dir_helper(name: &str, staggered: bool) -> TempDir {
    let root_dir = Builder::new().suffix(name).tempdir().expect("unable to create temp directory");
    let db = DirBuilder::new();

    create_test_dir_entries("ROOT", root_dir.path(), TEST_DIR_ENTRIES, &db, staggered);

    std::thread::sleep(Duration::from_millis(1));
    root_dir
}

pub fn create_temp_media_test_dir(name: &str) -> TempDir {
    create_temp_media_test_dir_helper(name, false)
}

trait TestSerialize {
    const INDENT: &'static str = "  ";
    const YAML_LIST_ITEM: &'static str = "- ";

    fn indent_chunk(s: String) -> String {
        let mut to_join = vec![];

        for line in s.lines() {
            to_join.push(format!("{}{}", Self::INDENT, line));
        }

        to_join.join("\n")
    }

    fn indent_yaml_list_chunk(s: String) -> String {
        let mut to_join = vec![];

        for (i, line) in s.lines().enumerate() {
            let prefix = if i == 0 { Self::YAML_LIST_ITEM } else { Self::INDENT };

            to_join.push(format!("{}{}", prefix, line));
        }

        to_join.join("\n")
    }

    fn to_serialized_chunk(&self, serialize_format: SerializeFormat) -> String;
}

impl TestSerialize for MetaStructure {
    fn to_serialized_chunk(&self, serialize_format: SerializeFormat) -> String {
        match self {
            &MetaStructure::One(ref mb) => Value::Mapping(mb.clone()).to_serialized_chunk(serialize_format),
            &MetaStructure::Seq(ref mb_seq) => {
                Value::Sequence(
                    mb_seq
                        .into_iter()
                        .map(|v| Value::Mapping(v.clone()))
                        .collect()
                ).to_serialized_chunk(serialize_format)
            },
            &MetaStructure::Map(ref mb_map) => {
                Value::Mapping(
                    mb_map
                        .into_iter()
                        .map(|(k, v)| (k.clone(), Value::Mapping(v.clone())))
                        .collect()
                ).to_serialized_chunk(serialize_format)
            },
        }
    }
}

impl TestSerialize for Value {
    fn to_serialized_chunk(&self, serialize_format: SerializeFormat) -> String {
        match (serialize_format, self) {
            (SerializeFormat::Json, &Self::Null) => "null".into(),
            (SerializeFormat::Yaml, &Self::Null) => "~".into(),
            (SerializeFormat::Json, &Self::String(ref s)) => format!(r#""{}""#, s),
            (SerializeFormat::Yaml, &Self::String(ref s)) => s.clone(),
            (_, &Self::Integer(i)) => format!("{}", i),
            (_, &Self::Decimal(ref d)) => format!("{}", d),
            (_, &Self::Boolean(b)) => format!("{}", b),
            (SerializeFormat::Json, &Self::Sequence(ref seq)) => {
                let mut val_chunks = vec![];

                for val in seq {
                    let val_chunk = val.to_serialized_chunk(serialize_format);

                    let val_chunk = Self::indent_chunk(val_chunk);

                    val_chunks.push(val_chunk);
                }

                if val_chunks.len() > 0 {
                    format!("[\n{}\n]", val_chunks.join(",\n"))
                }
                else {
                    String::from("[]")
                }
            },
            (SerializeFormat::Yaml, &Self::Sequence(ref seq)) => {
                let mut val_chunks = vec![];

                for val in seq {
                    let val_chunk = val.to_serialized_chunk(serialize_format);

                    let val_chunk = Self::indent_yaml_list_chunk(val_chunk);

                    val_chunks.push(val_chunk);
                }

                if val_chunks.len() > 0 {
                    format!("{}", val_chunks.join("\n"))
                }
                else {
                    String::from("[]")
                }
            },
            (SerializeFormat::Json, &Self::Mapping(ref map)) => {
                let mut kv_pair_chunks = vec![];

                for (key, val) in map {
                    let val_chunk = val.to_serialized_chunk(serialize_format);

                    let kv_pair_chunk = format!(r#""{}": {}"#, key, val_chunk);

                    let kv_pair_chunk = Self::indent_chunk(kv_pair_chunk);

                    kv_pair_chunks.push(kv_pair_chunk);
                }

                if kv_pair_chunks.len() > 0 {
                    format!("{{\n{}\n}}", kv_pair_chunks.join(",\n"))
                }
                else {
                    String::from("{}")
                }
            },
            (SerializeFormat::Yaml, &Self::Mapping(ref map)) => {
                let mut kv_pair_chunks = vec![];

                for (key, val) in map {
                    let val_chunk = {
                        let val_chunk = val.to_serialized_chunk(serialize_format);

                        match val {
                            Self::Sequence(..) | Self::Mapping(..) => format!("\n{}", Self::indent_chunk(val_chunk)),
                            _ => format!(" {}", val_chunk),
                        }
                    };

                    let kv_pair_chunk = format!("{}:{}", key, val_chunk);

                    kv_pair_chunks.push(kv_pair_chunk);
                }

                if kv_pair_chunks.len() > 0 {
                    format!("{}", kv_pair_chunks.join("\n"))
                }
                else {
                    String::from("{}")
                }
            },
        }
    }
}

pub(crate) struct TestUtil;

impl TestUtil {
    pub const STRING_KEY: &'static str = "string_key";
    pub const INTEGER_KEY: &'static str = "integer_key";
    pub const DECIMAL_KEY: &'static str = "decimal_key";
    pub const BOOLEAN_KEY: &'static str = "boolean_key";
    pub const NULL_KEY: &'static str = "null_key";
    pub const SEQUENCE_KEY: &'static str = "sequence_key";
    pub const MAPPING_KEY: &'static str = "mapping_key";

    pub fn sample_string() -> Value {
        Value::String(String::from("string"))
    }

    pub fn sample_integer() -> Value {
        Value::Integer(27)
    }

    pub fn sample_decimal() -> Value {
        Value::Decimal(Decimal::new(31415.into(), 4))
    }

    pub fn sample_boolean() -> Value {
        Value::Boolean(true)
    }

    pub fn sample_null() -> Value {
        Value::Null
    }

    pub fn core_flat_sequence() -> Vec<Value> {
        vec![
            Self::sample_string(),
            Self::sample_integer(),
            Self::sample_decimal(),
            Self::sample_boolean(),
            Self::sample_null(),
        ]
    }

    pub fn core_flat_mapping() -> BTreeMap<String, Value> {
        btreemap![
            String::from(Self::STRING_KEY) => Self::sample_string(),
            String::from(Self::INTEGER_KEY) => Self::sample_integer(),
            String::from(Self::DECIMAL_KEY) => Self::sample_decimal(),
            String::from(Self::BOOLEAN_KEY) => Self::sample_boolean(),
            String::from(Self::NULL_KEY) => Self::sample_null(),
        ]
    }

    pub fn core_nested_mapping() -> BTreeMap<String, Value> {
        let mut map = Self::core_flat_mapping();

        map.insert(String::from(Self::SEQUENCE_KEY), Self::sample_flat_sequence());
        map.insert(String::from(Self::MAPPING_KEY), Self::sample_flat_mapping());

        map
    }

    pub fn sample_flat_sequence() -> Value {
        Value::Sequence(Self::core_flat_sequence())
    }

    pub fn sample_flat_mapping() -> Value {
        Value::Mapping(Self::core_flat_mapping())
    }

    pub fn core_number_sequence(int_max: i64, dec_extremes: bool, shuffle: bool, include_zero: bool) -> Vec<Value> {
        let mut nums = vec![];

        for i in 1..=int_max {
            nums.push(Value::Integer(i));
            nums.push(Value::Integer(-i));

            // Add -0.5 decimal values.
            let m = (i - 1) * 10 + 5;
            nums.push(Value::Decimal(Decimal::new(m.into(), 1)));
            nums.push(Value::Decimal(Decimal::new((-m).into(), 1)));
        }

        if dec_extremes {
            // These are +/-(int_max + 0.5).
            let m = int_max * 10 + 5;
            nums.push(Value::Decimal(Decimal::new(m.into(), 1)));
            nums.push(Value::Decimal(Decimal::new((-m).into(), 1)));
        }

        if include_zero {
            nums.push(Value::Integer(0));
        }

        if shuffle {
            nums.shuffle(&mut rand::thread_rng());
        }

        nums
    }

    pub fn sample_number_sequence(int_max: i64, dec_extremes: bool, shuffle: bool, include_zero: bool) -> Value {
        Value::Sequence(Self::core_number_sequence(int_max, dec_extremes, shuffle, include_zero))
    }

    // pub fn sample_nested_sequence() -> Value {
    //     Value::Sequence(Self::core_nested_sequence())
    // }

    // pub fn sample_nested_mapping() -> Value {
    //     Value::Mapping(Self::core_nested_mapping())
    // }

    pub fn sample_meta_block(meta_target: Target, target_name: &str, include_flag_key: bool) -> Block {
        let mut map = Self::core_nested_mapping();

        map.insert(
            String::from(format!("{}_key", meta_target.default_file_name())),
            Value::String(format!("{}_val", meta_target.default_file_name())),
        );

        map.insert(
            String::from("meta_target"),
            Value::String(String::from(meta_target.default_file_name())),
        );

        map.insert(
            String::from("target_file_name"),
            Value::String(String::from(target_name)),
        );

        if include_flag_key {
            map.insert(
                String::from("flag_key"),
                Value::String(String::from(target_name)),
            );
        }

        map
    }

    // /// Used for test scenarios where a target is not needed.
    // pub fn sample_naive_meta_block(target_name: &str, include_flag_key: bool) -> Block {
    //     let mut map = Self::core_nested_mapping();

    //     map.insert(
    //         String::from("target_file_name"),
    //         Value::String(String::from(target_name)),
    //     );

    //     if include_flag_key {
    //         map.insert(
    //             String::from("flag_key"),
    //             Value::String(String::from(target_name)),
    //         );
    //     }

    //     map
    // }

    // pub fn create_fixed_value_stream<'a, II>(mvs: II) -> FixedValueStream<'a>
    // where
    //     II: IntoIterator<Item = Value<'a>>,
    // {
    //     FixedValueStream::new(mvs.into_iter().map(|mv| (Cow::Borrowed(Path::new("dummy")), mv)))
    // }

    pub fn create_plain_fanout_test_dir(name: &str, fanout: usize, max_depth: usize) -> TempDir {
        let root_dir = Builder::new().suffix(name).tempdir().expect("unable to create temp directory");

        fn fill_dir(p: &Path, db: &DirBuilder, fanout: usize, breadcrumbs: Vec<usize>, max_depth: usize) {
            for i in 0..fanout {
                let mut new_breadcrumbs = breadcrumbs.clone();

                new_breadcrumbs.push(i);

                let name = if new_breadcrumbs.len() == 0 {
                    String::from("ROOT")
                }
                else {
                    new_breadcrumbs.iter().map(|n| format!("{}", n)).collect::<Vec<_>>().join("_")
                };

                let new_path = p.join(&name);

                if breadcrumbs.len() >= max_depth {
                    // Create files.
                    File::create(&new_path).expect("unable to create file");
                }
                else {
                    // Create dirs and then recurse.
                    db.create(&new_path).expect("unable to create directory");
                    fill_dir(&new_path, &db, fanout, new_breadcrumbs, max_depth);
                }
            }
        }

        let db = DirBuilder::new();

        fill_dir(root_dir.path(), &db, fanout, vec![], max_depth);

        root_dir
    }

    pub fn flag_set_by_default(depth_left: usize, fanout_index: usize) -> bool {
        ((depth_left % 2 == 1) ^ (fanout_index % 2 == 1)) && depth_left <= 1
    }

    pub fn flag_set_by_all(_: usize, _: usize) -> bool {
        true
    }

    pub fn flag_set_by_none(_: usize, _: usize) -> bool {
        false
    }

    pub fn create_meta_fanout_test_dir(name: &str, fanout: usize, max_depth: usize, flag_set_by: fn(usize, usize) -> bool) -> TempDir
    {
        let root_dir = Builder::new().suffix(name).tempdir().expect("unable to create temp directory");

        fn fill_dir(p: &Path, db: &DirBuilder, parent_name: &str, fanout: usize, breadcrumbs: Vec<usize>, max_depth: usize, flag_set_by: fn(usize, usize) -> bool)
        {
            // Create self meta file.
            let mut self_meta_file = File::create(p.join("self.json")).expect("unable to create self meta file");

            let self_meta_struct = MetaStructure::One(TestUtil::sample_meta_block(Target::Parent, &parent_name, false));
            let self_lines = self_meta_struct.to_serialized_chunk(SerializeFormat::Json);
            writeln!(self_meta_file, "{}", self_lines).expect("unable to write to self meta file");

            let mut item_meta_blocks = vec![];

            for i in 0..fanout {
                let mut new_breadcrumbs = breadcrumbs.clone();

                new_breadcrumbs.push(i);

                let name = if new_breadcrumbs.len() == 0 {
                    String::from("ROOT")
                }
                else {
                    new_breadcrumbs.iter().map(|n| format!("{}", n)).collect::<Vec<_>>().join("_")
                };

                if breadcrumbs.len() >= max_depth {
                    // Create files.
                    let new_path = p.join(&name);
                    File::create(&new_path).expect("unable to create item file");
                } else {
                    // Create dirs and then recurse.
                    let new_path = p.join(&name);
                    db.create(&new_path).expect("unable to create item directory");
                    fill_dir(&new_path, &db, &name, fanout, new_breadcrumbs, max_depth, flag_set_by);
                }

                let include_flag_key = flag_set_by(max_depth - breadcrumbs.len(), i);

                let item_meta_block = TestUtil::sample_meta_block(Target::Siblings, &name, include_flag_key);
                item_meta_blocks.push(item_meta_block);
            }

            // Create item meta file.
            let mut item_meta_file = File::create(p.join("item.json")).expect("unable to create item meta file");

            let item_meta_struct = MetaStructure::Seq(item_meta_blocks);
            let item_lines = item_meta_struct.to_serialized_chunk(SerializeFormat::Json);
            writeln!(item_meta_file, "{}", item_lines).expect("unable to write to item meta file");
        }

        let db = DirBuilder::new();

        fill_dir(root_dir.path(), &db, "ROOT", fanout, vec![], max_depth, flag_set_by);

        std::thread::sleep(Duration::from_millis(1));
        root_dir
    }

    pub fn i(i: i64) -> Value {
        Value::Integer(i)
    }

    pub fn d_raw(i: i64, e: u32) -> Decimal {
        Decimal::new(i.into(), e)
    }

    pub fn d(i: i64, e: u32) -> Value {
        Value::Decimal(Self::d_raw(i, e))
    }
}

#[cfg(test)]
mod tests {
    use super::TestUtil;
    use super::TestSerialize;

    use rust_decimal::Decimal;

    use crate::config::serialize_format::SerializeFormat;
    use crate::metadata::value::Value;

    #[test]
    fn test_create_meta_fanout_test_dir() {
        TestUtil::create_meta_fanout_test_dir("test_create_meta_fanout_test_dir", 3, 3, |_, _| true);
    }

    #[test]
    fn test_sample_number_sequence() {
        let i = TestUtil::i;
        let d = TestUtil::d;

        let test_cases = vec![
            (
                TestUtil::sample_number_sequence(2, false, false, false),
                Value::Sequence(vec![i(1), i(-1), d(5, 1), d(-5, 1), i(2), i(-2), d(15, 1), d(-15, 1)]),
            ),
            (
                TestUtil::sample_number_sequence(2, true, false, false),
                Value::Sequence(vec![i(1), i(-1), d(5, 1), d(-5, 1), i(2), i(-2), d(15, 1), d(-15, 1), d(25, 1), d(-25, 1)]),
            ),
            (
                TestUtil::sample_number_sequence(2, false, false, true),
                Value::Sequence(vec![i(1), i(-1), d(5, 1), d(-5, 1), i(2), i(-2), d(15, 1), d(-15, 1), i(0)]),
            ),
            (
                TestUtil::sample_number_sequence(2, true, false, true),
                Value::Sequence(vec![i(1), i(-1), d(5, 1), d(-5, 1), i(2), i(-2), d(15, 1), d(-15, 1), d(25, 1), d(-25, 1), i(0)]),
            ),
        ];

        for (input, expected) in test_cases {
            assert_eq!(input, expected);
        }
    }

    #[test]
    fn test_to_serialized_chunk() {
        let dec = Decimal::new(31415.into(), 4);

        let seq_a = Value::Sequence(vec![Value::Integer(27), Value::String("string".into())]);
        let seq_b = Value::Sequence(vec![Value::Boolean(false), Value::Null, Value::Decimal(dec)]);

        let seq_seq = Value::Sequence(vec![seq_a.clone(), seq_b.clone()]);

        let map = Value::Mapping(btreemap![
            "key_a".into() => seq_a.clone(),
            "key_b".into() => seq_b.clone(),
            "key_c".into() => seq_seq.clone(),
        ]);

        let inputs_and_expected = vec![
            (
                (seq_a.clone(), SerializeFormat::Json),
                "[\n  27,\n  \"string\"\n]",
            ),
            (
                (seq_a.clone(), SerializeFormat::Yaml),
                "- 27\n- string",
            ),
            (
                (seq_seq.clone(), SerializeFormat::Json),
                "[\n  [\n    27,\n    \"string\"\n  ],\n  [\n    false,\n    null,\n    3.1415\n  ]\n]",
            ),
            (
                (seq_seq.clone(), SerializeFormat::Yaml),
                "- - 27\n  - string\n- - false\n  - ~\n  - 3.1415",
            ),
            (
                (map.clone(), SerializeFormat::Json),
                "{\n  \"key_a\": [\n    27,\n    \"string\"\n  ],\n  \"key_b\": [\n    false,\n    null,\n    3.1415\n  ],\n  \"key_c\": [\n    [\n      27,\n      \"string\"\n    ],\n    [\n      false,\n      null,\n      3.1415\n    ]\n  ]\n}",
            ),
            (
                (map.clone(), SerializeFormat::Yaml),
                "key_a:\n  - 27\n  - string\nkey_b:\n  - false\n  - ~\n  - 3.1415\nkey_c:\n  - - 27\n    - string\n  - - false\n    - ~\n    - 3.1415",
            ),
        ];

        for (inputs, expected) in inputs_and_expected {
            let (mv, serialize_format) = inputs;

            let produced = mv.to_serialized_chunk(serialize_format);

            assert_eq!(expected, produced);
        }
    }
}
