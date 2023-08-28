use eyre::Report;
use nextclade::analyze::pcr_primer_changes::PcrPrimer;
use nextclade::analyze::virus_properties::{PhenotypeAttrDesc, VirusProperties};
use nextclade::gene::gene_map::GeneMap;
use nextclade::io::dataset::{
  Dataset, DatasetAttributeValue, DatasetAttributes, DatasetCapabilities, DatasetCollectionMeta, DatasetCollectionUrl,
  DatasetsIndexJson,
};
use nextclade::io::fasta::FastaRecord;
use nextclade::io::file::create_file_or_stdout;
use nextclade::io::fs::ensure_dir;
use nextclade::io::json::{json_write_impl, JsonPretty};
use nextclade::io::nextclade_csv::CsvColumnConfig;
use nextclade::qc::qc_config::QcConfig;
use nextclade::qc::qc_run::QcResult;
use nextclade::run::nextclade_wasm::{
  AnalysisInitialData, AnalysisInput, NextcladeParams, NextcladeParamsRaw, NextcladeResult, OutputTrees,
};
use nextclade::translate::translate_genes::Translation;
use nextclade::tree::tree::{AuspiceTree, CladeNodeAttrKeyDesc};
use nextclade::types::outputs::{NextcladeErrorOutputs, NextcladeOutputs};
use schemars::{schema_for, JsonSchema};
use std::path::Path;

const OUTPUT_JSON_SCHEMA: &str = "src/gen/_SchemaRoot.json";

fn main() -> Result<(), Report> {
  ensure_dir(OUTPUT_JSON_SCHEMA)?;
  write_jsonschema::<_SchemaRoot>(OUTPUT_JSON_SCHEMA)?;
  Ok(())
}

/// Create JSON schema file from a given Rust struct type and write it to a specified file.
fn write_jsonschema<T: JsonSchema>(output_file: impl AsRef<Path>) -> Result<(), Report> {
  let schema = schema_for!(T);
  json_write_impl(create_file_or_stdout(output_file)?, &schema, JsonPretty(true))
}

// Dummy struct containing the types we want to expose (recursively).
//
// The doc comment below will appear in the schema file. Schema file should not be edited manually. But despite this
// comment also being here, you CAN edit this struct. Go ahead!
//
/// AUTOGENERATED! DO NOT EDIT! This JSON schema file is generated automatically from Rust types.
/// The topmost schema definition is a dummy container for the types we want to expose. Disregard
/// it. Instead, See the actual types in the `definitions` property of JSON schema.
#[derive(Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct _SchemaRoot<'a> {
  _1: GeneMap,
  _2: Translation,
  _3: AuspiceTree,
  _4: QcConfig,
  _5: QcResult,
  _6: PcrPrimer,
  _7: NextcladeOutputs,
  _9: CsvColumnConfig,
  _10: NextcladeErrorOutputs,
  _12: VirusProperties,
  _13: CladeNodeAttrKeyDesc,
  _14: PhenotypeAttrDesc,
  _15: FastaRecord,
  _17: AnalysisInitialData<'a>,
  _18: AnalysisInput,
  _19: NextcladeResult,
  _20: NextcladeParams,
  _21: NextcladeParamsRaw,
  _22: OutputTrees,
  _23: DatasetsIndexJson,
  _24: Dataset,
  _25: DatasetCollectionMeta,
  _26: DatasetCapabilities,
  _27: DatasetAttributeValue,
  _28: DatasetAttributes,
  _29: DatasetCollectionUrl,
}
