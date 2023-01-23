use crate::gene::cds::{Cds, CdsSegment};
use crate::gene::gene::{Gene, GeneStrand};
use crate::io::letter::Letter;
use crate::io::nuc::Nuc;
use crate::translate::complement::reverse_complement_in_place;
use crate::utils::range::Range;
use eyre::Report;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::ops::Range as StdRange;

/// Makes the "alignment to reference" coordinate map: from alignment coordinates to reference coordinates.
/// Given a position of a letter in the aligned sequence, the "alignment to reference" coordinate map allows to
/// lookup the position of the corresponding letter in the reference sequence.
fn make_aln_to_ref_map(ref_seq: &[Nuc]) -> Vec<usize> {
  let mut rev_coord_map = Vec::<usize>::with_capacity(ref_seq.len());
  let mut ref_pos = 0;

  for nuc in ref_seq {
    if nuc.is_gap() {
      if rev_coord_map.is_empty() {
        rev_coord_map.push(0);
      } else {
        let prev = *(rev_coord_map.last().unwrap());
        rev_coord_map.push(prev);
      }
    } else {
      rev_coord_map.push(ref_pos);
      ref_pos += 1;
    }
  }

  rev_coord_map.shrink_to_fit();
  rev_coord_map
}

/// Makes the "reference to alignment" coordinate map: from reference coordinates to alignment coordinates.
/// Given a position of a letter in the reference sequence, the "reference to alignment" coordinate map allows to
/// lookup the position of the corresponding letter in the aligned sequence.
///
fn make_ref_to_aln_map(ref_seq: &[Nuc]) -> Vec<usize> {
  let mut coord_map = Vec::<usize>::with_capacity(ref_seq.len());

  for (i, nuc) in ref_seq.iter().enumerate() {
    if !nuc.is_gap() {
      coord_map.push(i);
    }
  }

  coord_map.shrink_to_fit();
  coord_map
}

/// Converts sequence alignment to reference coordinates and vice versa.
///
/// Positions of nucleotides in the sequences change after alignment due to insertion stripping. Some operations are
/// done in alignment space, while others in reference space. This struct allows for conversion of position indices
/// from one space to another.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordMap {
  aln_to_ref_table: Vec<usize>,
  ref_to_aln_table: Vec<usize>,
}

impl CoordMap {
  /// Takes aligned ref_seq before insertions (i.e. gaps in ref) are stripped
  pub fn new(ref_seq: &[Nuc]) -> Self {
    Self {
      aln_to_ref_table: make_aln_to_ref_map(ref_seq),
      ref_to_aln_table: make_ref_to_aln_map(ref_seq),
    }
  }

  pub fn aln_to_ref_position(&self, aln: usize) -> usize {
    self.aln_to_ref_table[aln]
  }

  // Reff is used because `ref` is magic word in Rust
  pub fn ref_to_aln_position(&self, reff: usize) -> usize {
    self.ref_to_aln_table[reff]
  }

  /// Converts relative position inside an aligned feature (e.g. gene) to absolute position in the reference
  pub fn feature_aln_to_ref_position(&self, feature: &Gene, aln_pos_rel: usize) -> usize {
    let aln_pos = if feature.strand == GeneStrand::Reverse {
      self.ref_to_aln_position(feature.end - 1) - aln_pos_rel //feature.end points to the nuc after the feature, hence - 1
    } else {
      self.ref_to_aln_position(feature.start) + aln_pos_rel
    };
    self.aln_to_ref_position(aln_pos)
  }

  /// Converts relative position inside a feature (e.g. gene) to absolute position in the alignment
  pub fn feature_ref_to_aln_position(&self, feature: &Gene, ref_pos_rel: usize) -> usize {
    let ref_pos = if feature.strand == GeneStrand::Reverse {
      feature.end - 1 - ref_pos_rel // the feature end is one past the last character, hence -1
    } else {
      feature.start + ref_pos_rel
    };
    self.ref_to_aln_position(ref_pos)
  }

  pub fn aln_to_ref_range(&self, aln_range: &Range) -> Range {
    Range {
      begin: self.aln_to_ref_table[aln_range.begin],
      end: self.aln_to_ref_table[aln_range.end - 1] + 1,
    }
  }

  pub fn ref_to_aln_range(&self, ref_range: &Range) -> Range {
    Range {
      begin: self.ref_to_aln_table[ref_range.begin],
      end: self.ref_to_aln_table[ref_range.end - 1] + 1,
    }
  }

