use std::sync::Arc;

use bencher_json::{JsonNewReport, JsonReport, ResourceId};
use bencher_rbac::project::Permission;
use diesel::{
    expression_methods::BoolExpressionMethods, ExpressionMethods, JoinOnDsl, QueryDsl, RunQueryDsl,
};
use dropshot::{endpoint, HttpError, Path, RequestContext, TypedBody};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    endpoints::{
        endpoint::{response_accepted, response_ok, ResponseAccepted, ResponseOk},
        Endpoint, Method,
    },
    error::api_error,
    model::project::{
        metrics::Metrics,
        report::{InsertReport, QueryReport},
        version::InsertVersion,
        QueryProject,
    },
    model::user::auth::AuthUser,
    schema,
    util::{
        cors::{get_cors, CorsResponse},
        error::into_json,
        map_http_error,
        same_project::SameProject,
        Context,
    },
    ApiError,
};

use super::Resource;

const REPORT_RESOURCE: Resource = Resource::Report;

#[derive(Deserialize, JsonSchema)]
pub struct GetDirParams {
    pub project: ResourceId,
}

#[endpoint {
    method = OPTIONS,
    path =  "/v0/projects/{project}/reports",
    tags = ["projects", "reports"]
}]
pub async fn dir_options(
    _rqctx: Arc<RequestContext<Context>>,
    _path_params: Path<GetDirParams>,
) -> Result<CorsResponse, HttpError> {
    Ok(get_cors::<Context>())
}

#[endpoint {
    method = GET,
    path =  "/v0/projects/{project}/reports",
    tags = ["projects", "reports"]
}]
pub async fn get_ls(
    rqctx: Arc<RequestContext<Context>>,
    path_params: Path<GetDirParams>,
) -> Result<ResponseOk<Vec<JsonReport>>, HttpError> {
    let auth_user = AuthUser::new(&rqctx).await?;
    let endpoint = Endpoint::new(REPORT_RESOURCE, Method::GetLs);

    let json = get_ls_inner(
        rqctx.context(),
        &auth_user,
        path_params.into_inner(),
        endpoint,
    )
    .await
    .map_err(|e| endpoint.err(e))?;

    response_ok!(endpoint, json)
}

async fn get_ls_inner(
    context: &Context,
    auth_user: &AuthUser,
    path_params: GetDirParams,
    endpoint: Endpoint,
) -> Result<Vec<JsonReport>, ApiError> {
    let api_context = &mut *context.lock().await;
    let query_project = QueryProject::is_allowed_resource_id(
        api_context,
        &path_params.project,
        auth_user,
        Permission::View,
    )?;
    let conn = &mut api_context.database;

    Ok(schema::report::table
        .left_join(schema::testbed::table.on(schema::report::testbed_id.eq(schema::testbed::id)))
        .filter(schema::testbed::project_id.eq(query_project.id))
        .select((
            schema::report::id,
            schema::report::uuid,
            schema::report::user_id,
            schema::report::version_id,
            schema::report::testbed_id,
            schema::report::adapter,
            schema::report::start_time,
            schema::report::end_time,
        ))
        .order(schema::report::start_time.desc())
        .load::<QueryReport>(conn)
        .map_err(api_error!())?
        .into_iter()
        .filter_map(into_json!(endpoint, conn))
        .collect())
}

#[endpoint {
    method = POST,
    path =  "/v0/projects/{project}/reports",
    tags = ["projects", "reports"]
}]
// For simplicity, his query makes the assumption that all posts are perfectly
// chronological. That is, a report will never be posted for X after Y has
// already been submitted when X really happened before Y. For implementing git
// bisect more complex logic will be required.
pub async fn post(
    rqctx: Arc<RequestContext<Context>>,
    path_params: Path<GetDirParams>,
    body: TypedBody<JsonNewReport>,
) -> Result<ResponseAccepted<JsonReport>, HttpError> {
    let auth_user = AuthUser::new(&rqctx).await?;
    let endpoint = Endpoint::new(REPORT_RESOURCE, Method::Post);

    let json = post_inner(
        rqctx.context(),
        path_params.into_inner(),
        body.into_inner(),
        &auth_user,
    )
    .await
    .map_err(|e| endpoint.err(e))?;

    response_accepted!(endpoint, json)
}

