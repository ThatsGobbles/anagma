use serde::Deserialize;

#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum Ooms {
    One(String),
    Many(Vec<String>),
}

impl Ooms {
    pub(crate) fn iter(&self) -> OomsIter {
        match self {
            Self::One(s) => OomsIter::One(Some(s.as_str())),
            Self::Many(ss) => OomsIter::Many(ss.iter()),
        }
    }
}

pub(crate) enum OomsIter<'a> {
    One(Option<&'a str>),
    Many(std::slice::Iter<'a, String>),
}

impl<'a> Iterator for OomsIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::One(o) => o.take(),
            Self::Many(it) => it.next().map(|s| s.as_str()),
        }
    }
}
