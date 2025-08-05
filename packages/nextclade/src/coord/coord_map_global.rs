use crate::alphabet::nuc::Nuc;
use crate::coord::coord_map::{make_aln_to_ref_map, make_ref_to_aln_map};
use crate::coord::position::{NucAlnGlobalPosition, NucRefGlobalPosition, PositionLike};
use crate::coord::range::{NucAlnGlobalRange, NucRefGlobalRange, Range};

use serde::{Deserialize, Serialize};

/// Converts from global alignment coordinates to global reference coordinates and vice versa.
///
/// Positions of nucleotides in the sequences change after alignment due to insertion stripping. Some operations are
/// done in alignment space, while others in reference space. This struct allows for conversion of positions and ranges
/// from one space to another.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CoordMapGlobal {
  aln_to_ref_table: Vec<NucRefGlobalPosition>,
  ref_to_aln_table: Vec<NucAlnGlobalPosition>,
  aln_to_qry_table: Vec<NucRefGlobalPosition>,
}

impl CoordMapGlobal {
  /// Takes aligned ref_seq before insertions (i.e. gaps in ref) are stripped
  pub fn new(ref_seq_unstripped: &[Nuc], qry_seq_unstripped: &[Nuc]) -> Self {
    Self {
      aln_to_ref_table: make_aln_to_ref_map(ref_seq_unstripped),
      ref_to_aln_table: make_ref_to_aln_map(ref_seq_unstripped),
      aln_to_qry_table: make_aln_to_ref_map(qry_seq_unstripped),
    }
  }

  #[inline]
  pub fn aln_to_ref_position(&self, aln: NucAlnGlobalPosition) -> NucRefGlobalPosition {
    self.aln_to_ref_table[aln.as_usize()]
  }

  #[inline]
  pub fn ref_to_aln_position(&self, reff: NucRefGlobalPosition) -> NucAlnGlobalPosition {
    self.ref_to_aln_table[reff.as_usize()]
  }

  #[inline]
  pub fn ref_to_qry_position(&self, reff: NucRefGlobalPosition) -> NucRefGlobalPosition {
    self.aln_to_qry_table[self.ref_to_aln_position(reff).as_usize()]
  }

  #[inline]
  pub fn aln_to_ref_range(&self, aln_range: &NucAlnGlobalRange) -> NucRefGlobalRange {
    if aln_range.is_empty() {
      return Range::new(
        self.aln_to_ref_position(aln_range.begin),
        self.aln_to_ref_position(aln_range.begin),
      );
    }
    Range::new(
      self.aln_to_ref_position(aln_range.begin),
      self.aln_to_ref_position(aln_range.end - 1) + 1,
    )
  }

  #[inline]
  pub fn ref_to_aln_range(&self, ref_range: &NucRefGlobalRange) -> NucAlnGlobalRange {
    if ref_range.is_empty() {
      return Range::new(
        self.ref_to_aln_position(ref_range.begin),
        self.ref_to_aln_position(ref_range.begin),
      );
    }
    Range::new(
      self.ref_to_aln_position(ref_range.begin),
      self.ref_to_aln_position(ref_range.end - 1) + 1,
    )
  }

  #[inline]
  pub fn ref_to_qry_range(&self, ref_range: &NucRefGlobalRange) -> NucRefGlobalRange {
    if ref_range.is_empty() {
      return Range::new(
        self.ref_to_qry_position(ref_range.begin),
        self.ref_to_qry_position(ref_range.begin),
      );
    }
    Range::new(
      self.ref_to_qry_position(ref_range.begin),
      self.ref_to_qry_position(ref_range.end - 1) + 1,
    )
  }
}

#[cfg(test)]
mod coord_map_tests {
  use super::*;
  use crate::alphabet::nuc::to_nuc_seq;
  use eyre::Report;
  use pretty_assertions::assert_eq;
  use rstest::rstest;

  //noinspection SpellCheckingInspection
  #[rustfmt::skip]
  #[rstest]
  fn maps_example() -> Result<(), Report> {
    // index                            012345678901234567890123456789012345678901234567890123456789
    // index                            012345678888901234567890123456789012345678901234567890123333
    let ref_aln = to_nuc_seq("TGATGCACA---ATCGTTTTTAAACGGGTTTGCGGTGTAAGTGCAGCCCGTCTTACA---")?;
    let qry_aln = to_nuc_seq("---TGATGCACAATCGTTTTTAAACGGGTTTGCGGTGTA---AGTGCAGCCCGTCTTACA")?;
    // index                            000012345678901234567890123456789012345555678901234567890123

    let global_coord_map = CoordMapGlobal::new(&ref_aln, &qry_aln);

    assert_eq!(
      global_coord_map.aln_to_ref_table,
      vec![
        0, 1, 2, 3, 4, 5, 6, 7, 8, 8, 8, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
        28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 53, 53, 53
      ]
    );

    assert_eq!(
      global_coord_map.ref_to_aln_table,
      vec![
        0, 1, 2, 3, 4, 5, 6, 7, 8, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
        33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56
      ]
    );

    assert_eq!(
      global_coord_map.aln_to_qry_table,
      vec![
        0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
        28, 29, 30, 31, 32, 33, 34, 35, 35, 35, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53
      ]
    );


    Ok(())
  }
}
