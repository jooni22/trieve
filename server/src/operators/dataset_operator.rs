use crate::data::models::{
    DatasetAndOrgWithSubAndPlan, DatasetAndUsage, DatasetUsageCount, Organization,
    OrganizationWithSubAndPlan, RedisPool, ServerDatasetConfiguration, StripePlan,
    StripeSubscription, UnifiedId,
};
use crate::get_env;
use crate::handlers::dataset_handler::GetDatasetsPagination;
use crate::operators::event_operator::create_event_query;
use crate::operators::qdrant_operator::get_qdrant_connection;
use crate::{
    data::models::{Dataset, Event, EventType, Pool},
    errors::ServiceError,
};
use actix_web::web;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DBError};
use diesel_async::RunQueryDsl;
use qdrant_client::qdrant::{Condition, Filter};
use serde::{Deserialize, Serialize};

#[tracing::instrument(skip(pool))]
pub async fn create_dataset_query(
    new_dataset: Dataset,
    pool: web::Data<Pool>,
) -> Result<Dataset, ServiceError> {
    use crate::data::schema::datasets::dsl::*;

    let mut conn = pool
        .get()
        .await
        .map_err(|_| ServiceError::BadRequest("Could not get database connection".to_string()))?;

    diesel::insert_into(datasets)
        .values(&new_dataset)
        .execute(&mut conn)
        .await
        .map_err(|err| match err {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            ) => ServiceError::BadRequest(
                "Could not create dataset because a dataset with the same tracking_id already exists in the organization".to_string(),
            ),
            _ => ServiceError::BadRequest("Could not create dataset".to_string()),
        })?;

    Ok(new_dataset)
}

#[tracing::instrument(skip(pool))]
pub async fn get_dataset_by_id_query(
    id: UnifiedId,
    pool: web::Data<Pool>,
) -> Result<Dataset, ServiceError> {
    use crate::data::schema::datasets::dsl as datasets_columns;
    let mut conn = pool
        .get()
        .await
        .map_err(|_| ServiceError::BadRequest("Could not get database connection".to_string()))?;

    let dataset = match id {
        UnifiedId::TrieveUuid(id) => datasets_columns::datasets
            .filter(datasets_columns::id.eq(id))
            .filter(datasets_columns::deleted.eq(0))
            .select(Dataset::as_select())
            .first(&mut conn)
            .await
            .map_err(|_| ServiceError::NotFound("Could not find dataset".to_string()))?,
        UnifiedId::TrackingId(id) => datasets_columns::datasets
            .filter(datasets_columns::tracking_id.eq(id))
            .filter(datasets_columns::deleted.eq(0))
            .select(Dataset::as_select())
            .first(&mut conn)
            .await
            .map_err(|_| ServiceError::NotFound("Could not find dataset".to_string()))?,
    };

    Ok(dataset)
}

