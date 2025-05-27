use std::{collections::HashMap, fs::File, path::PathBuf};

use csv::Writer;

use crate::params::{ParamKind, Value};

pub struct CsvWriter {
    writer: Writer<File>,
}

impl CsvWriter {
    pub fn new(file_path: &PathBuf) -> eyre::Result<Self> {
        let params: Vec<ParamKind> = ParamKind::sorted();
        let mut writer = Writer::from_writer(File::create(file_path)?);

        let mut headers = vec![
            "timestamp_before_read".to_string(),
            "timestamp_after_read".to_string(),
        ];
        headers.extend(params.iter().map(|p| format!("{p:?}")));
        writer.write_record(&headers)?;
        writer.flush()?;

        Ok(Self { writer })
    }

    pub fn write_row(
        &mut self,
        timestamp_before: &str,
        timestamp_after: &str,
        values: &HashMap<ParamKind, Value>,
    ) -> eyre::Result<()> {
        let params: Vec<ParamKind> = ParamKind::sorted();
        let mut record = vec![timestamp_before.to_string(), timestamp_after.to_string()];

        record.extend(
            params
                .iter()
                .map(|param| values.get(param).map_or_else(String::new, Value::to_string)),
        );

        self.writer.write_record(&record)?;
        Ok(())
    }
}
