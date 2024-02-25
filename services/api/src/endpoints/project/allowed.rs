use bencher_json::{project::ProjectPermission, JsonAllowed, ResourceId};
use dropshot::{endpoint, HttpError, Path, RequestContext};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    conn,
    context::ApiContext,
    endpoints::{
        endpoint::{CorsResponse, Get, ResponseOk},
        Endpoint,
    },
    model::{
        project::QueryProject,
        user::auth::{AuthUser, BearerToken},
    },
};

#[derive(Deserialize, JsonSchema)]
pub struct ProjAllowedParams {
    pub project: ResourceId,
    pub permission: ProjectPermission,
}

#[allow(clippy::unused_async)]
#[endpoint {
    method = OPTIONS,
    path =  "/v0/projects/{project}/allowed/{permission}",
    tags = ["projects", "allowed"]
}]
pub async fn proj_allowed_options(
    _rqctx: RequestContext<ApiContext>,
    _path_params: Path<ProjAllowedParams>,
) -> Result<CorsResponse, HttpError> {
    Ok(Endpoint::cors(&[Get.into()]))
}

#[endpoint {
    method = GET,
    path = "/v0/projects/{project}/allowed/{permission}",
    tags = ["projects", "allowed"]
}]
pub async fn proj_allowed_get(
    rqctx: RequestContext<ApiContext>,
    bearer_token: BearerToken,
    path_params: Path<ProjAllowedParams>,
) -> Result<ResponseOk<JsonAllowed>, HttpError> {
    let auth_user = AuthUser::from_token(rqctx.context(), bearer_token).await?;
    let json = get_inner(rqctx.context(), path_params.into_inner(), &auth_user).await?;
    Ok(Get::auth_response_ok(json))
}

async fn get_inner(
    context: &ApiContext,
    path_params: ProjAllowedParams,
    auth_user: &AuthUser,
) -> Result<JsonAllowed, HttpError> {
    Ok(JsonAllowed {
        allowed: QueryProject::is_allowed(
            conn!(context),
            &context.rbac,
            &path_params.project,
            auth_user,
            crate::model::project::project_role::Permission::from(path_params.permission).into(),
        )
        .is_ok(),
    })
}
