
mod matcher;

use std::path::Path;
use std::path::PathBuf;
use std::io::Result as IoResult;
use std::cmp::Ordering;

use strum::IntoEnumIterator;
use serde::Deserialize;

use crate::config::sorter::Sorter;
use crate::metadata::target::Target;

pub use self::matcher::Matcher;
pub use self::matcher::Error as MatcherError;

#[derive(Debug)]
pub enum Error {
    InvalidDirPath(PathBuf),
    CannotBuildMatcher(MatcherError),
    // CannotReadDir(std::io::Error),
    // CannotReadDirEntry(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Self::InvalidDirPath(ref p)
                => write!(f, "not a valid directory: {}", p.display()),
            Self::CannotBuildMatcher(ref err)
                => write!(f, "cannot build matcher: {}", err),
            // Self::CannotReadDir(ref err)
            //     => write!(f, "cannot read directory: {}", err),
            // Self::CannotReadDirEntry(ref err)
            //     => write!(f, "cannot read directory entry: {}", err),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::InvalidDirPath(..) => None,
            Self::CannotBuildMatcher(ref err) => Some(err),
            // Self::CannotReadDir(ref err) => Some(err),
            // Self::CannotReadDirEntry(ref err) => Some(err),
        }
    }
}

enum FileOrDir {
    File,
    Dir,
}

/// A type that represents included/excluded item files and directories.
#[derive(Deserialize, Debug)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    include_files: Matcher,
    exclude_files: Matcher,
    include_dirs: Matcher,
    exclude_dirs: Matcher,
}

impl Default for Selection {
    fn default() -> Self {
        // TODO: Replace with `StaticVec` once released for stable Rust.
        let excluded_patterns = Target::iter()
            .map(|ml| format!("{}*", ml.default_file_name()))
            .collect::<Vec<_>>()
        ;

        let include_files = Matcher::any();
        let exclude_files = Matcher::build(&excluded_patterns).unwrap();
        let include_dirs = Matcher::any();
        let exclude_dirs = Matcher::empty();

        Self::new(include_files, exclude_files, include_dirs, exclude_dirs)
    }
}

impl Selection {
    pub fn new(
        include_files: Matcher,
        exclude_files: Matcher,
        include_dirs: Matcher,
        exclude_dirs: Matcher,
    ) -> Self
    {
        Self { include_files, exclude_files, include_dirs, exclude_dirs, }
    }

    pub fn from_patterns<S>(
        include_file_patterns: &[S],
        exclude_file_patterns: &[S],
        include_dir_patterns: &[S],
        exclude_dir_patterns: &[S],
    ) -> Result<Self, Error>
    where
        S: AsRef<str>,
    {
        let include_files = Matcher::build(include_file_patterns).map_err(Error::CannotBuildMatcher)?;
        let exclude_files = Matcher::build(exclude_file_patterns).map_err(Error::CannotBuildMatcher)?;
        let include_dirs = Matcher::build(include_dir_patterns).map_err(Error::CannotBuildMatcher)?;
        let exclude_dirs = Matcher::build(exclude_dir_patterns).map_err(Error::CannotBuildMatcher)?;

        Ok(Self::new(include_files, exclude_files, include_dirs, exclude_dirs))
    }

    fn is_pattern_match<P: AsRef<Path>>(&self, path: P, fod: FileOrDir) -> bool {
        let (inc, exc) = match fod {
            FileOrDir::File => (&self.include_files, &self.exclude_files),
            FileOrDir::Dir => (&self.include_dirs, &self.exclude_dirs),
        };

        inc.is_match(&path) && !exc.is_match(&path)
    }

    /// Returns true if the path matches according to the file matcher.
    /// In order to be a pattern match, the path must match the include filter,
    /// and must NOT match the exclude filter.
    /// Note that this method assumes the path is a file, and uses only the
    /// lexical content of the path; it does not access the filesystem.
    pub fn is_file_pattern_match<P: AsRef<Path>>(&self, path: P) -> bool {
        self.is_pattern_match(path, FileOrDir::File)
    }

    /// Returns true if the path matches according to the directory matcher.
    /// In order to be a pattern match, the path must match the include filter,
    /// and must NOT match the exclude filter.
    /// Note that this method assumes the path is a directory, and uses only the
    /// lexical content of the path; it does not access the filesystem.
    pub fn is_dir_pattern_match<P: AsRef<Path>>(&self, path: P) -> bool {
        self.is_pattern_match(path, FileOrDir::Dir)
    }

