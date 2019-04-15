use std::convert::TryInto;

use crate::metadata::types::MetaVal;
use crate::metadata::resolver::Error;
use crate::metadata::resolver::ops::Op;
use crate::metadata::resolver::ops::Operand;
use crate::metadata::resolver::ops::OperandStack;
use crate::metadata::resolver::context::ResolverContext;

use crate::metadata::resolver::number_like::NumberLike;
use crate::metadata::resolver::iterable_like::IterableLike;

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
    MaxIn,
    // (Iterable<Number>) -> Number
    MinIn,
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
}

impl Op for UnaryOp {
    fn process<'bo>(&self, stack: &mut OperandStack<'bo>) -> Result<(), Error> {
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
            &Self::MaxIn => {
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
            &Self::MinIn => {
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
        };

        stack.push(output_operand);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::UnaryOp;

    use bigdecimal::BigDecimal;

    use crate::metadata::resolver::ops::Op;
    use crate::metadata::resolver::ops::Operand;
    use crate::metadata::resolver::ops::OperandStack;
    use crate::metadata::resolver::streams::Stream;

    use crate::metadata::types::MetaVal;
    use crate::metadata::stream::value::MetaValueStream;

    use crate::test_util::TestUtil;

    fn stackify_vs<'a, VS>(vs: VS) -> OperandStack<'a>
    where
        VS: Into<MetaValueStream<'a>>,
    {
        let mut stack = OperandStack::new();
        stack.push(Operand::Stream(Stream::Raw(vs.into())));
        stack
    }

    #[test]
    fn test_process() {
        let op = UnaryOp::Collect;
        let mut stack = stackify_vs(TestUtil::create_sample_fixed_value_string_stream());

        op.process(&mut stack).expect("process failed");

        assert_eq!(1, stack.len());
        match stack.pop().expect("stack is empty") {
            Operand::Value(MetaVal::Seq(seq)) => {
                assert_eq!(
                    vec![
                        MetaVal::from("string_0"),
                        MetaVal::from("string_1"),
                        MetaVal::from("string_2"),
                        MetaVal::from("string_3"),
                        MetaVal::from("string_4"),
                    ],
                    seq
                );
            },
            _ => { panic!("unexpected operand"); },
        }

        let op = UnaryOp::Rev;
        let mut stack = stackify_vs(TestUtil::create_sample_fixed_value_string_stream());

        op.process(&mut stack).expect("process failed");

        assert_eq!(1, stack.len());
        match stack.pop().expect("stack is empty") {
            Operand::Value(MetaVal::Seq(seq)) => {
                assert_eq!(
                    vec![
                        MetaVal::from("string_4"),
                        MetaVal::from("string_3"),
                        MetaVal::from("string_2"),
                        MetaVal::from("string_1"),
                        MetaVal::from("string_0"),
                    ],
                    seq
                );
            },
            _ => { panic!("unexpected operand"); },
        }

        let op = UnaryOp::Count;
        let mut stack = stackify_vs(TestUtil::create_sample_fixed_value_string_stream());

        op.process(&mut stack).expect("process failed");

        assert_eq!(1, stack.len());
        match stack.pop().expect("stack is empty") {
            Operand::Value(MetaVal::Int(i)) => { assert_eq!(5, i); },
            _ => { panic!("unexpected operand"); },
        }

        let op = UnaryOp::First;
        let mut stack = stackify_vs(TestUtil::create_sample_fixed_value_string_stream());

        op.process(&mut stack).expect("process failed");

        assert_eq!(1, stack.len());
        match stack.pop().expect("stack is empty") {
            Operand::Value(mv) => { assert_eq!(MetaVal::from("string_0"), mv); },
            _ => { panic!("unexpected operand"); },
        }

        let op = UnaryOp::Last;
        let mut stack = stackify_vs(TestUtil::create_sample_fixed_value_string_stream());

        op.process(&mut stack).expect("process failed");

        assert_eq!(1, stack.len());
        match stack.pop().expect("stack is empty") {
            Operand::Value(mv) => { assert_eq!(MetaVal::from("string_4"), mv); },
            _ => { panic!("unexpected operand"); },
        }

        let op = UnaryOp::MaxIn;
        let mut stack = stackify_vs(TestUtil::create_sample_fixed_value_numbers_i_stream());

        op.process(&mut stack).expect("process failed");

        assert_eq!(1, stack.len());
        match stack.pop().expect("stack is empty") {
            Operand::Value(mv) => { assert_eq!(MetaVal::Int(9), mv); },
            _ => { panic!("unexpected operand"); },
        }

        let mut stack = stackify_vs(TestUtil::create_sample_fixed_value_numbers_d_stream());

        op.process(&mut stack).expect("process failed");

        assert_eq!(1, stack.len());
        match stack.pop().expect("stack is empty") {
            Operand::Value(mv) => { assert_eq!(MetaVal::Dec(BigDecimal::new(31415.into(), 4)), mv); },
            _ => { panic!("unexpected operand"); },
        }

        let op = UnaryOp::MinIn;
        let mut stack = stackify_vs(TestUtil::create_sample_fixed_value_numbers_i_stream());

        op.process(&mut stack).expect("process failed");

        assert_eq!(1, stack.len());
        match stack.pop().expect("stack is empty") {
            Operand::Value(mv) => { assert_eq!(MetaVal::Int(-9), mv); },
            _ => { panic!("unexpected operand"); },
        }

        let mut stack = stackify_vs(TestUtil::create_sample_fixed_value_numbers_d_stream());

        op.process(&mut stack).expect("process failed");

        assert_eq!(1, stack.len());
        match stack.pop().expect("stack is empty") {
            Operand::Value(mv) => { assert_eq!(MetaVal::Dec(BigDecimal::new((-27182).into(), 4)), mv); },
            _ => { panic!("unexpected operand"); },
        }
    }
}
