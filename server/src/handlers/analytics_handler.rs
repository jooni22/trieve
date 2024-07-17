use super::auth_handler::AdminOnly;
use crate::{
    data::models::{
        ClusterAnalytics, ClusterAnalyticsResponse, DatasetAndOrgWithSubAndPlan, Pool,
        RAGAnalytics, RAGAnalyticsResponse, RecommendationAnalytics,
        RecommendationAnalyticsResponse, SearchAnalytics, SearchAnalyticsResponse,
    },
    errors::ServiceError,
    operators::analytics_operator::*,
};
use actix_web::{web, HttpResponse};

/// Get Cluster Analytics
///
/// This route allows you to view the cluster analytics for a dataset.
#[utoipa::path(
    post,
    path = "/analytics/search/cluster",
    context_path = "/api",
    tag = "Analytics",
    request_body(content = ClusterAnalytics, description = "JSON request payload to filter the graph", content_type = "application/json"),
    responses(
        (status = 200, description = "The cluster analytics for the dataset", body = ClusterAnalyticsResponse),

        (status = 400, description = "Service error relating to getting cluster analytics", body = ErrorResponseBody),
    ),
    params(
        ("TR-Dataset" = String, Header, description = "The dataset id to use for the request"),
    ),
    security(
        ("ApiKey" = ["admin"]),
    )
)]
pub async fn get_cluster_analytics(
    data: web::Json<ClusterAnalytics>,
    _user: AdminOnly,
    clickhouse_client: web::Data<clickhouse::Client>,
    pool: web::Data<Pool>,
    dataset_org_plan_sub: DatasetAndOrgWithSubAndPlan,
) -> Result<HttpResponse, ServiceError> {
    let response = match data.into_inner() {
        ClusterAnalytics::ClusterTopics { filter } => {
            let clusters = get_clusters_query(
                dataset_org_plan_sub.dataset.id,
                filter,
                clickhouse_client.get_ref(),
            )
            .await?;
            ClusterAnalyticsResponse::ClusterTopics(clusters)
        }
        ClusterAnalytics::ClusterQueries { cluster_id, page } => {
            let cluster_queries = get_queries_for_cluster_query(
                dataset_org_plan_sub.dataset.id,
                cluster_id,
                page,
                pool,
                clickhouse_client.get_ref(),
            )
            .await?;
            ClusterAnalyticsResponse::ClusterQueries(cluster_queries)
        }
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Get Search Analytics
///
/// This route allows you to view the search analytics for a dataset.
#[utoipa::path(
    post,
    path = "/analytics/search",
    context_path = "/api",
    tag = "Analytics",
    request_body(content = SearchAnalytics, description = "JSON request payload to filter the graph", content_type = "application/json"),
    responses(
        (status = 200, description = "The search analytics for the dataset", body = SearchAnalyticsResponse),

        (status = 400, description = "Service error relating to getting search analytics", body = ErrorResponseBody),
    ),
    params(
        ("TR-Dataset" = String, Header, description = "The dataset id to use for the request"),
    ),
    security(
        ("ApiKey" = ["admin"]),
    )
)]
pub async fn get_search_analytics(
    data: web::Json<SearchAnalytics>,
    _user: AdminOnly,
    clickhouse_client: web::Data<clickhouse::Client>,
    pool: web::Data<Pool>,
    dataset_org_plan_sub: DatasetAndOrgWithSubAndPlan,
) -> Result<HttpResponse, ServiceError> {
    let response = match data.into_inner() {
        SearchAnalytics::LatencyGraph {
            filter,
            granularity,
        } => {
            let latency_graph = get_latency_graph_query(
                dataset_org_plan_sub.dataset.id,
                filter,
                granularity,
                clickhouse_client.get_ref(),
            )
            .await?;

            SearchAnalyticsResponse::LatencyGraph(latency_graph)
        }
        SearchAnalytics::RPSGraph {
            filter,
            granularity,
        } => {
            let rps_graph = get_rps_graph_query(
                dataset_org_plan_sub.dataset.id,
                filter,
                granularity,
                clickhouse_client.get_ref(),
            )
            .await?;

            SearchAnalyticsResponse::RPSGraph(rps_graph)
        }
        SearchAnalytics::SearchMetrics { filter } => {
            let search_metrics = get_search_metrics_query(
                dataset_org_plan_sub.dataset.id,
                filter,
                clickhouse_client.get_ref(),
            )
            .await?;

            SearchAnalyticsResponse::SearchMetrics(search_metrics)
        }
        SearchAnalytics::HeadQueries { filter, page } => {
            let head_queries = get_head_queries_query(
                dataset_org_plan_sub.dataset.id,
                filter,
                page,
                clickhouse_client.get_ref(),
            )
            .await?;

            SearchAnalyticsResponse::HeadQueries(head_queries)
        }
        SearchAnalytics::LowConfidenceQueries {
            filter,
            page,
            threshold,
        } => {
            let low_confidence_queries = get_low_confidence_queries_query(
                dataset_org_plan_sub.dataset.id,
                filter,
                threshold,
                page,
                pool.clone(),
                clickhouse_client.get_ref(),
            )
            .await?;

            SearchAnalyticsResponse::LowConfidenceQueries(low_confidence_queries)
        }
        SearchAnalytics::NoResultQueries { filter, page } => {
            let no_result_queries = get_no_result_queries_query(
                dataset_org_plan_sub.dataset.id,
                filter,
                page,
                pool.clone(),
                clickhouse_client.get_ref(),
            )
            .await?;

            SearchAnalyticsResponse::NoResultQueries(no_result_queries)
        }
        SearchAnalytics::SearchQueries {
            filter,
            page,
            sort_by,
            sort_order,
        } => {
            let search_queries = get_all_queries_query(
                dataset_org_plan_sub.dataset.id,
                filter,
                sort_by,
                sort_order,
                page,
                pool.clone(),
                clickhouse_client.get_ref(),
            )
            .await?;

            SearchAnalyticsResponse::SearchQueries(search_queries)
        }
        SearchAnalytics::QueryDetails { search_id } => {
            let query = get_search_query(
                dataset_org_plan_sub.dataset.id,
                search_id,
                pool.clone(),
                clickhouse_client.get_ref(),
            )
            .await?;

            SearchAnalyticsResponse::QueryDetails(query)
        }
        SearchAnalytics::CountQueries { filter } => {
            let count_queries = get_query_counts_query(
                dataset_org_plan_sub.dataset.id,
                filter,
                clickhouse_client.get_ref(),
            )
            .await?;

            SearchAnalyticsResponse::CountQueries(count_queries)
        }
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Get RAG Analytics
///
/// This route allows you to view the RAG analytics for a dataset.
#[utoipa::path(
    post,
    path = "/analytics/rag",
    context_path = "/api",
    tag = "Analytics",
    request_body(content = RAGAnalytics, description = "JSON request payload to filter the graph", content_type = "application/json"),
    responses(
        (status = 200, description = "The RAG analytics for the dataset", body = RAGAnalyticsResponse),

        (status = 400, description = "Service error relating to getting RAG analytics", body = ErrorResponseBody),
    ),
    params(
        ("TR-Dataset" = String, Header, description = "The dataset id to use for the request"),
    ),
    security(
        ("ApiKey" = ["admin"]),
    )
)]
pub async fn get_rag_analytics(
    data: web::Json<RAGAnalytics>,
    _user: AdminOnly,
    clickhouse_client: web::Data<clickhouse::Client>,
    pool: web::Data<Pool>,
    dataset_org_plan_sub: DatasetAndOrgWithSubAndPlan,
) -> Result<HttpResponse, ServiceError> {
    let response = match data.into_inner() {
        RAGAnalytics::RAGUsage { filter } => {
            let rag_graph = get_rag_usage_query(
                dataset_org_plan_sub.dataset.id,
                filter,
                clickhouse_client.get_ref(),
            )
            .await?;
            RAGAnalyticsResponse::RAGUsage(rag_graph)
        }
        RAGAnalytics::RAGQueries {
            filter,
            page,
            sort_by,
            sort_order,
        } => {
            let rag_queries = get_rag_queries_query(
                dataset_org_plan_sub.dataset.id,
                filter,
                sort_by,
                sort_order,
                page,
                pool.clone(),
                clickhouse_client.get_ref(),
            )
            .await?;
            RAGAnalyticsResponse::RAGQueries(rag_queries)
        }
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Get Recommendation Analytics
///
/// This route allows you to view the recommendation analytics for a dataset.
#[utoipa::path(
    post,
    path = "/analytics/recommendation",
    context_path = "/api",
    tag = "Analytics",
    request_body(content = RecommendationAnalytics, description = "JSON request payload to filter the graph", content_type = "application/json"),
    responses(
        (status = 200, description = "The recommendation analytics for the dataset", body = RecommendationAnalyticsResponse),

        (status = 400, description = "Service error relating to getting recommendation analytics", body = ErrorResponseBody),
    ),
    params(
        ("TR-Dataset" = String, Header, description = "The dataset id to use for the request"),
    ),
    security(
        ("ApiKey" = ["admin"]),
    )
)]
pub async fn get_recommendation_analytics(
    data: web::Json<RecommendationAnalytics>,
    _user: AdminOnly,
    clickhouse_client: web::Data<clickhouse::Client>,
    pool: web::Data<Pool>,
    dataset_org_plan_sub: DatasetAndOrgWithSubAndPlan,
) -> Result<HttpResponse, ServiceError> {
    let response = match data.into_inner() {
        RecommendationAnalytics::LowConfidenceRecommendations {
            filter,
            page,
            threshold,
        } => {
            let low_confidence_recommendations = get_low_confidence_recommendations_query(
                dataset_org_plan_sub.dataset.id,
                filter,
                threshold,
                page,
                pool.clone(),
                clickhouse_client.get_ref(),
            )
            .await?;
            RecommendationAnalyticsResponse::LowConfidenceRecommendations(
                low_confidence_recommendations,
            )
        }
        RecommendationAnalytics::RecommendationQueries {
            filter,
            page,
            sort_by,
            sort_order,
        } => {
            let recommendation_queries = get_recommendation_queries_query(
                dataset_org_plan_sub.dataset.id,
                filter,
                sort_by,
                sort_order,
                page,
                pool.clone(),
                clickhouse_client.get_ref(),
            )
            .await?;
            RecommendationAnalyticsResponse::RecommendationQueries(recommendation_queries)
        }
    };

    Ok(HttpResponse::Ok().json(response))
}
