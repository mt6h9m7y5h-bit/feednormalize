use bytes::Bytes;
use csv_async::AsyncReaderBuilder;
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::io::Cursor;
use thiserror::Error;
use tokio::io::{AsyncReadExt, BufReader};
use uuid::Uuid;

use crate::models::{Job, UniversalProduct};
use crate::services::{StorageService, normalized_output_key, original_file_key};
use crate::utils::parsing::{parse_price, trim_or_none};

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
    #[error("storage error: {0}")]
    Storage(#[from] crate::services::StorageError),
}

struct NormalizedWriter {
    output: Vec<u8>,
    first: bool,
}

impl NormalizedWriter {
    fn new() -> Self {
        let mut output = Vec::new();
        output.extend_from_slice(b"[\n");

        Self {
            output,
            first: true,
        }
    }

    async fn write_product(&mut self, product: &UniversalProduct) -> Result<(), NormalizationError> {
        if !self.first {
            self.output.extend_from_slice(b",\n");
        }
        self.first = false;

        let encoded = serde_json::to_vec(product)?;
        self.output.extend_from_slice(&encoded);

        Ok(())
    }

    fn finish(mut self) -> Bytes {
        self.output.extend_from_slice(b"\n]\n");
        Bytes::from(self.output)
    }
}

/// Transforms raw feed records into the canonical product schema.
#[derive(Debug, Default)]
pub struct NormalizationEngine;

impl NormalizationEngine {
    pub fn new() -> Self {
        Self
    }

    pub async fn process_feed(
        &self,
        storage: &StorageService,
        job: &Job,
    ) -> Result<Vec<UniversalProduct>, NormalizationError> {
        let input_key = original_file_key(job.id);

        if !storage.object_exists(&input_key).await? {
            return Err(NormalizationError::InputMissing(job.id));
        }

        let input_bytes = storage.get_object(&input_key).await?;
        let format = resolve_format(job);
        let mut writer = NormalizedWriter::new();
        let mut products = Vec::new();

        match format.as_str() {
            "csv" | "tsv" => {
                self.process_csv(input_bytes, format == "tsv", &mut writer, &mut products)
                    .await?;
            }
            "json" | "ndjson" => {
                self.process_json(input_bytes, &mut writer, &mut products)
                    .await?;
            }
            other => return Err(NormalizationError::UnsupportedFormat(other.to_owned())),
        }

        storage
            .put_object(&normalized_output_key(job.id), writer.finish())
            .await?;

        Ok(products)
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
        input_bytes: Bytes,
        tsv: bool,
        writer: &mut NormalizedWriter,
        products: &mut Vec<UniversalProduct>,
    ) -> Result<(), NormalizationError> {
        let reader = BufReader::new(Cursor::new(input_bytes));
        let mut csv_reader = AsyncReaderBuilder::new()
            .delimiter(if tsv { b'\t' } else { b',' })
            .create_reader(reader);

        let headers = csv_reader.headers().await?.clone();
        let mut records = csv_reader.into_records();

        while let Some(record) = records.next().await {
            let record = record?;
            let raw = record_to_json(&headers, &record);
            let product = self.normalize(&raw);
            products.push(product.clone());
            writer.write_product(&product).await?;
        }

        Ok(())
    }

    async fn process_json(
        &self,
        input_bytes: Bytes,
        writer: &mut NormalizedWriter,
        products: &mut Vec<UniversalProduct>,
    ) -> Result<(), NormalizationError> {
        let mut cursor = Cursor::new(input_bytes);
        let mut buffer = Vec::new();
        cursor.read_to_end(&mut buffer).await?;

        let trimmed = std::str::from_utf8(&buffer)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?
            .trim();

        if trimmed.starts_with('[') {
            if let Ok(products_input) = serde_json::from_str::<Vec<UniversalProduct>>(trimmed) {
                for product in &products_input {
                    let raw = serde_json::to_value(product)?;
                    let normalized = self.normalize(&raw);
                    products.push(normalized.clone());
                    writer.write_product(&normalized).await?;
                }
                return Ok(());
            }

            let rows: Vec<Value> = serde_json::from_str(trimmed)?;
            for row in rows {
                let product = self.normalize(&row);
                products.push(product.clone());
                writer.write_product(&product).await?;
            }
            return Ok(());
        }

        for line in trimmed.lines().filter(|line| !line.trim().is_empty()) {
            let row: Value = serde_json::from_str(line)?;
            let product = self.normalize(&row);
            products.push(product.clone());
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