    /// Returns true if a path is selected.
    /// This accesses the filesystem to tell if the path is a file or directory.
    pub fn is_selected<P: AsRef<Path>>(&self, path: P) -> Result<bool, std::io::Error> {
        let file_info = std::fs::metadata(&path)?;

        Ok(
            if file_info.is_file() { self.is_file_pattern_match(path) }
            else if file_info.is_dir() { self.is_dir_pattern_match(path) }
            else { false }
        )
    }

    /// Selects paths inside a directory that match this `Selection`.
    // NOTE: This returns two "levels" of `Error`, a top-level one for any error
    //       relating to accessing the passed-in directory path, and a `Vec` of
    //       `Result`s for errors encountered when iterating over sub-paths.
    pub fn select_in_dir(&self, dir_path: &Path) -> IoResult<Vec<IoResult<PathBuf>>> {
        // Try to open the path as a directory, handle the error as appropriate.
        let dir_reader = dir_path.read_dir()?;

        let sub_path_results =
            dir_reader
            .filter_map(|res| {
                match res {
                    Ok(dir_entry) => {
                        let sub_item_path = dir_entry.path();
                        match self.is_selected(&sub_item_path) {
                            Ok(true) => Some(Ok(sub_item_path)),
                            Ok(false) => None,
                            Err(err) => Some(Err(err)),
                        }
                    },
                    Err(err) => Some(Err(err)),
                }
            })
            .collect()
        ;

        Ok(sub_path_results)
    }

