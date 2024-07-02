use crate::{
    data::models::{Pool, RedisPool, SlimUser, UnifiedId, User, UserApiKey, UserRole},
    errors::ServiceError,
    handlers::auth_handler::{AdminOnly, LoggedUser, OrganizationRole, OwnerOnly},
    operators::{
        dataset_operator::get_dataset_and_organization_from_dataset_id_query,
        organization_operator::{
            get_arbitrary_org_owner_from_dataset_id, get_arbitrary_org_owner_from_org_id,
        },
        user_operator::{get_user_by_id_query, get_user_from_api_key_query},
    },
};
use actix_identity::Identity;
use actix_web::{
    dev::{forward_ready, Payload, Service, ServiceRequest, ServiceResponse, Transform},
    http::header::HeaderMap,
    web, Error, FromRequest, HttpMessage, HttpRequest,
};
use futures_util::future::LocalBoxFuture;
use redis::AsyncCommands;
use sentry::Transaction;
use std::{
    future::{ready, Ready},
    rc::Rc,
};

pub struct AuthenticationMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthenticationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);
    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        // Clone the Rc pointers so we can move them into the async block.
        let srv = self.service.clone();
        Box::pin(async move {
            let tx_ctx =
                sentry::TransactionContext::new("middleware", "get dataset, org, and/or user");
            let transaction = sentry::start_transaction(tx_ctx);
            sentry::configure_scope(|scope| scope.set_span(Some(transaction.clone().into())));

            let pool = req.app_data::<web::Data<Pool>>().unwrap().to_owned();

            let get_user_span = transaction.start_child("get_user", "Getting user");

            let (http_req, pl) = req.parts_mut();
            let mut user = get_user(http_req, pl, transaction.clone(), pool.clone()).await;
            let mut api_key = None;
            if user.is_none() {
                (user, api_key) =
                    auth_with_api_key(http_req, transaction.clone(), pool.clone()).await?;
            }

            if let Some(user) = user.clone() {
                req.extensions_mut().insert(user);
            }

            get_user_span.finish();

            let org_id = match get_dataset_id_from_headers(req.headers()) {
                Some(dataset_id) => {
                    let get_dataset_and_org_span = transaction
                        .start_child("get_dataset_and_org", "Getting dataset and organization");

                    let dataset_org_plan_sub = match dataset_id.parse::<uuid::Uuid>() {
                        Ok(dataset_id) => {
                            get_dataset_and_organization_from_dataset_id_query(
                                UnifiedId::TrieveUuid(dataset_id),
                                pool.clone(),
                            )
                            .await?
                        }
                        Err(_) => {
                            get_dataset_and_organization_from_dataset_id_query(
                                UnifiedId::TrackingId(dataset_id.clone()),
                                pool.clone(),
                            )
                            .await?
                        }
                    };

                    get_dataset_and_org_span.finish();

                    if let Some(user_api_key) = api_key {
                        if let Some(api_key_org_ids) = user_api_key.organization_ids {
                            if !api_key_org_ids.is_empty()
                                && !api_key_org_ids.contains(&Some(
                                    dataset_org_plan_sub
                                        .organization
                                        .organization
                                        .id
                                        .to_string(),
                                ))
                            {
                                return Err(ServiceError::Unauthorized.into());
                            }
                        }

                        if let Some(api_key_dataset_ids) = user_api_key.dataset_ids {
                            if !api_key_dataset_ids.is_empty()
                                && !api_key_dataset_ids
                                    .contains(&Some(dataset_org_plan_sub.dataset.id.to_string()))
                            {
                                return Err(ServiceError::Unauthorized.into());
                            }
                        }
                    }

                    req.extensions_mut().insert(dataset_org_plan_sub.clone());

                    dataset_org_plan_sub.organization.organization.id
                }
                None => {
                    if let Some(org_header) = get_org_id_from_headers(req.headers()) {
                        org_header.parse::<uuid::Uuid>().map_err(|_| {
                            Into::<Error>::into(ServiceError::BadRequest(
                                "Could not convert Organization to UUID".to_string(),
                            ))
                        })?
                    } else {
                        let res = srv.call(req).await?;
                        return Ok(res);
                    }
                }
            };

            if let Some(user) = user {
                let find_user_org_span =
                    transaction.start_child("find_user_org_role", "Finding user org role");

                let user_org = user
                    .user_orgs
                    .iter()
                    .find(|org| org.organization_id == org_id)
                    .ok_or(ServiceError::Forbidden)?;

                let org_role = if user_org.role >= UserRole::User.into() {
                    Ok(OrganizationRole {
                        user: user.clone(),
                        role: UserRole::from(user_org.role),
                    })
                } else {
                    Err(ServiceError::Forbidden)
                }?;

                req.extensions_mut().insert(org_role);

                find_user_org_span.finish();
            }

            transaction.finish();

            let res = srv.call(req).await?;

            Ok(res)
        })
    }
}