#[tracing::instrument(skip(pool))]
pub async fn get_dataset_and_organization_from_dataset_id_query(
    id: UnifiedId,
    pool: web::Data<Pool>,
) -> Result<DatasetAndOrgWithSubAndPlan, ServiceError> {
    use crate::data::schema::datasets::dsl as datasets_columns;
    use crate::data::schema::organizations::dsl as organizations_columns;
    use crate::data::schema::stripe_plans::dsl as stripe_plans_columns;
    use crate::data::schema::stripe_subscriptions::dsl as stripe_subscriptions_columns;

    let mut conn = pool
        .get()
        .await
        .map_err(|_| ServiceError::BadRequest("Could not get database connection".to_string()))?;

    let query = datasets_columns::datasets
        .inner_join(organizations_columns::organizations)
        .left_outer_join(
            stripe_subscriptions_columns::stripe_subscriptions
                .on(stripe_subscriptions_columns::organization_id.eq(organizations_columns::id)),
        )
        .left_outer_join(
            stripe_plans_columns::stripe_plans
                .on(stripe_plans_columns::id.eq(stripe_subscriptions_columns::plan_id)),
        )
        .filter(datasets_columns::deleted.eq(0))
        .into_boxed();

    let (dataset, organization, stripe_plan, stripe_subscription) = match id {
        UnifiedId::TrieveUuid(id) => query
            .filter(datasets_columns::id.eq(id))
            .filter(datasets_columns::deleted.eq(0))
            .select((
                Dataset::as_select(),
                organizations_columns::organizations::all_columns(),
                stripe_plans_columns::stripe_plans::all_columns().nullable(),
                stripe_subscriptions_columns::stripe_subscriptions::all_columns().nullable(),
            ))
            .first::<(
                Dataset,
                Organization,
                Option<StripePlan>,
                Option<StripeSubscription>,
            )>(&mut conn)
            .await
            .map_err(|_| ServiceError::NotFound("Could not find dataset".to_string()))?,
        UnifiedId::TrackingId(id) => query
            .filter(datasets_columns::tracking_id.eq(id))
            .filter(datasets_columns::deleted.eq(0))
            .select((
                Dataset::as_select(),
                organizations_columns::organizations::all_columns(),
                stripe_plans_columns::stripe_plans::all_columns().nullable(),
                stripe_subscriptions_columns::stripe_subscriptions::all_columns().nullable(),
            ))
            .first::<(
                Dataset,
                Organization,
                Option<StripePlan>,
                Option<StripeSubscription>,
            )>(&mut conn)
            .await
            .map_err(|_| ServiceError::NotFound("Could not find dataset".to_string()))?,
    };

    let org_with_plan_sub: OrganizationWithSubAndPlan =
        OrganizationWithSubAndPlan::from_components(organization, stripe_plan, stripe_subscription);

    Ok(DatasetAndOrgWithSubAndPlan::from_components(
        dataset,
        org_with_plan_sub,
    ))
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeleteMessage {
    pub dataset_id: uuid::Uuid,
    pub server_config: ServerDatasetConfiguration,
    pub attempt_number: usize,
    pub empty_dataset: bool,
}

#[tracing::instrument(skip(pool))]
pub async fn soft_delete_dataset_by_id_query(
    id: uuid::Uuid,
    config: ServerDatasetConfiguration,
    pool: web::Data<Pool>,
    redis_pool: web::Data<RedisPool>,
) -> Result<(), ServiceError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|_| ServiceError::BadRequest("Could not get database connection".to_string()))?;

    if config.LOCKED {
        return Err(ServiceError::BadRequest(
            "Cannot delete a locked dataset".to_string(),
        ));
    }

    diesel::sql_query(format!(
        "UPDATE datasets SET deleted = 1, tracking_id = NULL WHERE id = '{}'::uuid",
        id
    ))
    .execute(&mut conn)
    .await
    .map_err(|err| {
        log::error!("Could not delete dataset: {}", err);
        ServiceError::BadRequest("Could not delete dataset".to_string())
    })?;

    let mut redis_conn = redis_pool
        .get()
        .await
        .map_err(|err| ServiceError::BadRequest(err.to_string()))?;

    let message = DeleteMessage {
        dataset_id: id,
        server_config: config,
        attempt_number: 0,
        empty_dataset: false,
    };

    let serialized_message =
        serde_json::to_string(&message).map_err(|err| ServiceError::BadRequest(err.to_string()))?;

    redis::cmd("lpush")
        .arg("delete_dataset_queue")
        .arg(&serialized_message)
        .query_async(&mut *redis_conn)
        .await
        .map_err(|err| ServiceError::BadRequest(err.to_string()))?;

    Ok(())
}

pub async fn clear_dataset_by_dataset_id_query(
    id: uuid::Uuid,
    config: ServerDatasetConfiguration,
    redis_pool: web::Data<RedisPool>,
) -> Result<(), ServiceError> {
    if config.LOCKED {
        return Err(ServiceError::BadRequest(
            "Cannot delete a locked dataset".to_string(),
        ));
    }
    let mut redis_conn = redis_pool
        .get()
        .await
        .map_err(|err| ServiceError::BadRequest(err.to_string()))?;

    let message = DeleteMessage {
        dataset_id: id,
        server_config: config,
        attempt_number: 0,
        empty_dataset: true,
    };

    let serialized_message =
        serde_json::to_string(&message).map_err(|err| ServiceError::BadRequest(err.to_string()))?;

    redis::cmd("lpush")
        .arg("delete_dataset_queue")
        .arg(&serialized_message)
        .query_async(&mut *redis_conn)
        .await
        .map_err(|err| ServiceError::BadRequest(err.to_string()))?;

    Ok(())
}

