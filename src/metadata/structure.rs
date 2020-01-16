use crate::metadata::block::Block;
use crate::metadata::block::BlockSequence;
use crate::metadata::block::BlockMapping;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum UnitMetaStructureRepr {
    One(Block),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum ManyMetaStructureRepr {
    Seq(BlockSequence),
    Map(BlockMapping),
}

/// An easy-to-deserialize flavor of a meta structure.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum MetaStructureRepr {
    Unit(UnitMetaStructureRepr),
    Many(ManyMetaStructureRepr),
}

/// A data structure-level representation of all metadata structures.
/// This is intended to be agnostic to the text-level format of the metadata.
#[derive(Debug, Clone)]
pub enum MetaStructure {
    One(Block),
    Seq(BlockSequence),
    Map(BlockMapping),
}

impl From<MetaStructureRepr> for MetaStructure {
    fn from(msr: MetaStructureRepr) -> Self {
        match msr {
            MetaStructureRepr::Unit(UnitMetaStructureRepr::One(mb)) => Self::One(mb),
            MetaStructureRepr::Many(ManyMetaStructureRepr::Seq(mb_seq)) => Self::Seq(mb_seq),
            MetaStructureRepr::Many(ManyMetaStructureRepr::Map(mb_map)) => Self::Map(mb_map),
        }
    }
}
