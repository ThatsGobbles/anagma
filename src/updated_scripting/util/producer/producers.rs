use std::collections::VecDeque;
use std::collections::HashSet;

use crate::metadata::stream::value::MetaValueStream;
use crate::metadata::types::MetaVal;
use crate::updated_scripting::Error;
use crate::updated_scripting::traits::Predicate;
use crate::updated_scripting::traits::Converter;

pub struct Source<'a>(MetaValueStream<'a>);

impl<'a> Source<'a> {
    pub fn new(mvs: MetaValueStream<'a>) -> Self {
        Self(mvs)
    }
}

impl<'a> Iterator for Source<'a> {
    type Item = Result<MetaVal, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|res| res.map(|(_, mv)| mv).map_err(Error::ValueStream))
    }
}

pub struct Fixed(std::vec::IntoIter<MetaVal>);

impl Fixed {
    pub fn new(v: Vec<MetaVal>) -> Self {
        Self(v.into_iter())
    }
}

impl Iterator for Fixed {
    type Item = Result<MetaVal, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(Result::Ok)
    }
}

impl From<Vec<MetaVal>> for Fixed {
    fn from(v: Vec<MetaVal>) -> Self {
        Fixed::new(v)
    }
}

pub struct Raw(std::vec::IntoIter<Result<MetaVal, Error>>);

impl Raw {
    pub fn new(v: Vec<Result<MetaVal, Error>>) -> Self {
        Self(v.into_iter())
    }
}

impl Iterator for Raw {
    type Item = Result<MetaVal, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl From<Vec<Result<MetaVal, Error>>> for Raw {
    fn from(v: Vec<Result<MetaVal, Error>>) -> Self {
        Raw::new(v)
    }
}

pub struct Flatten<I>(I, VecDeque<MetaVal>)
where I: Iterator<Item = Result<MetaVal, Error>>;

impl<I> Flatten<I>
where
    I: Iterator<Item = Result<MetaVal, Error>>,
{
    pub fn new(iter: I) -> Self {
        Self(iter, VecDeque::new())
    }
}

impl<I> Iterator for Flatten<I>
where
    I: Iterator<Item = Result<MetaVal, Error>>,
{
    type Item = Result<MetaVal, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        // Try to pop from the holding queue first.
        match self.1.pop_front() {
            // If there is an item in the holding queue, return it and do not advance the original iterator.
            Some(mv) => Some(Ok(mv)),

            // Advance the underlying iterator, and process the item as appropriate.
            None => {
                // Try to get the next item from the stream.
                match self.0.next()? {
                    Ok(MetaVal::Seq(seq)) => {
                        // Move all elements in the sequence into the queue.
                        self.1.extend(seq);
                        self.next()
                    },
                    o => Some(o),
                }
            },
        }
    }
}

pub struct Dedup<I>(I, Option<MetaVal>)
where I: Iterator<Item = Result<MetaVal, Error>>;

impl<I> Dedup<I>
where
    I: Iterator<Item = Result<MetaVal, Error>>,
{
    pub fn new(iter: I) -> Self {
        Self(iter, None)
    }
}

impl<I> Iterator for Dedup<I>
where
    I: Iterator<Item = Result<MetaVal, Error>>,
{
    type Item = Result<MetaVal, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let res = self.0.next()?;

            return match res {
                Err(err) => Some(Err(err)),
                Ok(curr_val) => {
                    if Some(&curr_val) == self.1.as_ref() {
                        // Delegate to the next iteration.
                        continue
                    }
                    else {
                        // A non-duplicate was found.
                        self.1 = Some(curr_val.clone());
                        Some(Ok(curr_val))
                    }
                },
            }
        }
    }
}

pub struct Unique<I>(I, HashSet<MetaVal>)
where I: Iterator<Item = Result<MetaVal, Error>>;

impl<I> Unique<I>
where
    I: Iterator<Item = Result<MetaVal, Error>>,
{
    pub fn new(iter: I) -> Self {
        Self(iter, HashSet::new())
    }
}

impl<I> Iterator for Unique<I>
where
    I: Iterator<Item = Result<MetaVal, Error>>,
{
    type Item = Result<MetaVal, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let res = self.0.next()?;

            return match res {
                Err(err) => Some(Err(err)),
                Ok(curr_val) => {
                    if self.1.contains(&curr_val) {
                        // Delegate to the next iteration.
                        continue
                    }
                    else {
                        self.1.insert(curr_val.clone());
                        Some(Ok(curr_val))
                    }
                },
            }
        }
    }
}

pub struct Filter<I, P>(I, P)
where
    I: Iterator<Item = Result<MetaVal, Error>>,
    P: Predicate,
;

impl<I, P> Filter<I, P>
where
    I: Iterator<Item = Result<MetaVal, Error>>,
    P: Predicate,
{
    pub fn new(iter: I, pred: P) -> Self {
        Self(iter, pred)
    }
}

impl<I, P> Iterator for Filter<I, P>
where
    I: Iterator<Item = Result<MetaVal, Error>>,
    P: Predicate,
{
    type Item = Result<MetaVal, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let res = self.0.next()?;
            return match res {
                Ok(mv) => {
                    match self.1.test(&mv) {
                        true => Some(Ok(mv)),
                        false => continue,

                        // Err(err) => Some(Err(err)),
                        // Ok(b) => {
                        //     if b { Some(Ok(mv)) }
                        //     else { continue }
                        // },
                    }
                },
                Err(err) => Some(Err(err)),
            }
        }
    }
}

pub struct Map<I, C>(I, C)
where
    I: Iterator<Item = Result<MetaVal, Error>>,
    C: Converter,
;

impl<I, C> Map<I, C>
where
    I: Iterator<Item = Result<MetaVal, Error>>,
    C: Converter,
{
    pub fn new(iter: I, conv: C) -> Self {
        Self(iter, conv)
    }
}

impl<I, C> Iterator for Map<I, C>
where
    I: Iterator<Item = Result<MetaVal, Error>>,
    C: Converter,
{
    type Item = Result<MetaVal, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.next()? {
            Ok(mv) => Some(Ok(self.1.convert(mv))),
            // Ok(mv) => Some(self.1.convert(mv)),
            Err(err) => Some(Err(err)),
        }
    }
}