#[tracing::instrument(skip(pool, clickhouse_client))]
pub async fn delete_chunks_in_dataset(
    id: uuid::Uuid,
    pool: web::Data<Pool>,
    clickhouse_client: web::Data<clickhouse::Client>,
    config: ServerDatasetConfiguration,
) -> Result<(), ServiceError> {
    use crate::data::schema::chunk_metadata::dsl as chunk_metadata_columns;

    let mut conn = pool.get().await.unwrap();

    let qdrant_collection = format!("{}_vectors", config.EMBEDDING_SIZE);

    let qdrant_client = get_qdrant_connection(
        Some(get_env!("QDRANT_URL", "QDRANT_URL should be set")),
        Some(get_env!("QDRANT_API_KEY", "QDRANT_API_KEY should be set")),
    )
    .await?;

    let mut last_offset_id = uuid::Uuid::nil();

    loop {
        // Fetch a batch of chunk IDs
        let chunk_and_qdrant_ids = chunk_metadata_columns::chunk_metadata
            .filter(chunk_metadata_columns::dataset_id.eq(id))
            .filter(chunk_metadata_columns::id.gt(last_offset_id))
            .select((
                chunk_metadata_columns::id,
                chunk_metadata_columns::qdrant_point_id,
            ))
            .order(chunk_metadata_columns::id)
            .limit(
                option_env!("DELETE_CHUNK_BATCH_SIZE")
                    .unwrap_or("5000")
                    .parse::<i64>()
                    .unwrap_or(5000),
            )
            .load::<(uuid::Uuid, Option<uuid::Uuid>)>(&mut conn)
            .await
            .map_err(|err| {
                log::error!("Could not fetch chunk IDs: {}", err);
                ServiceError::BadRequest("Could not fetch chunk IDs".to_string())
            })?;

        let chunk_ids = chunk_and_qdrant_ids
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<uuid::Uuid>>();
        let qdrant_point_ids = chunk_and_qdrant_ids
            .iter()
            .filter_map(|(_, qdrant_id)| Some((*qdrant_id)?.to_string()))
            .collect::<Vec<String>>();

        if chunk_ids.is_empty() {
            break;
        }

        // Delete the chunks in the current batch
        diesel::delete(
            chunk_metadata_columns::chunk_metadata
                .filter(chunk_metadata_columns::id.eq_any(&chunk_ids)),
        )
        .execute(&mut conn)
        .await
        .map_err(|err| {
            log::error!("Could not delete chunks: {}", err);
            ServiceError::BadRequest("Could not delete chunks".to_string())
        })?;

        let _ = create_event_query(
            Event::from_details(
                id,
                EventType::BulkChunksDeleted {
                    message: format!("Deleted {} chunks", chunk_ids.len()),
                },
            ),
            clickhouse_client.clone(),
        )
        .await
        .map_err(|err| {
            log::error!("Failed to create event: {:?}", err);
        });

        qdrant_client
            .delete_points(
                qdrant_collection.clone(),
                None,
                &Filter::must([Condition::has_id(qdrant_point_ids)]).into(),
                None,
            )
            .await
            .map_err(|err| {
                ServiceError::BadRequest(format!("Could not delete points from qdrant: {}", err))
            })?;

        // Move to the next batch
        last_offset_id = *chunk_ids.last().unwrap();
    }

    Ok(())
}

#[tracing::instrument(skip(pool, clickhouse_client))]
pub async fn delete_dataset_by_id_query(
    id: uuid::Uuid,
    pool: web::Data<Pool>,
    clickhouse_client: actix_web::web::Data<clickhouse::Client>,
    config: ServerDatasetConfiguration,
) -> Result<Dataset, ServiceError> {
    use crate::data::schema::datasets::dsl as datasets_columns;

    let mut conn = pool.get().await.unwrap();

    delete_chunks_in_dataset(id, pool.clone(), clickhouse_client.clone(), config.clone()).await?;

    let dataset: Dataset =
        diesel::delete(datasets_columns::datasets.filter(datasets_columns::id.eq(id)))
            .get_result(&mut conn)
            .await
            .map_err(|err| {
                log::error!("Could not delete dataset: {}", err);
                ServiceError::BadRequest("Could not delete dataset".to_string())
            })?;

    clickhouse_client
        .query("DELETE FROM default.dataset_events WHERE dataset_id = ?")
        .bind(id)
        .execute()
        .await
        .map_err(|err| {
            log::error!("Could not delete dataset events: {}", err);
            ServiceError::BadRequest("Could not delete dataset events".to_string())
        })?;

    clickhouse_client
        .query(
            "
        ALTER TABLE default.dataset_events
        DELETE WHERE dataset_id = ?;
        ",
        )
        .bind(id)
        .execute()
        .await
        .unwrap();

    clickhouse_client
        .query(
            "
        ALTER TABLE default.search_queries
        DELETE WHERE dataset_id = ?;
        ",
        )
        .bind(id)
        .execute()
        .await
        .unwrap();

    clickhouse_client
        .query(
            "
        ALTER TABLE default.search_cluster_memberships
        DELETE WHERE cluster_id IN (
            SELECT id FROM cluster_topics WHERE dataset_id = ?
        );
        ",
        )
        .bind(id)
        .execute()
        .await
        .unwrap();

    clickhouse_client
        .query(
            "
        ALTER TABLE default.cluster_topics
        DELETE WHERE dataset_id = ?;
        ",
        )
        .bind(id)
        .execute()
        .await
        .unwrap();

    Ok(dataset)
}