async fn get_user(
    req: &HttpRequest,
    pl: &mut Payload,
    tx: Transaction,
    pool: web::Data<Pool>,
) -> Option<LoggedUser> {
    let get_user_from_identity_span =
        tx.start_child("get_user_from_identity", "Getting user from identity");
    if let Ok(identity) = Identity::from_request(req, pl).into_inner() {
        if let Ok(user_json) = identity.id() {
            if let Ok(user) = serde_json::from_str::<User>(&user_json) {
                let redis_pool = req.app_data::<web::Data<RedisPool>>().unwrap().to_owned();

                let mut redis_conn = redis_pool.get().await.ok()?;

                let slim_user_string: Result<String, _> =
                    redis_conn.get(&user.id.to_string()).await;

                match slim_user_string {
                    Ok(slim_user_string) => {
                        let slim_user = serde_json::from_str::<SlimUser>(&slim_user_string).ok()?;

                        return Some(slim_user);
                    }
                    Err(_) => {
                        let (user, user_orgs, orgs) =
                            get_user_by_id_query(&user.id, pool).await.ok()?;
                        let slim_user = SlimUser::from_details(user, user_orgs, orgs);

                        let slim_user_string = serde_json::to_string(&slim_user).ok()?;
                        redis_conn
                            .set(&slim_user.id.to_string(), slim_user_string)
                            .await
                            .ok()?;

                        return Some(slim_user);
                    }
                }
            }
        }
    }
    get_user_from_identity_span.finish();

    None
}

// Can either be Bearer {}, or x-api-key, or Authorization
fn get_api_key_from_headers(headers: &HeaderMap) -> Option<String> {
    if let Some(auth_header_value) = headers.get("Authorization") {
        // Check if the Authorization header is a Bearer token
        if let Ok(auth_header_value) = auth_header_value.to_str() {
            if let Some(stripeped_auth_header) = auth_header_value.strip_prefix("Bearer ") {
                return Some(stripeped_auth_header.to_string());
            } else {
                return Some(auth_header_value.to_string());
            }
        }
    }
    // Check for x-api-key,
    if let Some(api_key_header_value) = headers.get("x-api-key") {
        if let Ok(api_key_header_value) = api_key_header_value.to_str() {
            if let Some(stripeped_api_key_header_value) =
                api_key_header_value.strip_prefix("Bearer ")
            {
                return Some(stripeped_api_key_header_value.to_string());
            } else {
                return Some(api_key_header_value.to_string());
            }
        }
    }

    None
}

fn get_dataset_id_from_headers(headers: &HeaderMap) -> Option<String> {
    if let Some(dataset_id_header) = headers.get("TR-Dataset") {
        if let Ok(dataset_id) = dataset_id_header.to_str() {
            return Some(dataset_id.to_string());
        }
    }

    if let Some(dataset_id_header) = headers.get("x-dataset") {
        if let Ok(dataset_id) = dataset_id_header.to_str() {
            return Some(dataset_id.to_string());
        }
    }

    None
}

fn get_org_id_from_headers(headers: &HeaderMap) -> Option<String> {
    if let Some(org_id_header) = headers.get("TR-Organization") {
        if let Ok(org_id) = org_id_header.to_str() {
            return Some(org_id.to_string());
        }
    }

    if let Some(org_id_header) = headers.get("x-organization") {
        if let Ok(org_id) = org_id_header.to_str() {
            return Some(org_id.to_string());
        }
    }

    None
}

async fn auth_with_api_key(
    req: &HttpRequest,
    tx: Transaction,
    pool: web::Data<Pool>,
) -> Result<(Option<LoggedUser>, Option<UserApiKey>), ServiceError> {
    if let Some(authen_header) = get_api_key_from_headers(req.headers()) {
        if authen_header == std::env::var("ADMIN_API_KEY").unwrap_or("".to_string()) {
            if let Some(org_id) = get_org_id_from_headers(req.headers()) {
                if let Ok(org_id) = org_id.parse::<uuid::Uuid>() {
                    if let Ok(user) =
                        get_arbitrary_org_owner_from_org_id(org_id, pool.clone()).await
                    {
                        return Ok((Some(user), None));
                    }
                }
            }

            if let Some(dataset_id) = get_dataset_id_from_headers(req.headers()) {
                if let Ok(dataset_id) = dataset_id.parse::<uuid::Uuid>() {
                    if let Ok(user) =
                        get_arbitrary_org_owner_from_dataset_id(dataset_id, pool.clone()).await
                    {
                        return Ok((Some(user), None));
                    }
                }
            }
        }

        let get_user_from_api_key_span =
            tx.start_child("get_user_from_api_key", "Getting user from api key");
        //TODO: Cache the api key in redis
        if let Ok((user, api_key)) =
            get_user_from_api_key_query(authen_header.as_str(), pool.clone()).await
        {
            return Ok((Some(user), Some(api_key)));
        }

        get_user_from_api_key_span.finish();
    }

    //TODO: Add path scoped api key using the path field of `HTTPRequest` struct

    Ok((None, None))
}

pub struct AuthMiddlewareFactory;

impl<S, B> Transform<S, ServiceRequest> for AuthMiddlewareFactory
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthenticationMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthenticationMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub fn get_role_for_org(user: &SlimUser, org_id: &uuid::Uuid) -> Option<UserRole> {
    user.user_orgs
        .iter()
        .find(|org_conn| org_conn.organization_id == *org_id)
        .map(|org_conn| UserRole::from(org_conn.role))
}

pub fn verify_owner(user: &OwnerOnly, org_id: &uuid::Uuid) -> bool {
    if let Some(user_role) = get_role_for_org(&user.0, org_id) {
        return user_role >= UserRole::Owner;
    }

    false
}

pub fn verify_admin(user: &AdminOnly, org_id: &uuid::Uuid) -> bool {
    if let Some(user_role) = get_role_for_org(&user.0, org_id) {
        return user_role >= UserRole::Admin;
    }

    false
}

pub fn verify_member(user: &LoggedUser, org_id: &uuid::Uuid) -> bool {
    if let Some(user_role) = get_role_for_org(user, org_id) {
        return user_role >= UserRole::User;
    }

    false
}
