pub mod converter;
pub mod predicate;
pub mod iter_consumer;
pub mod iter_adaptor;

pub use self::converter::Converter;
pub use self::predicate::Predicate;
pub use self::iter_consumer::IterConsumer;
pub use self::iter_adaptor::IterAdaptor;

use crate::metadata::types::MetaVal;
use crate::functions::Error;
use crate::functions::operator::UnaryConverter;
use crate::functions::operator::UnaryPredicate;
use crate::functions::util::value_producer::ValueProducer;
use crate::functions::util::value_producer::Fixed;
use crate::functions::util::value_producer::Filter;
use crate::functions::util::value_producer::Map;
use crate::functions::util::value_producer::StepBy;
use crate::functions::util::value_producer::Chain;
use crate::functions::util::value_producer::Zip;
use crate::functions::util::value_producer::Skip;
use crate::functions::util::value_producer::Take;
use crate::functions::util::value_producer::SkipWhile;
use crate::functions::util::value_producer::TakeWhile;
use crate::functions::util::value_producer::Intersperse;
use crate::functions::util::value_producer::Interleave;

#[derive(Clone, Copy)]
enum AllAny { All, Any, }

impl AllAny {
    fn target(self) -> bool {
        match self {
            Self::All => false,
            Self::Any => true,
        }
    }
}

/// Namespace for all the implementation of various functions in this module.
pub struct Impl;

impl Impl {
    pub fn nth<'a, VP: ValueProducer<'a>>(vp: VP, n: usize) -> Result<MetaVal<'a>, Error> {
        let mut i = 0;
        for res_mv in vp {
            let mv = res_mv?;

            if i == n { return Ok(mv) }
            else { i += 1; }
        }