async fn post_inner(
    context: &Context,
    path_params: GetDirParams,
    json_report: JsonNewReport,
    auth_user: &AuthUser,
) -> Result<JsonReport, ApiError> {
    let api_context = &mut *context.lock().await;
    let conn = &mut api_context.database;

    // Verify that the branch and testbed are part of the same project
    let SameProject {
        project_id,
        branch_id,
        testbed_id,
    } = SameProject::validate(
        conn,
        &path_params.project,
        json_report.branch,
        json_report.testbed,
    )?;

    // Verify that the user is allowed
    QueryProject::is_allowed_id(api_context, project_id, auth_user, Permission::Create)?;
    let conn = &mut api_context.database;

    // If there is a hash then try to see if there is already a code version for
    // this branch with that particular hash.
    // Otherwise, create a new code version for this branch with/without the hash.
    let version_id = if let Some(hash) = &json_report.hash {
        if let Ok(version_id) = schema::version::table
            .filter(
                schema::version::branch_id
                    .eq(branch_id)
                    .and(schema::version::hash.eq(hash)),
            )
            .select(schema::version::id)
            .first::<i32>(conn)
        {
            version_id
        } else {
            InsertVersion::increment(conn, branch_id, Some(hash.clone()))?
        }
    } else {
        InsertVersion::increment(conn, branch_id, None)?
    };

    // Create a new report and add it to the database
    let insert_report = InsertReport::from_json(auth_user.id, version_id, testbed_id, &json_report);

    diesel::insert_into(schema::report::table)
        .values(&insert_report)
        .execute(conn)
        .map_err(map_http_error!("Failed to create report."))?;

    let query_report = schema::report::table
        .filter(schema::report::uuid.eq(&insert_report.uuid))
        .first::<QueryReport>(conn)
        .map_err(map_http_error!("Failed to create report."))?;

    // Metrics is used to add benchmarks, perf metrics, and alerts.
    let mut metrics = Metrics::new(
        conn,
        project_id,
        branch_id,
        testbed_id,
        query_report.id,
        json_report.benchmarks.clone(),
    )?;

    for (index, benchmark) in json_report.benchmarks.inner.into_iter().enumerate() {
        for (benchmark_name, json_metrics) in benchmark.inner {
            metrics.benchmark(conn, index as i32, &benchmark_name, json_metrics)?;
        }
    }

    query_report.into_json(conn)
}

#[derive(Deserialize, JsonSchema)]
pub struct GetOneParams {
    pub project: ResourceId,
    pub report_uuid: Uuid,
}

#[endpoint {
    method = OPTIONS,
    path =  "/v0/projects/{project}/reports/{report_uuid}",
    tags = ["projects", "reports"]
}]
pub async fn one_options(
    _rqctx: Arc<RequestContext<Context>>,
    _path_params: Path<GetOneParams>,
) -> Result<CorsResponse, HttpError> {
    Ok(get_cors::<Context>())
}

#[endpoint {
    method = GET,
    path =  "/v0/projects/{project}/reports/{report_uuid}",
    tags = ["projects", "reports"]
}]
pub async fn get_one(
    rqctx: Arc<RequestContext<Context>>,
    path_params: Path<GetOneParams>,
) -> Result<ResponseOk<JsonReport>, HttpError> {
    let auth_user = AuthUser::new(&rqctx).await?;
    let endpoint = Endpoint::new(REPORT_RESOURCE, Method::GetOne);

    let json = get_one_inner(rqctx.context(), path_params.into_inner(), &auth_user)
        .await
        .map_err(|e| endpoint.err(e))?;

    response_ok!(endpoint, json)
}

async fn get_one_inner(
    context: &Context,
    path_params: GetOneParams,
    auth_user: &AuthUser,
) -> Result<JsonReport, ApiError> {
    let api_context = &mut *context.lock().await;
    let query_project = QueryProject::is_allowed_resource_id(
        api_context,
        &path_params.project,
        auth_user,
        Permission::View,
    )?;
    let conn = &mut api_context.database;

    schema::report::table
        .left_join(schema::testbed::table.on(schema::report::testbed_id.eq(schema::testbed::id)))
        .filter(
            schema::testbed::project_id
                .eq(query_project.id)
                .and(schema::report::uuid.eq(path_params.report_uuid.to_string())),
        )
        .select((
            schema::report::id,
            schema::report::uuid,
            schema::report::user_id,
            schema::report::version_id,
            schema::report::testbed_id,
            schema::report::adapter,
            schema::report::start_time,
            schema::report::end_time,
        ))
        .first::<QueryReport>(conn)
        .map_err(api_error!())?
        .into_json(conn)
}