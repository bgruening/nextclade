use clap::{Parser, ValueEnum};
use optfield::optfield;
use serde::{Deserialize, Serialize};

#[derive(ValueEnum, Copy, Clone, Debug, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum GapAlignmentSide {
  Left,
  Right,
}

// NOTE: The `optfield` attribute creates a struct that have the same fields, but which are wrapped into `Option`,
// as well as adds a method `.merge_opt(&opt)` to the original struct, which merges values from the optional counterpart
// into self (mutably).

#[allow(clippy::struct_excessive_bools)]
#[optfield(pub AlignPairwiseParamsOptional, attrs, doc, field_attrs, field_doc, merge_fn = pub)]
#[derive(Parser, Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AlignPairwiseParams {
  /// Minimum length of nucleotide sequence to consider for alignment.
  ///
  /// If a sequence is shorter than that, alignment will not be attempted and a warning will be emitted. When adjusting this parameter, note that alignment of short sequences can be unreliable.
  #[clap(long)]
  pub min_length: usize,

  /// Penalty for extending a gap in alignment. If zero, all gaps regardless of length incur the same penalty.
  #[clap(long)]
  pub penalty_gap_extend: i32,

  /// Penalty for opening of a gap in alignment. A higher penalty results in fewer gaps and more mismatches. Should be less than `--penalty-gap-open-in-frame` to avoid gaps in genes.
  #[clap(long)]
  pub penalty_gap_open: i32,

  /// As `--penalty-gap-open`, but for opening gaps at the beginning of a codon. Should be greater than `--penalty-gap-open` and less than `--penalty-gap-open-out-of-frame`, to avoid gaps in genes, but favor gaps that align with codons.
  #[clap(long)]
  pub penalty_gap_open_in_frame: i32,

  /// As `--penalty-gap-open`, but for opening gaps in the body of a codon. Should be greater than `--penalty-gap-open-in-frame` to favor gaps that align with codons.
  #[clap(long)]
  pub penalty_gap_open_out_of_frame: i32,

  /// Penalty for aligned nucleotides or amino acids that differ in state during alignment. Note that this is redundantly parameterized with `--score-match`
  #[clap(long)]
  pub penalty_mismatch: i32,

  /// Score for matching states in nucleotide or amino acid alignments.
  #[clap(long)]
  pub score_match: i32,

  /// Maximum area of the alignment band. Alignments with large bands are slow to compute and require substantial memory in the current implementation. Alignment of sequences requiring band with area larger than this value, will not be attempted and a warning will be emitted.
  #[clap(long)]
  pub max_band_area: usize,

  /// Maximum length of insertions or deletions allowed to proceed with alignment. Alignments with long indels are slow to compute and require substantial memory in the current implementation. Alignment of sequences with indels longer that this value, will not be attempted and a warning will be emitted.
  #[clap(long)]
  pub max_indel: usize,

  /// k-mer length to determine approximate alignments between query and reference and determine the bandwidth of the banded alignment.
  #[clap(long)]
  pub seed_length: usize,

  /// Maximum number of mismatching nucleotides allowed for a seed to be considered a match.
  #[clap(long)]
  pub mismatches_allowed: usize,

  /// Minimum number of seeds to search for during nucleotide alignment. Relevant for short sequences. In long sequences, the number of seeds is determined by `--seed-spacing`.
  #[clap(long)]
  pub min_seeds: i32,

  /// Minimum seed mathing rate (a ratio of seed matches to total number of attempted seeds)
  #[clap(long)]
  pub min_match_rate: f64,

  /// Spacing between seeds during nucleotide alignment.
  #[clap(long)]
  pub seed_spacing: i32,

  /// Retry seed matching step with a reverse complement if the first attempt failed
  #[clap(long)]
  #[clap(num_args=0..=1, default_missing_value = "true")]
  pub retry_reverse_complement: bool,

  /// If this flag is present, the amino acid sequences will be truncated at the first stop codon, if mutations or sequencing errors cause premature stop codons to be present. No amino acid mutations in the truncated region will be recorded.
  #[clap(long)]
  #[clap(num_args=0..=1, default_missing_value = "true")]
  pub no_translate_past_stop: bool,

  // Internal alignment parameter
  #[clap(skip)]
  pub left_terminal_gaps_free: bool,

  // Internal alignment parameter
  #[clap(skip)]
  pub right_terminal_gaps_free: bool,

  /// Excess bandwidth for internal stripes.
  #[clap(long)]
  pub excess_bandwidth: i32,

  /// Excess bandwidth for terminal stripes.
  #[clap(long)]
  pub terminal_bandwidth: i32,

  /// Whether to align gaps on the left or right side if equally parsimonious.
  /// Left aligning gaps is the convention, right align is Nextclade's historic default
  #[clap(long, value_enum)]
  pub gap_alignment_side: GapAlignmentSide,

  /// Length of exactly matching kmers used in the seed alignment of the query to the reference.
  #[clap(long)]
  pub kmer_length: usize,

  /// Interval of successive kmers on the query sequence. Should be small compared to the query length.
  #[clap(long)]
  pub kmer_distance: usize,

  /// Exactly matching kmers are extended to the left and right until more
  /// than `allowed_mismatches` are observed in a sliding window (`window_size`).
  #[clap(long)]
  pub allowed_mismatches: usize,

  /// Size of the window within which mismatches are accumulated during seed extension
  #[clap(long)]
  pub window_size: usize,

  /// Minimum length of extended kmers
  #[clap(long)]
  pub min_match_length: usize,

  /// Fraction of the query sequence that has to be covered by extended seeds
  /// to proceed with the banded alignment.
  #[clap(long)]
  pub min_seed_cover: f64,

  /// Number of times Nextclade will retry alignment with more relaxed results if alignment band boundaries are hit
  #[clap(long)]
  pub max_alignment_attempts: usize,
}

impl Default for AlignPairwiseParams {
  fn default() -> Self {
    Self {
      min_length: 100,
      penalty_gap_extend: 0,
      penalty_gap_open: 6,
      penalty_gap_open_in_frame: 7,
      penalty_gap_open_out_of_frame: 8,
      penalty_mismatch: 1,
      score_match: 3,
      max_band_area: 500_000_000,
      max_indel: 400,        // obsolete
      seed_length: 21,       // obsolete
      min_seeds: 10,         // obsolete
      min_match_rate: 0.3,   // obsolete
      seed_spacing: 100,     // obsolete
      mismatches_allowed: 3, // obsolete
      retry_reverse_complement: false,
      no_translate_past_stop: false,
      left_terminal_gaps_free: true,
      right_terminal_gaps_free: true,
      gap_alignment_side: GapAlignmentSide::Right,
      excess_bandwidth: 9,
      terminal_bandwidth: 50,
      min_seed_cover: 0.33,
      kmer_length: 10,       // Should not be much larger than 1/divergence of amino acids
      kmer_distance: 50,     // Distance between successive kmers
      min_match_length: 40,  // Experimentally determined, to keep off-target matches reasonably low
      allowed_mismatches: 8, // Ns count as mismatches
      window_size: 30,
      max_alignment_attempts: 3,
    }
  }
}