#[tracing::instrument(skip(pool))]
pub async fn update_dataset_query(
    id: uuid::Uuid,
    name: String,
    server_configuration: serde_json::Value,
    client_configuration: serde_json::Value,
    new_tracking_id: Option<String>,
    pool: web::Data<Pool>,
) -> Result<Dataset, ServiceError> {
    use crate::data::schema::datasets::dsl as datasets_columns;

    let mut conn = pool
        .get()
        .await
        .map_err(|_| ServiceError::BadRequest("Could not get database connection".to_string()))?;

    let new_dataset: Dataset = diesel::update(
        datasets_columns::datasets
            .filter(datasets_columns::id.eq(id))
            .filter(datasets_columns::deleted.eq(0)),
    )
    .set((
        new_tracking_id.map(|id| datasets_columns::tracking_id.eq(id)),
        datasets_columns::name.eq(name),
        datasets_columns::updated_at.eq(diesel::dsl::now),
        datasets_columns::server_configuration.eq(server_configuration),
        datasets_columns::client_configuration.eq(client_configuration),
    ))
    .get_result(&mut conn)
    .await
    .map_err(|e: DBError| {
        match e {
        DBError::DatabaseError(db_error, _) => match db_error {
            DatabaseErrorKind::UniqueViolation => {
                ServiceError::BadRequest("Could not update tracking_id because a dataset with the same tracking_id already exists in the organization".to_string())
            }
            _ => ServiceError::BadRequest("Failed to update dataset".to_string())
        }
        _ => {
            ServiceError::BadRequest("Failed to update dataset".to_string())
        }
    }
    })?;

    Ok(new_dataset)
}

#[tracing::instrument(skip(pool))]
pub async fn get_datasets_by_organization_id(
    org_id: web::Path<uuid::Uuid>,
    pagination: GetDatasetsPagination,
    pool: web::Data<Pool>,
) -> Result<Vec<DatasetAndUsage>, ServiceError> {
    use crate::data::schema::dataset_usage_counts::dsl as dataset_usage_counts_columns;
    use crate::data::schema::datasets::dsl as datasets_columns;

    let mut conn = pool
        .get()
        .await
        .map_err(|_| ServiceError::BadRequest("Could not get database connection".to_string()))?;

    let get_datasets_query = datasets_columns::datasets
        .inner_join(dataset_usage_counts_columns::dataset_usage_counts)
        .filter(datasets_columns::deleted.eq(0))
        .filter(datasets_columns::organization_id.eq(org_id.into_inner()))
        .order(datasets_columns::created_at.desc())
        .select((Dataset::as_select(), DatasetUsageCount::as_select()))
        .into_boxed();

    let dataset_and_usages = match pagination.limit {
        Some(limit) => get_datasets_query
            .offset(pagination.offset.unwrap_or(0))
            .limit(limit)
            .load::<(Dataset, DatasetUsageCount)>(&mut conn)
            .await
            .map_err(|_| ServiceError::NotFound("Could not find organization".to_string()))?,
        None => get_datasets_query
            .load::<(Dataset, DatasetUsageCount)>(&mut conn)
            .await
            .map_err(|_| ServiceError::NotFound("Could not find organization".to_string()))?,
    };

    let dataset_and_usages = dataset_and_usages
        .into_iter()
        .map(|(dataset, usage_count)| DatasetAndUsage::from_components(dataset.into(), usage_count))
        .collect::<Vec<DatasetAndUsage>>();

    Ok(dataset_and_usages)
}

pub async fn get_dataset_usage_query(
    dataset_id: uuid::Uuid,
    pool: web::Data<Pool>,
) -> Result<DatasetUsageCount, ServiceError> {
    use crate::data::schema::dataset_usage_counts::dsl as dataset_usage_counts_columns;

    let mut conn = pool
        .get()
        .await
        .map_err(|_| ServiceError::BadRequest("Could not get database connection".to_string()))?;

    let dataset_usage = dataset_usage_counts_columns::dataset_usage_counts
        .filter(dataset_usage_counts_columns::dataset_id.eq(dataset_id))
        .first::<DatasetUsageCount>(&mut conn)
        .await
        .map_err(|_| ServiceError::NotFound("Could not find dataset".to_string()))?;

    Ok(dataset_usage)
}
