use std::{collections::HashMap, fs::File, path::PathBuf};

use csv::Writer;
use strum::IntoEnumIterator;
use tracing::info;

use crate::params::{Param, Value};

pub fn write_csv(data: Vec<(f32, HashMap<Param, Value>)>, file_path: &PathBuf) -> eyre::Result<()> {
    info!("Writing CSV file '{file_path:?}' with {} lines", data.len());

    let params: Vec<Param> = Param::iter().collect();
    let mut wtr = Writer::from_writer(File::create(file_path)?);

    let mut headers = vec!["timestamp".to_string()];
    headers.extend(params.iter().map(|p| format!("{p:?}")));
    wtr.write_record(&headers)?;

    for (time, row) in data {
        let mut record = vec![time.to_string()];

        record.extend(params.iter().map(|param| match row.get(param) {
            Some(Value::Int(_, v)) => v.to_string(),
            Some(Value::Float(_, v)) => v.to_string(),
            None => String::new(),
        }));

        wtr.write_record(&record)?;
    }

    wtr.flush()?;
    Ok(())
}