    /// Selects paths inside a directory that match this `Selection`, and sorts them.
    pub fn select_in_dir_sorted(&self, dir_path: &Path, sorter: Sorter) -> IoResult<Vec<IoResult<PathBuf>>> {
        let mut sel_item_paths = self.select_in_dir(dir_path)?;

        sel_item_paths.sort_by(|x, y| {
            match (x, y) {
                (Ok(a), Ok(b)) => sorter.path_sort_cmp(&a, &b),
                (Err(_), Ok(_)) => Ordering::Less,
                (Ok(_), Err(_)) => Ordering::Greater,
                (Err(_), Err(_)) => Ordering::Equal,
            }
        });

        Ok(sel_item_paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::File;

    use tempfile::Builder;
    use tempfile::TempDir;

    use maplit::hashset;

    use crate::config::sorter::Sorter;

    fn create_test_dir(name: &str) -> TempDir {
        let temp = Builder::new().suffix(name).tempdir().expect("unable to create temp directory");

        {
            let path = temp.path();

            let file_names = vec![
                "music.flac",
                "music.wav",
                "music.aac",
                "music.mp3",
                "music.ogg",
                "item",
                "self",
                "item.yml",
                "self.yml",
                "item.flac",
                "self.flac",
            ];

            for file_name in file_names {
                File::create(path.join(file_name)).expect("unable to create temp file");
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }

        temp
    }

    #[test]
    fn default() {
        let selection = Selection::default();

        assert!(selection.is_file_pattern_match("all"));
        assert!(selection.is_file_pattern_match("files"));
        assert!(selection.is_file_pattern_match("should"));
        assert!(selection.is_file_pattern_match("pass"));
        assert!(selection.is_file_pattern_match("except"));
        assert!(selection.is_file_pattern_match("for"));
        assert!(selection.is_file_pattern_match("these"));
        assert!(!selection.is_file_pattern_match("item",));
        assert!(!selection.is_file_pattern_match("self",));

        assert!(selection.is_dir_pattern_match("all"));
        assert!(selection.is_dir_pattern_match("dirs"));
        assert!(selection.is_dir_pattern_match("should"));
        assert!(selection.is_dir_pattern_match("pass"));
        assert!(selection.is_dir_pattern_match("even"));
        assert!(selection.is_dir_pattern_match("including"));
        assert!(selection.is_dir_pattern_match("item"));
        assert!(selection.is_dir_pattern_match("self"));
    }

    #[test]
    fn deserialization() {
        // A single pattern for each of include and exclude.
        let text = "{ include_files: '*.flac', exclude_files: '*.mp3' }";
        let selection: Selection = serde_yaml::from_str(&text).unwrap();

        assert_eq!(selection.is_file_pattern_match("path/to/music.flac"), true);
        assert_eq!(selection.is_file_pattern_match("path/to/music.mp3"), false);
        assert_eq!(selection.is_file_pattern_match("path/to/music.wav"), false);

        // Multiple patterns for each of include and exclude.
        let text = "{ include_files: ['*.flac', '*.wav'], exclude_files: ['*.mp3', '*.ogg'] }";
        let selection: Selection = serde_yaml::from_str(&text).unwrap();

        assert_eq!(selection.is_file_pattern_match("path/to/music.flac"), true);
        assert_eq!(selection.is_file_pattern_match("path/to/music.wav"), true);
        assert_eq!(selection.is_file_pattern_match("path/to/music.mp3"), false);
        assert_eq!(selection.is_file_pattern_match("path/to/music.ogg"), false);
        assert_eq!(selection.is_file_pattern_match("path/to/music.aac"), false);

        // Using a default value for missing include patterns.
        let text = "{ exclude_files: ['*.mp3', '*.ogg'] }";
        let selection: Selection = serde_yaml::from_str(&text).unwrap();

        assert_eq!(selection.is_file_pattern_match("path/to/music.flac"), true);
        assert_eq!(selection.is_file_pattern_match("path/to/music.wav"), true);
        assert_eq!(selection.is_file_pattern_match("path/to/music.aac"), true);
        assert_eq!(selection.is_file_pattern_match("path/to/music.mpc"), true);
        assert_eq!(selection.is_file_pattern_match("path/to/music.mp3"), false);
        assert_eq!(selection.is_file_pattern_match("path/to/music.ogg"), false);

        // Using a default value for missing exclude patterns.
        let text = "{ include_files: ['*.flac', '*.wav'] }";
        let selection: Selection = serde_yaml::from_str(&text).unwrap();

        assert_eq!(selection.is_file_pattern_match("path/to/music.flac"), true);
        assert_eq!(selection.is_file_pattern_match("path/to/music.wav"), true);
        assert_eq!(selection.is_file_pattern_match("path/to/item.flac"), false);
        assert_eq!(selection.is_file_pattern_match("path/to/self.flac"), false);
        assert_eq!(selection.is_file_pattern_match("path/to/music.aac"), false);
        assert_eq!(selection.is_file_pattern_match("path/to/music.mpc"), false);
        assert_eq!(selection.is_file_pattern_match("path/to/music.mp3"), false);
        assert_eq!(selection.is_file_pattern_match("path/to/music.ogg"), false);
    }

    #[test]
    fn is_pattern_match() {
        let selection = Selection::from_patterns(&["*.flac"], &["*.mp3"], &["*"], &[] as &[&str]).unwrap();

        assert_eq!(selection.is_file_pattern_match("path/to/music.flac"), true);
        assert_eq!(selection.is_file_pattern_match("path/to/music.mp3"), false);
        assert_eq!(selection.is_file_pattern_match("path/to/music.wav"), false);

        let selection = Selection::from_patterns(&["*.flac", "*.wav"], &["*.mp3", "*.ogg"], &["*"], &[] as &[&str]).unwrap();

        assert_eq!(selection.is_file_pattern_match("path/to/music.flac"), true);
        assert_eq!(selection.is_file_pattern_match("path/to/music.wav"), true);
        assert_eq!(selection.is_file_pattern_match("path/to/music.mp3"), false);
        assert_eq!(selection.is_file_pattern_match("path/to/music.ogg"), false);
        assert_eq!(selection.is_file_pattern_match("path/to/music.aac"), false);

        let selection = Selection::from_patterns(&["*"], &["*.mp3", "*.ogg"], &["*"], &[] as &[&str]).unwrap();

        assert_eq!(selection.is_file_pattern_match("path/to/music.flac"), true);
        assert_eq!(selection.is_file_pattern_match("path/to/music.wav"), true);
        assert_eq!(selection.is_file_pattern_match("path/to/music.aac"), true);
        assert_eq!(selection.is_file_pattern_match("path/to/music.mpc"), true);
        assert_eq!(selection.is_file_pattern_match("path/to/music.mp3"), false);
        assert_eq!(selection.is_file_pattern_match("path/to/music.ogg"), false);

        let selection = Selection::from_patterns(&["*.flac", "*.wav"], &["item*", "self*"], &["*"], &[] as &[&str]).unwrap();

        assert_eq!(selection.is_file_pattern_match("path/to/music.flac"), true);
        assert_eq!(selection.is_file_pattern_match("path/to/music.wav"), true);
        assert_eq!(selection.is_file_pattern_match("path/to/item.flac"), false);
        assert_eq!(selection.is_file_pattern_match("path/to/self.flac"), false);
        assert_eq!(selection.is_file_pattern_match("path/to/music.aac"), false);
        assert_eq!(selection.is_file_pattern_match("path/to/music.mpc"), false);
        assert_eq!(selection.is_file_pattern_match("path/to/music.mp3"), false);
        assert_eq!(selection.is_file_pattern_match("path/to/music.ogg"), false);
    }

    #[test]
    fn select_in_dir() {
        let temp_dir = create_test_dir("test_select_in_dir");
        let path = temp_dir.path();

        let inputs_and_expected = vec![
            (
                (vec!["music*"], vec!["*.mp3", "*.ogg", "*.aac"], vec!["*"], Vec::<&str>::new()),
                hashset![
                    path.join("music.flac"),
                    path.join("music.wav"),
                ],
            ),
            (
                (vec!["*.flac"], vec![], vec!["*"], Vec::<&str>::new()),
                hashset![
                    path.join("music.flac"),
                    path.join("item.flac"),
                    path.join("self.flac"),
                ],
            ),
            (
                (vec!["music*"], vec![], vec!["*"], Vec::<&str>::new()),
                hashset![
                    path.join("music.flac"),
                    path.join("music.wav"),
                    path.join("music.aac"),
                    path.join("music.mp3"),
                    path.join("music.ogg"),
                ],
            ),
            (
                (vec!["item.*", "self.*"], vec!["*.flac"], vec!["*"], Vec::<&str>::new()),
                hashset![
                    path.join("item.yml"),
                    path.join("self.yml"),
                ],
            ),
        ];

        for (input, expected) in inputs_and_expected {
            let (include_file_patterns, exclude_file_patterns, include_dir_patterns, exclude_dir_patterns) = input;

            let selection = Selection::from_patterns(
                &include_file_patterns,
                &exclude_file_patterns,
                &include_dir_patterns,
                &exclude_dir_patterns,
            ).unwrap();

            let produced =
                selection
                .select_in_dir(&path).unwrap()
                .into_iter()
                .map(|res| res.expect("unexpected IO error"))
                .collect()
            ;

            assert_eq!(expected, produced);
        }
    }

    #[test]
    fn select_in_dir_sorted() {
        let temp_dir = create_test_dir("test_select_in_dir_sorted");
        let path = temp_dir.path();

        let inputs_and_expected = vec![
            (
                (vec!["music*"], vec!["*.mp3", "*.ogg", "*.aac"], vec!["*"], Vec::<&str>::new(), Sorter::default()),
                vec![
                    path.join("music.flac"),
                    path.join("music.wav"),
                ],
            ),
            (
                (vec!["*.flac"], vec![], vec!["*"], Vec::<&str>::new(), Sorter::default()),
                vec![
                    path.join("item.flac"),
                    path.join("music.flac"),
                    path.join("self.flac"),
                ],
            ),
            (
                (vec!["music*"], vec![], vec!["*"], Vec::<&str>::new(), Sorter::default()),
                vec![
                    path.join("music.aac"),
                    path.join("music.flac"),
                    path.join("music.mp3"),
                    path.join("music.ogg"),
                    path.join("music.wav"),
                ],
            ),
            (
                (vec!["item.*", "self.*"], vec!["*.flac"], vec!["*"], Vec::<&str>::new(), Sorter::default()),
                vec![
                    path.join("item.yml"),
                    path.join("self.yml"),
                ],
            ),
        ];

        for (input, expected) in inputs_and_expected {
            let (include_file_patterns, exclude_file_patterns, include_dir_patterns, exclude_dir_patterns, sort_order) = input;

            let selection = Selection::from_patterns(
                &include_file_patterns,
                &exclude_file_patterns,
                &include_dir_patterns,
                &exclude_dir_patterns,
            ).unwrap();

            let produced =
                selection
                .select_in_dir_sorted(&path, sort_order).unwrap()
                .into_iter()
                .map(|res| res.expect("unexpected IO error"))
                .collect::<Vec<_>>()
            ;

            assert_eq!(expected, produced);
        }
    }
}
