use std::convert::TryInto;

use metadata::types::MetaVal;
use metadata::resolver::Error;
use metadata::resolver::ops::Op;
use metadata::resolver::ops::Operand;
use metadata::resolver::ops::OperandStack;
use metadata::resolver::context::ResolverContext;
use metadata::resolver::streams::Stream;
use metadata::resolver::number_like::NumberLike;
use metadata::resolver::iterable_like::IterableLike;
use metadata::stream::block::FileMetaBlockStream;
use metadata::stream::value::MetaValueStream;
use util::file_walkers::ParentFileWalker;
use util::file_walkers::ChildFileWalker;

#[derive(Clone, Copy, Debug)]
pub enum UnaryOp {
    // (Iterable<V>) -> Sequence<V>
    Collect,
    // (Iterable<V>) -> Integer
    Count,
    // (Iterable<V>) -> V
    First,
    // (Iterable<V>) -> V
    Last,
    // (Iterable<Number>) -> Number
    Max,
    // (Iterable<Number>) -> Number
    Min,
    // (Iterable<V>) -> Sequence<V>
    Rev,
    // (Iterable<Number>) -> Number
    Sum,
    // (Iterable<Number>) -> Number
    Product,
    // (Iterable<V>) -> Boolean
    AllEqual,
    // (Iterable<V>) -> Sequence<V>
    Sort,

    // (KeyPath) -> Stream<V>
    ParentsRef,
    // (KeyPath) -> Stream<V>
    ChildrenRef,
}

impl Op for UnaryOp {
    fn process<'bo>(&self, rc: &ResolverContext<'bo>, stack: &mut OperandStack<'bo>) -> Result<(), Error> {
        let output_operand = match self {
            &Self::Collect | &Self::Rev | &Self::Sort => {
                let mut coll = match stack.pop_iterable_like()? {
                    IterableLike::Stream(st) => st.collect::<Result<Vec<_>, _>>()?,
                    IterableLike::Sequence(sq) => sq,
                };

                match self {
                    &Self::Rev => { coll.reverse(); },
                    // TODO: How do sorting maps work?
                    &Self::Sort => { coll.sort(); },
                    _ => {},
                }

                Operand::Value(MetaVal::Seq(coll))
            },
            &Self::Count => {
                let len = match stack.pop_iterable_like()? {
                    // TODO: Make this work without needing to allocate a vector.
                    IterableLike::Stream(st) => st.collect::<Result<Vec<_>, _>>()?.len() as i64,
                    IterableLike::Sequence(sq) => sq.len() as i64,
                };

                Operand::Value(MetaVal::Int(len))
            },
            &Self::First => {
                let mv = stack.pop_iterable_like()?.into_iter().next().unwrap_or(Ok(MetaVal::Nil))?;
                Operand::Value(mv)
            },
            &Self::Last => {
                let mv = match stack.pop_iterable_like()? {
                    IterableLike::Stream(st) => {
                        let mut last_seen = None;
                        for res_mv in st {
                            last_seen = Some(res_mv?);
                        }

                        last_seen
                    },
                    IterableLike::Sequence(sq) => sq.into_iter().last(),
                }.unwrap_or(MetaVal::Nil);

                Operand::Value(mv)
            },
            &Self::Max => {
                let mut m: Option<NumberLike> = None;

                for mv in stack.pop_iterable_like()? {
                    let num: NumberLike = mv?.try_into()?;

                    m = Some(
                        match m {
                            None => num,
                            Some(curr_m) => curr_m.max(num),
                        }
                    );
                }

                Operand::Value(m.ok_or(Error::EmptyIterable)?.into())
            },
            &Self::Min => {
                let mut m: Option<NumberLike> = None;

                for mv in stack.pop_iterable_like()? {
                    let num: NumberLike = mv?.try_into()?;

                    m = Some(
                        match m {
                            None => num,
                            Some(curr_m) => curr_m.min(num),
                        }
                    );
                }

                Operand::Value(m.ok_or(Error::EmptyIterable)?.into())
            },
            &Self::Sum => {
                let mut total = NumberLike::Integer(0);

                for mv in stack.pop_iterable_like()? {
                    let num: NumberLike = mv?.try_into()?;
                    total += num;
                }

                Operand::Value(total.into())
            },
            &Self::Product => {
                let mut total = NumberLike::Integer(1);

                for mv in stack.pop_iterable_like()? {
                    let num: NumberLike = mv?.try_into()?;
                    total *= num;
                }

                Operand::Value(total.into())
            },
            &Self::AllEqual => {
                let mut it = stack.pop_iterable_like()?.into_iter();

                let res = match it.next() {
                    None => true,
                    Some(res_first) => {
                        let first = res_first?;
                        let mut eq_so_far = true;

                        for res_mv in it {
                            let mv = res_mv?;
                            if mv != first {
                                eq_so_far = false;
                                break;
                            }
                        }

                        eq_so_far
                    }
                };

                Operand::Value(MetaVal::Bul(res))
            },
            &Self::ParentsRef | &Self::ChildrenRef => {
                // let kp = stack.pop_key_path_like()?;

                // let mb_stream = match self {
                //     &Self::ParentsRef => FileMetaBlockStream::new(ParentFileWalker::new(rc.current_item_file_path), rc.meta_format, rc.selection, rc.sort_order),
                //     &Self::ChildrenRef => FileMetaBlockStream::new(ChildFileWalker::new(rc.current_item_file_path), rc.meta_format, rc.selection, rc.sort_order),
                //     _ => unreachable!(),
                // };

                // let stream = Stream::Raw(MetaValueStream::new(kp, mb_stream));

                // Operand::Stream(stream)
                Operand::Value(MetaVal::Nil)
            },
        };

        stack.push(output_operand);

        Ok(())
    }
}