  pub fn feature_aln_to_ref_range(&self, feature: &Gene, aln_range: &Range) -> Range {
    if feature.strand == GeneStrand::Reverse {
      Range {
        begin: self.feature_aln_to_ref_position(feature, aln_range.end - 1),
        end: self.feature_aln_to_ref_position(feature, aln_range.begin) + 1,
      }
    } else {
      Range {
        begin: self.feature_aln_to_ref_position(feature, aln_range.begin),
        end: self.feature_aln_to_ref_position(feature, aln_range.end - 1) + 1,
      }
    }
  }

  pub fn feature_ref_to_aln_range(&self, feature: &Gene, ref_range: &Range) -> Range {
    Range {
      begin: self.feature_ref_to_aln_position(feature, ref_range.begin),
      end: self.feature_ref_to_aln_position(feature, ref_range.end - 1) + 1,
    }
  }

  pub fn feature_aln_to_feature_ref_position(&self, feature: &Gene, aln_position: usize) -> usize {
    if feature.strand == GeneStrand::Reverse {
      feature.end - 1 - self.feature_aln_to_ref_position(feature, aln_position)
    } else {
      self.feature_aln_to_ref_position(feature, aln_position) - feature.start
    }
  }

  pub fn feature_aln_to_feature_ref_range(&self, feature: &Gene, aln_range: &Range) -> Range {
    Range {
      begin: self.feature_aln_to_feature_ref_position(feature, aln_range.begin),
      end: self.feature_aln_to_feature_ref_position(feature, aln_range.end - 1) + 1,
    }
  }

  /// Extracts nucleotide sequence of a gene
  pub fn extract_gene(&self, full_aln_seq: &[Nuc], gene: &Gene) -> Vec<Nuc> {
    gene
      .cdses
      .iter()
      .flat_map(|cds| self.extract_cds(full_aln_seq, cds))
      .collect_vec()
  }

  /// Extracts nucleotide sequence of a CDS
  pub fn extract_cds(&self, full_aln_seq: &[Nuc], cds: &Cds) -> Vec<Nuc> {
    cds
      .segments
      .iter()
      .flat_map(|cds_segment| self.extract_cds_segment(full_aln_seq, cds_segment))
      .collect_vec()
  }

  /// Extracts nucleotide sequence of a CDS segment
  pub fn extract_cds_segment(&self, full_aln_seq: &[Nuc], cds_segment: &CdsSegment) -> Vec<Nuc> {
    // Genemap contains ranges in reference coordinates (like in ref sequence)
    let range_ref = Range {
      begin: cds_segment.start,
      end: cds_segment.end,
    };

    // ...but we are extracting from aligned sequence, so we need to convert it to alignment coordinates (like in aligned sequences)
    let range_aln = self.ref_to_aln_range(&range_ref);
    let mut nucs = full_aln_seq[StdRange::from(range_aln)].to_vec();

    // Reverse strands should be reverse-complemented
    if cds_segment.strand == GeneStrand::Reverse {
      reverse_complement_in_place(&mut nucs);
    }

    nucs
  }
}

