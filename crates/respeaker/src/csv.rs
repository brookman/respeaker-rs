use std::{collections::HashMap, fs::File, path::PathBuf};

use csv::Writer;
use tracing::info;

use crate::params::{ParamKind, Value};

pub fn write_csv(
    data: Vec<(f32, HashMap<ParamKind, Value>)>,
    file_path: &PathBuf,
) -> eyre::Result<()> {
    info!("Writing CSV file '{file_path:?}' with {} lines", data.len());

    let params: Vec<ParamKind> = ParamKind::sorted();
    let mut wtr = Writer::from_writer(File::create(file_path)?);

    let mut headers = vec!["timestamp".to_string()];
    headers.extend(params.iter().map(|p| format!("{p:?}")));
    wtr.write_record(&headers)?;

    for (time, row) in data {
        let mut record = vec![time.to_string()];

        record.extend(
            params
                .iter()
                .map(|param| row.get(param).map_or_else(String::new, Value::to_string)),
        );

        wtr.write_record(&record)?;
    }

    wtr.flush()?;
    Ok(())
}
