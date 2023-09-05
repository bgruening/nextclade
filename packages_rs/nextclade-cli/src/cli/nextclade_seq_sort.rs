use crate::cli::nextclade_cli::{NextcladeRunOtherParams, NextcladeSeqSortArgs};
use crate::dataset::dataset_download::download_datasets_index_json;
use crate::io::http_client::HttpClient;
use eyre::{Report, WrapErr};
use itertools::Itertools;
use log::{info, trace, LevelFilter};
use nextclade::io::fasta::{FastaReader, FastaRecord, FastaWriter};
use nextclade::make_error;
use nextclade::sort::minimizer_index::{MinimizerIndexJson, MINIMIZER_INDEX_ALGO_VERSION};
use nextclade::sort::minimizer_search::{run_minimizer_search, MinimizerSearchRecord};
use nextclade::utils::string::truncate;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;
use tinytemplate::TinyTemplate;

pub fn nextclade_seq_sort(args: &NextcladeSeqSortArgs) -> Result<(), Report> {
  check_args(args)?;

  let NextcladeSeqSortArgs {
    server,
    proxy_config,
    input_minimizer_index_json,
    ..
  } = args;

  let verbose = log::max_level() > LevelFilter::Info;

  let minimizer_index = if let Some(input_minimizer_index_json) = &input_minimizer_index_json {
    // If a file is provided, use data from it
    MinimizerIndexJson::from_path(input_minimizer_index_json)
  } else {
    // Otherwise fetch from dataset server
    let mut http = HttpClient::new(server, proxy_config, verbose)?;
    let index = download_datasets_index_json(&mut http)?;
    let minimizer_index_path = index
      .minimizer_index
      .iter()
      .find(|minimizer_index| MINIMIZER_INDEX_ALGO_VERSION == minimizer_index.version)
      .map(|minimizer_index| &minimizer_index.path);

    if let Some(minimizer_index_path) = minimizer_index_path {
      let minimizer_index_str = http.get(minimizer_index_path)?;
      MinimizerIndexJson::from_str(String::from_utf8(minimizer_index_str)?)
    } else {
      let server_versions = index
        .minimizer_index
        .iter()
        .map(|minimizer_index| format!("'{}'", minimizer_index.version))
        .join(",");
      let server_versions = if server_versions.is_empty() {
        "none".to_owned()
      } else {
        format!(": {server_versions}")
      };

      make_error!("No compatible reference minimizer index data is found for this dataset sever. Cannot proceed. \n\nThis version of Nextclade supports index versions up to '{}', but the server has{}.\n\nTry to to upgrade Nextclade to the latest version and/or contact dataset server maintainers.", MINIMIZER_INDEX_ALGO_VERSION, server_versions)
    }
  }?;

  run(args, &minimizer_index)
}

pub fn run(args: &NextcladeSeqSortArgs, minimizer_index: &MinimizerIndexJson) -> Result<(), Report> {
  let NextcladeSeqSortArgs {
    input_fastas,
    output_dir,
    output,
    search_params,
    other_params: NextcladeRunOtherParams { jobs },
    ..
  } = args;

  std::thread::scope(|s| {
    const CHANNEL_SIZE: usize = 128;
    let (fasta_sender, fasta_receiver) = crossbeam_channel::bounded::<FastaRecord>(CHANNEL_SIZE);
    let (result_sender, result_receiver) = crossbeam_channel::bounded::<MinimizerSearchRecord>(CHANNEL_SIZE);

    s.spawn(|| {
      let mut reader = FastaReader::from_paths(input_fastas).unwrap();
      loop {
        let mut record = FastaRecord::default();
        reader.read(&mut record).unwrap();
        if record.is_empty() {
          break;
        }
        fasta_sender
          .send(record)
          .wrap_err("When sending a FastaRecord")
          .unwrap();
      }
      drop(fasta_sender);
    });

    for _ in 0..*jobs {
      let fasta_receiver = fasta_receiver.clone();
      let result_sender = result_sender.clone();

      s.spawn(move || {
        let result_sender = result_sender.clone();

        for fasta_record in &fasta_receiver {
          info!("Processing sequence '{}'", fasta_record.seq_name);

          let result = run_minimizer_search(&fasta_record, minimizer_index, search_params)
            .wrap_err_with(|| {
              format!(
                "When processing sequence #{} '{}'",
                fasta_record.index, fasta_record.seq_name
              )
            })
            .unwrap();

          result_sender
            .send(MinimizerSearchRecord { fasta_record, result })
            .wrap_err("When sending minimizer record into the channel")
            .unwrap();
        }

        drop(result_sender);
      });
    }

    let writer = s.spawn(move || {
      let output_dir = &output_dir;
      let output = &output;

      let tt = output.as_ref().map(move |output| {
        let mut tt = TinyTemplate::new();
        tt.add_template("output", output)
          .wrap_err_with(|| format!("When parsing template: {output}"))
          .unwrap();
        tt
      });

      println!(
        "{:40} | {:40} | {:10} | {:10}",
        "Seq. name", "dataset", "total hits", "max hit"
      );

      let mut writers = BTreeMap::new();

      for record in result_receiver {
        if let Some(name) = &record.result.dataset {
          let filepath = match (&tt, output_dir) {
            (Some(tt), None) => {
              let filepath_str = tt
                .render("output", &OutputTemplateContext { name })
                .wrap_err("When rendering output path template")
                .unwrap();

              Some(
                PathBuf::from_str(&filepath_str)
                  .wrap_err_with(|| format!("Invalid output translations path: '{filepath_str}'"))
                  .unwrap(),
              )
            }
            (None, Some(output_dir)) => Some(output_dir.join(name).join("sequences.fasta")),
            _ => None,
          };

          if let Some(filepath) = filepath {
            let writer = writers.entry(filepath.clone()).or_insert_with(|| {
              trace!("Creating fasta writer to file {filepath:#?}");
              FastaWriter::from_path(filepath).unwrap()
            });

            writer
              .write(&record.fasta_record.seq_name, &record.fasta_record.seq, false)
              .unwrap();
          }
        }

        println!(
          "{:40} | {:40} | {:>10} | {:>.3}",
          &truncate(record.fasta_record.seq_name, 40),
          &truncate(record.result.dataset.unwrap_or_default(), 40),
          &record.result.total_hits,
          &record.result.max_normalized_hit
        );
      }
    });
  });

  Ok(())
}

#[derive(Serialize)]
struct OutputTemplateContext<'a> {
  name: &'a str,
}

fn check_args(args: &NextcladeSeqSortArgs) -> Result<(), Report> {
  let NextcladeSeqSortArgs { output_dir, output, .. } = args;

  if output.is_some() && output_dir.is_some() {
    return make_error!(
      "The arguments `--output-dir` and `--output` cannot be used together. Remove one or the other."
    );
  }

  if let Some(output) = output {
    if !output.contains("{name}") {
      return make_error!(
        r#"
Expected `--output` argument to contain a template string containing template variable {{name}} (with curly braces), but received:

  {output}

Make sure the variable is not substituted by your shell, programming language or workflow manager. Apply proper escaping as needed.
Example for bash shell:

  --output='outputs/{{name}}/sorted.fasta.gz'

      "#
      );
    }
  }

  Ok(())
}