fn extract_cds_sequence(seq: &[Nuc], cds: &Cds) -> Vec<Nuc> {
  cds
    .segments
    .iter()
    .flat_map(|cds_segment| seq[cds_segment.start..cds_segment.end].iter().copied())
    .collect_vec()
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CdsToAln {
  global: Vec<usize>,
  start: usize,
  len: usize,
}

fn extract_cds_alignment(seq_aln: &[Nuc], cds: &Cds, coord_map: &CoordMap) -> (Vec<Nuc>, Vec<CdsToAln>) {
  let mut cds_aln = vec![];
  let mut cds_to_aln = vec![];
  for segment in &cds.segments {
    let start = coord_map.ref_to_aln_position(segment.start);
    let end = coord_map.ref_to_aln_position(segment.end);
    cds_to_aln.push(CdsToAln {
      global: (start..end).collect_vec(),
      start: cds_aln.len(),
      len: end - start,
    });
    cds_aln.extend_from_slice(&seq_aln[start..end]);
  }
  (cds_aln, cds_to_aln)
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum CdsPosition {
  Before,
  Inside(usize),
  After,
}

/// Map a position in the extracted alignment of the CDS to the global alignment.
/// Returns a result for each CDS segment, but a single position can  only be in one CDS segment.
fn cds_to_global_aln_position(pos: usize, cds_to_aln_map: &[CdsToAln]) -> Vec<CdsPosition> {
  cds_to_aln_map
    .iter()
    .map(|segment| {
      let pos_in_segment = pos as isize - segment.start as isize;

      if pos_in_segment < 0 {
        CdsPosition::Before
      } else if pos_in_segment >= segment.len as isize {
        CdsPosition::After
      } else {
        CdsPosition::Inside(segment.global[pos_in_segment as usize])
      }
    })
    .collect_vec()
}

#[cfg(test)]
mod coord_map_tests {
  use super::*;
  use crate::io::nuc::to_nuc_seq;
  use eyre::Report;
  use multimap::MultiMap;
  use pretty_assertions::assert_eq;
  use rstest::rstest;

  fn create_fake_cds(segment_ranges: &[(usize, usize)]) -> Cds {
    Cds {
      id: "".to_owned(),
      name: "".to_owned(),
      segments: segment_ranges
        .iter()
        .map(|(start, end)| CdsSegment {
          index: 0,
          id: "".to_owned(),
          name: "".to_owned(),
          start: *start,
          end: *end,
          strand: GeneStrand::Forward,
          frame: 0,
          exceptions: vec![],
          attributes: MultiMap::default(),
          source_record: None,
          compat_is_gene: false,
        })
        .collect_vec(),
      proteins: vec![],
      compat_is_gene: false,
    }
  }

  #[rustfmt::skip]
  #[rstest]
  fn extracts_cds_sequence() -> Result<(), Report> {
    // CDS range                   11111111111111111
    // CDS range                                   2222222222222222222      333333
    // index                   012345678901234567890123456789012345678901234567890123
    let seq =      to_nuc_seq("TGATGCACAATCGTTTTTAAACGGGTTTGCGGTGTAAGTGCAGCCCGTCTTACA")?;
    let expected = to_nuc_seq(    "GCACAATCGTTTTTAAAACGGGTTTGCGGTGTAAGTCGTCTT")?;
    let cds = create_fake_cds(&[(4, 21), (20, 39), (45, 51)]);
    let actual = extract_cds_sequence(&seq, &cds);
    assert_eq!(actual, expected);
    Ok(())
  }

  #[rustfmt::skip]
  #[rstest]
  fn extracts_cds_alignment() -> Result<(), Report> {
    // CDS range                  11111111111111111
    // CDS range                                  2222222222222222222      333333
    // index                  012345678901234567890123456789012345678901234567890123456
    let reff =    to_nuc_seq("TGATGCACAATCGTTTTTAAACGGGTTTGCGGTGTAAGTGCAGCCCGTCTTACA")?;
    let ref_aln = to_nuc_seq("TGATGCACA---ATCGTTTTTAAACGGGTTTGCGGTGTAAGTGCAGCCCGTCTTACA")?;
    let qry_aln = to_nuc_seq("-GATGCACACGCATC---TTTAAACGGGTTTGCGGTGTCAGT---GCCCGTCTTACA")?;

    let cds = create_fake_cds(&[(4, 21), (20, 39), (45, 51)]);
    let global_coord_map = CoordMap::new(&ref_aln);

    let expected = to_nuc_seq("GCACAATCGTTTTTAAAACGGGTTTGCGGTGTAAGTCGTCTT")?;
    let (ref_cds_aln, ref_cds_to_aln) = extract_cds_alignment(&ref_aln, &cds, &global_coord_map);
    assert_eq!(
      ref_cds_aln,
      to_nuc_seq("GCACA---ATCGTTTTTAAAACGGGTTTGCGGTGTAAGTCGTCTT")?
    );
    assert_eq!(
      ref_cds_to_aln,
      vec![
        CdsToAln {
          global: vec![4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23],
          start: 0,
          len: 20,
        },
        CdsToAln {
          global: vec![23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41],
          start: 20,
          len: 19,
        },
        CdsToAln {
          global: vec![48, 49, 50, 51, 52, 53],
          start: 39,
          len: 6,
        },
      ]
    );

    let (qry_cds_aln, qry_cds_to_aln) = extract_cds_alignment(&qry_aln, &cds, &global_coord_map);
    assert_eq!(
      qry_cds_aln,
      to_nuc_seq("GCACACGCATC---TTTAAAACGGGTTTGCGGTGTCAGTCGTCTT")?
    );
    assert_eq!(
      qry_cds_to_aln,
      vec![
        CdsToAln {
          global: vec![4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23],
          start: 0,
          len: 20,
        },
        CdsToAln {
          global: vec![23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41],
          start: 20,
          len: 19,
        },
        CdsToAln {
          global: vec![48, 49, 50, 51, 52, 53],
          start: 39,
          len: 6,
        },
      ]
    );

    Ok(())
  }

  #[rustfmt::skip]
  #[rstest]
  fn maps_example() -> Result<(), Report> {
    let reff =    to_nuc_seq("TGATGCACAATCGTTTTTAAACGGGTTTGCGGTGTAAGTGCAGCCCGTCTTACA")?;
    let ref_aln = to_nuc_seq("TGATGCACA---ATCGTTTTTAAACGGGTTTGCGGTGTAAGTGCAGCCCGTCTTACA")?;
    let qry_aln = to_nuc_seq("-GATGCACACGCATC---TTTAAACGGGTTTGCGGTGTCAGT---GCCCGTCTTACA")?;

    let cds = create_fake_cds(&[(4, 21), (20, 39), (45, 51)]);
    let global_coord_map = CoordMap::new(&ref_aln);

    assert_eq!(
      global_coord_map.aln_to_ref_table,
      vec![
        0, 1, 2, 3, 4, 5, 6, 7, 8, 8, 8, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
        28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53
      ]
    );

    assert_eq!(
      global_coord_map.ref_to_aln_table,
      vec![
        0, 1, 2, 3, 4, 5, 6, 7, 8, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
        33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56
      ]
    );

    Ok(())
  }

  // #[rstest]
  // fn maps_ref_to_aln_simple() -> Result<(), Report> {
  //   // ref pos: 0  1  2  3  4  5  6  7  8  9  10 11 12 13 14
  //   // ref    : A  C  T  C  -  -  -  C  G  T  G  -  -  -  A
  //   // aln pos: 0  1  2  3           7  8  9  10          14
  //   let coord_map = CoordMap::new(&to_nuc_seq("ACTC---CGTG---A")?);
  //   assert_eq!(coord_map.ref_to_aln_table, vec![0, 1, 2, 3, 7, 8, 9, 10, 14]);
  //   Ok(())
  // }
  //
  // #[rstest]
  // fn maps_ref_to_aln_with_leading_insertions() -> Result<(), Report> {
  //   // ref pos:  0  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15 16
  //   // ref    :  -  -  A  C  T  C  -  -  -  C  G  T  G  -  -  -  A
  //   // aln pos:  -  -  2  3  4  5           9  10 11 12          16
  //   let coord_map = CoordMap::new(&to_nuc_seq("--ACTC---CGTG---A")?);
  //   assert_eq!(coord_map.ref_to_aln_table, vec![2, 3, 4, 5, 9, 10, 11, 12, 16]);
  //   Ok(())
  // }
  //
  // #[rstest]
  // fn maps_aln_to_ref_simple() -> Result<(), Report> {
  //   // ref pos: 0  1  2  3  4  5  6  7  8  9  10 11 12 13 14
  //   // ref    : A  C  T  C  -  -  -  C  G  T  G  -  -  -  A
  //   // aln pos: 0  1  2  3  3  3  3  4  5  6  7  7  7  7  8
  //   let coord_map = CoordMap::new(&to_nuc_seq("ACTC---CGTG---A")?);
  //   assert_eq!(
  //     coord_map.aln_to_ref_table,
  //     vec![0, 1, 2, 3, 3, 3, 3, 4, 5, 6, 7, 7, 7, 7, 8]
  //   );
  //   Ok(())
  // }
  //
  // #[rstest]
  // fn maps_aln_to_ref_with_leading_insertions() -> Result<(), Report> {
  //   // ref pos: 0  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15 16
  //   // ref    : -  -  A  C  T  C  -  -  -  C  G  T  G  -  -  -  A
  //   // aln pos: 0  0  0  1  2  3  3  3  3  4  5  6  7  7  7  7  8
  //   let coord_map = CoordMap::new(&to_nuc_seq("--ACTC---CGTG---A")?);
  //   assert_eq!(
  //     coord_map.aln_to_ref_table,
  //     vec![0, 0, 0, 1, 2, 3, 3, 3, 3, 4, 5, 6, 7, 7, 7, 7, 8]
  //   );
  //   Ok(())
  // }
  //
  // #[rstest]
  // fn maps_range_ref_to_aln_simple() -> Result<(), Report> {
  //   let coord_map = CoordMap::new(&to_nuc_seq("ACTC---CGTG---A")?);
  //   assert_eq!(
  //     coord_map.ref_to_aln_range(&Range { begin: 3, end: 6 }),
  //     Range { begin: 3, end: 9 }
  //   );
  //   Ok(())
  // }
  //
  // #[rstest]
  // fn maps_range_aln_to_ref_simple() -> Result<(), Report> {
  //   let coord_map = CoordMap::new(&to_nuc_seq("ACTC---CGTG---A")?);
  //   assert_eq!(
  //     coord_map.aln_to_ref_range(&Range { begin: 3, end: 9 }),
  //     Range { begin: 3, end: 6 }
  //   );
  //   Ok(())
  // }
  //
  // #[rstest]
  // fn maps_range_ref_to_aln_with_leading_insertions() -> Result<(), Report> {
  //   let coord_map = CoordMap::new(&to_nuc_seq("--ACTC---CGTG---A")?);
  //   assert_eq!(
  //     coord_map.ref_to_aln_range(&Range { begin: 3, end: 6 }),
  //     Range { begin: 5, end: 11 }
  //   );
  //   Ok(())
  // }
  //
  // #[rstest]
  // fn maps_range_aln_to_ref_with_leading_insertions() -> Result<(), Report> {
  //   let coord_map = CoordMap::new(&to_nuc_seq("--ACTC---CGTG---A")?);
  //   assert_eq!(
  //     coord_map.aln_to_ref_range(&Range { begin: 5, end: 11 }),
  //     Range { begin: 3, end: 6 }
  //   );
  //   Ok(())
  // }

  // #[rstest]
  // fn extract_gene_plus_strand() -> Result<(), Report> {
  //   let gene = Gene {
  //     gene_name: "g1".to_owned(),
  //     start: 3,
  //     end: 12,
  //     strand: GeneStrand::Forward,
  //     frame: 0,
  //     cdses: vec![],
  //     attributes: multimap!(),
  //   };
  //   // reference: ACT|CCGTGACCG|CGT
  //   // ref_aln: A--CT|CCGT---GACCG|--CGT
  //   //                5           |17
  //   // qry_aln: ACGCT|CCGTGCGG--CG|TGCGT
  //
  //   let coord_map = CoordMap::new(&to_nuc_seq("A--CTCCGT---GACCG--CGT")?);
  //   assert_eq!(
  //     from_nuc_seq(&coord_map.extract_gene(&to_nuc_seq("ACGCTCCGTGCGG--CGTGCGT")?, &gene)),
  //     "CCGTGCGG--CG"
  //   );
  //   Ok(())
  // }
  //
  // #[rstest]
  // fn extract_gene_minus_strand() -> Result<(), Report> {
  //   let gene = Gene {
  //     gene_name: "g1".to_owned(),
  //     start: 3,
  //     end: 12,
  //     strand: GeneStrand::Reverse,
  //     frame: 0,
  //     cdses: vec![],
  //     attributes: multimap!(),
  //   };
  //   // reference: ACT|CCGTGACCG|CGT
  //   // ref_aln: A--CT|CCGT---GACCG|--CGT
  //   //                5           |17
  //   // qry_aln: ACGCT|CCGTGCGG--CG|TGCGT
  //   // rev comp       CG--CCGCACGG
  //
  //   let coord_map = CoordMap::new(&to_nuc_seq("A--CTCCGT---GACCG--CGT")?);
  //   assert_eq!(
  //     from_nuc_seq(&coord_map.extract_gene(&to_nuc_seq("ACGCTCCGTGCGG--CGTGCGT")?, &gene)),
  //     "CG--CCGCACGG"
  //   );
  //   Ok(())
  // }
  //
  // #[rstest]
  // fn ref_feature_pos_to_aln_fwd() -> Result<(), Report> {
  //   let gene = Gene {
  //     gene_name: "g1".to_owned(),
  //     start: 3,
  //     end: 12,
  //     strand: GeneStrand::Forward,
  //     frame: 0,
  //     cdses: vec![],
  //     attributes: multimap!(),
  //   };
  //   //                0..    |7
  //   // reference: ACT|CCGTGACCG|CGT
  //   // ref_aln: A--CT|CCGT---GACCG|--CGT
  //   //                5         |15
  //
  //   let coord_map = CoordMap::new(&to_nuc_seq("A--CTCCGT---GACCG--CGT")?);
  //   assert_eq!(coord_map.feature_ref_to_aln_position(&gene, 7), 15);
  //   Ok(())
  // }
  //
  // #[rstest]
  // fn ref_feature_pos_to_aln_rev() -> Result<(), Report> {
  //   let gene = Gene {
  //     gene_name: "g1".to_owned(),
  //     start: 3,
  //     end: 12,
  //     strand: GeneStrand::Reverse,
  //     frame: 0,
  //     cdses: vec![],
  //     attributes: multimap!(),
  //   };
  //   //                 |7      |0
  //   // reference: ACT|CCGTGACCG|CGT
  //   // ref_aln: A--CT|CCGT---GACCG|--CGT
  //   //                 |6
  //
  //   let coord_map = CoordMap::new(&to_nuc_seq("A--CTCCGT---GACCG--CGT")?);
  //   assert_eq!(coord_map.feature_ref_to_aln_position(&gene, 7), 6);
  //   Ok(())
  // }
  //
  // #[rstest]
  // fn aln_feature_pos_to_ref_fwd() -> Result<(), Report> {
  //   let gene = Gene {
  //     gene_name: "g1".to_owned(),
  //     start: 3,
  //     end: 12,
  //     strand: GeneStrand::Forward,
  //     frame: 0,
  //     cdses: vec![],
  //     attributes: multimap!(),
  //   };
  //   //               |3    |8
  //   // reference: ACT|CCGTGACCG|CGT
  //   // ref_aln: A--CT|CCGT---GACCG|--CGT
  //   //               |        |8
  //
  //   let coord_map = CoordMap::new(&to_nuc_seq("A--CTCCGT---GACCG--CGT")?);
  //   assert_eq!(coord_map.feature_aln_to_ref_position(&gene, 8), 8);
  //   Ok(())
  // }
  //
  // #[rstest]
  // fn aln_feature_pos_to_ref_rev() -> Result<(), Report> {
  //   let gene = Gene {
  //     gene_name: "g1".to_owned(),
  //     start: 3,
  //     end: 12,
  //     strand: GeneStrand::Reverse,
  //     frame: 0,
  //     cdses: vec![],
  //     attributes: multimap!(),
  //   };
  //   //               |3 |5
  //   // reference: ACT|CCGTGACCG|CGT
  //   // ref_aln: A--CT|CCGT---GACCG|--CGT
  //   //               |  |9       |0
  //
  //   let coord_map = CoordMap::new(&to_nuc_seq("A--CTCCGT---GACCG--CGT")?);
  //   assert_eq!(coord_map.feature_aln_to_ref_position(&gene, 9), 5);
  //   Ok(())
  // }
  //
  // #[rstest]
  // fn aln_feature_range_to_ref_fwd() -> Result<(), Report> {
  //   let gene = Gene {
  //     gene_name: "g1".to_owned(),
  //     start: 3,
  //     end: 12,
  //     strand: GeneStrand::Forward,
  //     frame: 0,
  //     cdses: vec![],
  //     attributes: multimap!(),
  //   };
  //   //               |   |3 |6
  //   // reference: ACT|CCGTGACCG|CGT
  //   // ref_aln: A--CT|CCGT---GACCG|--CGT
  //   //               |   |     |9
  //
  //   let coord_map = CoordMap::new(&to_nuc_seq("A--CTCCGT---GACCG--CGT")?);
  //   assert_eq!(
  //     coord_map.feature_aln_to_ref_range(&gene, &Range { begin: 3, end: 9 }),
  //     Range { begin: 6, end: 9 }
  //   );
  //   Ok(())
  // }
  //
  // #[rstest]
  // fn aln_feature_range_to_ref_rev() -> Result<(), Report> {
  //   let gene = Gene {
  //     gene_name: "g1".to_owned(),
  //     start: 3,
  //     end: 12,
  //     strand: GeneStrand::Reverse,
  //     frame: 0,
  //     cdses: vec![],
  //     attributes: multimap!(),
  //   };
  //   //               |   |6 |9
  //   // reference: ACT|CCGTGACCG|CGT
  //   // ref_aln: A--CT|CCGT---GACCG|--CGT
  //   //               |  9|   3|
  //
  //   let coord_map = CoordMap::new(&to_nuc_seq("A--CTCCGT---GACCG--CGT")?);
  //   assert_eq!(
  //     coord_map.feature_aln_to_ref_range(&gene, &Range { begin: 3, end: 9 }),
  //     Range { begin: 6, end: 9 }
  //   );
  //   Ok(())
  // }
}
