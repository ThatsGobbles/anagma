///! Wrapper type for items on the consumer stack that behave as a sequence of meta values.

use std::convert::TryFrom;
use std::borrow::Cow;

use crate::metadata::types::MetaVal;
use crate::functions::op::operand::Operand;
use crate::functions::util::StreamAdaptor;
use crate::functions::Error;

pub enum IterableLike<'il> {
    StreamAdaptor(StreamAdaptor<'il>),
    Sequence(Vec<MetaVal<'il>>),
}

impl<'il> IterableLike<'il> {
    pub fn is_lazy(&self) -> bool {
        match self {
            &Self::StreamAdaptor(..) => true,
            &Self::Sequence(..) => false,
        }
    }

    pub fn is_eager(&self) -> bool {
        !self.is_lazy()
    }
}

impl<'il> From<IterableLike<'il>> for Operand<'il> {
    fn from(il: IterableLike<'il>) -> Self {
        match il {
            IterableLike::StreamAdaptor(stream) => Self::StreamAdaptor(stream),
            IterableLike::Sequence(sequence) => Self::Value(Cow::Owned(MetaVal::Seq(sequence))),
        }
    }
}

impl<'il> TryFrom<Operand<'il>> for IterableLike<'il> {
    type Error = Error;

    fn try_from(value: Operand<'il>) -> Result<Self, Self::Error> {
        match value {
            Operand::StreamAdaptor(s) => Ok(Self::StreamAdaptor(s)),
            Operand::Value(cow_mv) => Self::try_from(cow_mv.into_owned()),
            _ => Err(Error::NotIterable),
        }
    }
}

impl<'il> TryFrom<MetaVal<'il>> for IterableLike<'il> {
    type Error = Error;

    fn try_from(value: MetaVal<'il>) -> Result<Self, Self::Error> {
        match value {
            MetaVal::Seq(s) => Ok(Self::Sequence(s)),
            _ => Err(Error::NotIterable),
        }
    }
}

impl<'il> IntoIterator for IterableLike<'il> {
    type Item = Result<MetaVal<'il>, Error>;
    type IntoIter = IterLike<'il>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::StreamAdaptor(s) => IterLike::StreamAdaptor(s),
            Self::Sequence(s) => IterLike::Sequence(s.into_iter()),
        }
    }
}

pub enum IterLike<'il> {
    StreamAdaptor(StreamAdaptor<'il>),
    Sequence(std::vec::IntoIter<MetaVal<'il>>),
}

impl<'il> Iterator for IterLike<'il> {
    type Item = Result<MetaVal<'il>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            &mut Self::StreamAdaptor(ref mut s) => s.next(),
            &mut Self::Sequence(ref mut s) => s.next().map(Result::Ok),
        }
    }
}