        Err(Error::OutOfBounds)
    }

    pub fn nth_s(seq: Vec<MetaVal>, n: usize) -> Result<MetaVal, Error> {
        seq.into_iter().nth(n).ok_or(Error::OutOfBounds)
    }

    fn all_any<'a, VP: ValueProducer<'a>>(vp: VP, u_pred: UnaryPredicate, flag: AllAny) -> Result<bool, Error> {
        let target = flag.target();
        for res_mv in vp {
            let mv = res_mv?;
            if u_pred.process(&mv)? == target { return Ok(target) }
        }

        Ok(!target)
    }

    pub fn all<'a, VP: ValueProducer<'a>>(vp: VP, u_pred: UnaryPredicate) -> Result<bool, Error> {
        Self::all_any(vp, u_pred, AllAny::All)
    }

    pub fn all_s(seq: Vec<MetaVal>, u_pred: UnaryPredicate) -> bool {
        match Self::all_any(Fixed::new(seq), u_pred, AllAny::All) {
            Err(_) => unreachable!(),
            Ok(b) => b,
        }
    }

    pub fn any<'a, VP: ValueProducer<'a>>(vp: VP, u_pred: UnaryPredicate) -> Result<bool, Error> {
        Self::all_any(vp, u_pred, AllAny::Any)
    }

    pub fn any_s(seq: Vec<MetaVal>, u_pred: UnaryPredicate) -> bool {
        match Self::all_any(Fixed::new(seq), u_pred, AllAny::Any) {
            Err(_) => unreachable!(),
            Ok(b) => b,
        }
    }

    pub fn find<'a, VP: ValueProducer<'a>>(vp: VP, u_pred: UnaryPredicate) -> Result<MetaVal<'a>, Error> {
        for res_mv in vp {
            let mv = res_mv?;
            if u_pred.process(&mv)? { return Ok(mv) }
        }

        Err(Error::ItemNotFound)
    }

    pub fn find_s(seq: Vec<MetaVal>, u_pred: UnaryPredicate) -> Result<MetaVal, Error> {
        Self::find(Fixed::new(seq), u_pred)
    }

    pub fn position<'a, VP: ValueProducer<'a>>(vp: VP, u_pred: UnaryPredicate) -> Result<usize, Error> {
        let mut i = 0;
        for res_mv in vp {
            let mv = res_mv?;
            if u_pred.process(&mv)? { return Ok(i) }
            i += 1;
        }

        Err(Error::ItemNotFound)
    }

    pub fn position_s(seq: Vec<MetaVal>, u_pred: UnaryPredicate) -> Result<usize, Error> {
        Self::position(Fixed::new(seq), u_pred)
    }

    pub fn filter<'a, VP: ValueProducer<'a>>(vp: VP, u_pred: UnaryPredicate) -> Filter<VP> {
        Filter::new(vp, u_pred)
    }

    pub fn filter_s(seq: Vec<MetaVal>, u_pred: UnaryPredicate) -> Result<Vec<MetaVal>, Error> {
        // It is possible for the predicate to fail.
        Filter::new(Fixed::new(seq), u_pred).collect()
    }

    pub fn map<'a, VP: ValueProducer<'a>>(vp: VP, u_conv: UnaryConverter) -> Map<VP> {
        Map::new(vp, u_conv)
    }

    pub fn map_s(seq: Vec<MetaVal>, u_conv: UnaryConverter) -> Result<Vec<MetaVal>, Error> {
        // It is possible for the converter to fail.
        Map::new(Fixed::new(seq), u_conv).collect()
    }

    pub fn step_by<'a, VP: ValueProducer<'a>>(vp: VP, step: usize) -> Result<StepBy<VP>, Error> {
        StepBy::new(vp, step)
    }

    pub fn step_by_s(seq: Vec<MetaVal>, step: usize) -> Result<Vec<MetaVal>, Error> {
        // It is possible for the step by producer creation to fail.
        // NOTE: The match is not needed, but it seems desirable to make explicit that the collect cannot fail.
        match StepBy::new(Fixed::new(seq), step)?.collect::<Result<Vec<MetaVal>, _>>() {
            Err(_) => unreachable!(),
            Ok(seq) => Ok(seq),
        }
    }

    pub fn chain<'a, VPA: ValueProducer<'a>, VPB: ValueProducer<'a>>(vp_a: VPA, vp_b: VPB) -> Chain<VPA, VPB> {
        Chain::new(vp_a, vp_b)
    }

    pub fn chain_s<'a>(seq_a: Vec<MetaVal<'a>>, seq_b: Vec<MetaVal<'a>>) -> Vec<MetaVal<'a>> {
        let mut seq_a = seq_a;
        seq_a.extend(seq_b);
        seq_a
    }

    pub fn zip<'a, VPA: ValueProducer<'a>, VPB: ValueProducer<'a>>(vp_a: VPA, vp_b: VPB) -> Zip<VPA, VPB> {
        Zip::new(vp_a, vp_b)
    }

    pub fn zip_s<'a>(seq_a: Vec<MetaVal<'a>>, seq_b: Vec<MetaVal<'a>>) -> Vec<MetaVal<'a>> {
        // Zipping cannot fail.
        match Zip::new(Fixed::new(seq_a), Fixed::new(seq_b)).collect::<Result<Vec<MetaVal>, _>>() {
            Err(_) => unreachable!(),
            Ok(seq) => seq,
        }
    }

    pub fn skip<'a, VP: ValueProducer<'a>>(vp: VP, n: usize) -> Skip<'a, VP> {
        Skip::new(vp, n)
    }

    pub fn skip_s(seq: Vec<MetaVal>, n: usize) -> Vec<MetaVal> {
        seq.into_iter().skip(n).collect()
    }

    pub fn take<'a, VP: ValueProducer<'a>>(vp: VP, n: usize) -> Take<'a, VP> {
        Take::new(vp, n)
    }

    pub fn take_s(seq: Vec<MetaVal>, n: usize) -> Vec<MetaVal> {
        seq.into_iter().take(n).collect()
    }

    pub fn skip_while<'a, VP: ValueProducer<'a>>(vp: VP, u_pred: UnaryPredicate) -> SkipWhile<VP> {
        SkipWhile::new(vp, u_pred)
    }

    pub fn skip_while_s(seq: Vec<MetaVal>, u_pred: UnaryPredicate) -> Result<Vec<MetaVal>, Error> {
        // It is possible for the predicate to fail.
        SkipWhile::new(Fixed::new(seq), u_pred).collect()
    }

    pub fn take_while<'a, VP: ValueProducer<'a>>(vp: VP, u_pred: UnaryPredicate) -> TakeWhile<VP> {
        TakeWhile::new(vp, u_pred)
    }

    pub fn take_while_s(seq: Vec<MetaVal>, u_pred: UnaryPredicate) -> Result<Vec<MetaVal>, Error> {
        // It is possible for the predicate to fail.
        TakeWhile::new(Fixed::new(seq), u_pred).collect()
    }

    pub fn intersperse<'a, VP: ValueProducer<'a>>(vp: VP, mv: MetaVal<'a>) -> Intersperse<'a, VP> {
        Intersperse::new(vp, mv)
    }

    pub fn intersperse_s<'a>(seq: Vec<MetaVal<'a>>, mv: MetaVal<'a>) -> Vec<MetaVal<'a>> {
        // Interspersing cannot fail.
        match Intersperse::new(Fixed::new(seq), mv).collect::<Result<Vec<MetaVal>, _>>() {
            Err(_) => unreachable!(),
            Ok(seq) => seq,
        }
    }

    pub fn interleave<'a, VPA: ValueProducer<'a>, VPB: ValueProducer<'a>>(vp_a: VPA, vp_b: VPB) -> Interleave<VPA, VPB> {
        Interleave::new(vp_a, vp_b)
    }

    pub fn interleave_s<'a>(seq_a: Vec<MetaVal<'a>>, seq_b: Vec<MetaVal<'a>>) -> Vec<MetaVal<'a>> {
        // Interleaving cannot fail.
        match Interleave::new(Fixed::new(seq_a), Fixed::new(seq_b)).collect::<Result<Vec<MetaVal>, _>>() {
            Err(_) => unreachable!(),
            Ok(seq) => seq,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Impl;

    use crate::test_util::TestUtil as TU;

    use crate::metadata::types::MetaVal;
    use crate::functions::Error;
    use crate::functions::ErrorKind;
    use crate::functions::util::value_producer::Raw;
    use crate::functions::util::NumberLike;

    #[test]
    fn test_nth() {
        let inputs_and_expected = vec![
            (
                (vec![], 1usize),
                Err(ErrorKind::OutOfBounds),
            ),
            (
                (TU::core_nested_sequence().into_iter().map(Result::Ok).collect(), 0),
                Ok(TU::sample_string()),
            ),
            (
                (TU::core_nested_sequence().into_iter().map(Result::Ok).collect(), 100),
                Err(ErrorKind::OutOfBounds),
            ),
            (
                (vec![Ok(MetaVal::Bul(true)), Ok(MetaVal::Bul(true)), Err(Error::Sentinel)], 1),
                Ok(MetaVal::Bul(true)),
            ),
            (
                (vec![Err(Error::Sentinel), Ok(MetaVal::Bul(true)), Ok(MetaVal::Bul(true))], 1),
                Err(ErrorKind::Sentinel),
            ),
        ];

        for (inputs, expected) in inputs_and_expected {
            let (input_a, input_b) = inputs;
            let produced = Impl::nth(Raw::new(input_a), input_b).map_err(Into::<ErrorKind>::into);
            assert_eq!(expected, produced);
        }
    }

    #[test]
    fn test_nth_s() {
        let inputs_and_expected = vec![
            (
                (vec![], 1usize),
                Err(ErrorKind::OutOfBounds),
            ),
            (
                (TU::core_nested_sequence(), 0),
                Ok(TU::sample_string()),
            ),
            (
                (TU::core_nested_sequence(), 100),
                Err(ErrorKind::OutOfBounds),
            ),
            (
                (vec![MetaVal::Bul(true), MetaVal::Bul(true)], 1),
                Ok(MetaVal::Bul(true)),
            ),
        ];

        for (inputs, expected) in inputs_and_expected {
            let (input_a, input_b) = inputs;
            let produced = Impl::nth_s(input_a, input_b).map_err(Into::<ErrorKind>::into);
            assert_eq!(expected, produced);
        }
    }
}
