use std::path::Path;

use csv_async::AsyncReaderBuilder;
use futures_util::StreamExt;
use serde_json::{Value, json};
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

use crate::models::{Job, UniversalProduct};
use crate::utils::parsing::{parse_price, trim_or_none};
use crate::utils::{normalized_output_path, original_file_path};

#[derive(Debug, Error)]
pub enum NormalizationError {
    #[error("uploaded file not found for job {0}")]
    InputMissing(Uuid),
    #[error("unsupported feed format: {0}")]
    UnsupportedFormat(String),
    #[error("csv error: {0}")]
    Csv(#[from] csv_async::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

struct NormalizedWriter {
    output: File,
    first: bool,
}

impl NormalizedWriter {
    async fn new(path: &Path) -> Result<Self, NormalizationError> {
        let mut output = File::create(path).await?;
        output.write_all(b"[\n").await?;

        Ok(Self {
            output,
            first: true,
        })
    }

    async fn write_product(&mut self, product: &UniversalProduct) -> Result<(), NormalizationError> {
        if !self.first {
            self.output.write_all(b",\n").await?;
        }
        self.first = false;

        let encoded = serde_json::to_vec(product)?;
        self.output.write_all(&encoded).await?;

        Ok(())
    }

    async fn finish(mut self) -> Result<(), NormalizationError> {
        self.output.write_all(b"\n]\n").await?;
        self.output.flush().await?;
        Ok(())
    }
}

/// Transforms raw feed records into the canonical product schema.
#[derive(Debug, Default)]
pub struct NormalizationEngine;

impl NormalizationEngine {
    pub fn new() -> Self {
        Self
    }

    pub async fn process_feed(&self, job: &Job) -> Result<(), NormalizationError> {
        let input_path = original_file_path(job.id);
        let output_path = normalized_output_path(job.id);

        if !input_path.exists() {
            return Err(NormalizationError::InputMissing(job.id));
        }

        let format = resolve_format(job);
        let mut writer = NormalizedWriter::new(&output_path).await?;

        match format.as_str() {
            "csv" | "tsv" => {
                self.process_csv(&input_path, format == "tsv", &mut writer)
                    .await?;
            }
            "json" | "ndjson" => {
                self.process_json(&input_path, &mut writer).await?;
            }
            other => return Err(NormalizationError::UnsupportedFormat(other.to_owned())),
        }

        writer.finish().await?;
        Ok(())
    }

    /// Maps common supplier field names to the universal schema.
    pub fn normalize(&self, raw: &Value) -> UniversalProduct {
        let str_field = |keys: &[&str]| -> Option<String> {
            keys.iter()
                .find_map(|key| raw.get(*key))
                .and_then(|value| value.as_str())
                .and_then(trim_or_none)
        };

        let price = ["price", "product_price", "cost", "gross_price"]
            .iter()
            .find_map(|key| raw.get(*key))
            .and_then(|value| {
                value
                    .as_f64()
                    .or_else(|| value.as_str().and_then(parse_price))
            });

        UniversalProduct {
            sku: str_field(&["sku", "article_number", "product_id", "item_no", "id"]),
            title: str_field(&["title", "product_name", "headline", "description_title"]),
            price,
            currency: str_field(&["currency"]),
            ean: str_field(&["ean", "gtin", "upc"]),
        }
    }

    async fn process_csv(
        &self,
        input_path: &Path,
        tsv: bool,
        writer: &mut NormalizedWriter,
    ) -> Result<(), NormalizationError> {
        let file = File::open(input_path).await?;
        let mut reader = AsyncReaderBuilder::new()
            .delimiter(if tsv { b'\t' } else { b',' })
            .create_reader(file);

        let headers = reader.headers().await?.clone();
        let mut records = reader.into_records();

        while let Some(record) = records.next().await {
            let record = record?;
            let raw = record_to_json(&headers, &record);
            let product = self.normalize(&raw);
            writer.write_product(&product).await?;
        }

        Ok(())
    }

    async fn process_json(
        &self,
        input_path: &Path,
        writer: &mut NormalizedWriter,
    ) -> Result<(), NormalizationError> {
        let mut file = File::open(input_path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        let trimmed = std::str::from_utf8(&buffer)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?
            .trim();

        if trimmed.starts_with('[') {
            if let Ok(products) = serde_json::from_str::<Vec<UniversalProduct>>(trimmed) {
                for product in &products {
                    let raw = serde_json::to_value(product)?;
                    let normalized = self.normalize(&raw);
                    writer.write_product(&normalized).await?;
                }
                return Ok(());
            }

            let rows: Vec<Value> = serde_json::from_str(trimmed)?;
            for row in rows {
                let product = self.normalize(&row);
                writer.write_product(&product).await?;
            }
            return Ok(());
        }

        for line in trimmed.lines().filter(|line| !line.trim().is_empty()) {
            let row: Value = serde_json::from_str(line)?;
            let product = self.normalize(&row);
            writer.write_product(&product).await?;
        }

        Ok(())
    }
}

fn resolve_format(job: &Job) -> String {
    if let Some(format) = job.format.as_deref() {
        return format.to_ascii_lowercase();
    }

    job.filename
        .as_deref()
        .and_then(|name| name.rsplit('.').next())
        .map(str::to_ascii_lowercase)
        .unwrap_or_else(|| "csv".to_owned())
}

fn record_to_json(
    headers: &csv_async::StringRecord,
    record: &csv_async::StringRecord,
) -> Value {
    let mut object = serde_json::Map::new();

    for (header, value) in headers.iter().zip(record.iter()) {
        object.insert(header.to_owned(), json!(value));
    }

    Value::Object(object)
}
