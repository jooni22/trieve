use prometheus::{Encoder, Error, Gauge, Registry};

use crate::{data::models::RedisPool, errors::ServiceError};

#[derive(Clone, Debug)]
pub struct Metrics {
    registry: Registry,
    pub ingest_queue_gauge: Gauge,
    pub delete_queue_gauge: Gauge,
    pub file_queue_gauge: Gauge,
    pub update_queue_gauge: Gauge,
    pub file_processing_gauge: Gauge,
    pub delete_processing_gauge: Gauge,
    pub ingest_processing_gauge: Gauge,
    pub group_update_processing_gauge: Gauge,
}

impl Metrics {
    pub fn new() -> Result<Self, Error> {
        let registry = Registry::new();

        let ingest_queue_gauge = Gauge::new("ingest_queue", "number of items in the ingest queue")?;
        registry.register(Box::new(ingest_queue_gauge.clone()))?;

        let delete_queue_gauge = Gauge::new("delete_queue", "number of items in the delete queue")?;
        registry.register(Box::new(delete_queue_gauge.clone()))?;

        let file_queue_gauge =
            Gauge::new("file_queue", "number of items in the file ingest queue")?;
        registry.register(Box::new(file_queue_gauge.clone()))?;

        let update_queue_gauge =
            Gauge::new("group_update_queue", "number of items in the update queue")?;
        registry.register(Box::new(update_queue_gauge.clone()))?;

        let file_processing_gauge =
            Gauge::new("file_processing", "number of files being processed")?;
        registry.register(Box::new(file_processing_gauge.clone()))?;

        let delete_processing_gauge =
            Gauge::new("delete_processing", "number of files being deleted")?;
        registry.register(Box::new(delete_processing_gauge.clone()))?;

        let ingest_processing_gauge =
            Gauge::new("ingest_processing", "number of chunks being ingested")?;
        registry.register(Box::new(ingest_processing_gauge.clone()))?;

        let group_update_processing_gauge = Gauge::new(
            "group_update_processing",
            "number of group updates being processed",
        )?;
        registry.register(Box::new(group_update_processing_gauge.clone()))?;

        Ok(Metrics {
            registry,
            ingest_queue_gauge,
            file_queue_gauge,
            delete_queue_gauge,
            update_queue_gauge,
            file_processing_gauge,
            delete_processing_gauge,
            ingest_processing_gauge,
            group_update_processing_gauge,
        })
    }

    pub async fn update_queue_gauges(
        self: &Self,
        redis_pool: actix_web::web::Data<RedisPool>,
    ) -> Result<(), ServiceError> {
        let mut redis_conn = redis_pool
            .get()
            .await
            .map_err(|err| ServiceError::InternalServerError(err.to_string()))?;

        let (
            ingestion,
            delete_dataset_queue,
            file_ingestion,
            file_processing,
            delete_dataset_processing,
            processing,
            group_update_queue,
            group_update_processing,
        ): (i32, i32, i32, i32, i32, i32, i32, i32) = redis::pipe()
            .cmd("LLEN")
            .arg("ingestion")
            .cmd("LLEN")
            .arg("delete_dataset_queue")
            .cmd("LLEN")
            .arg("file_ingestion")
            .cmd("LLEN")
            .arg("file_processing")
            .cmd("LLEN")
            .arg("delete_dataset_processing")
            .cmd("LLEN")
            .arg("processing")
            .cmd("LLEN")
            .arg("group_update_queue")
            .cmd("LLEN")
            .arg("group_update_processing")
            .query_async(&mut *redis_conn)
            .await
            .map_err(|err| ServiceError::InternalServerError(err.to_string()))?;

        self.ingest_queue_gauge.set(ingestion as f64);
        self.delete_queue_gauge.set(delete_dataset_queue as f64);
        self.file_queue_gauge.set(file_ingestion as f64);
        self.file_processing_gauge.set(file_processing as f64);
        self.delete_processing_gauge
            .set(delete_dataset_processing as f64);
        self.ingest_processing_gauge.set(processing as f64);
        self.update_queue_gauge.set(group_update_queue as f64);
        self.group_update_processing_gauge
            .set(group_update_processing as f64);

        Ok(())
    }

    pub fn get_response(&self) -> String {
        let mut buffer = vec![];
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}
