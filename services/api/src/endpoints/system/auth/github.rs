#![cfg(feature = "plus")]

use bencher_json::JsonAuth;
use bencher_json::JsonAuthUser;
use bencher_json::JsonLogin;

use bencher_json::system::auth::JsonOAuth;
use bencher_json::JsonSignup;
use diesel::sql_types::Json;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use dropshot::{endpoint, HttpError, RequestContext, TypedBody};
use http::StatusCode;
use slog::Logger;

use crate::endpoints::endpoint::CorsResponse;
use crate::endpoints::endpoint::Get;
use crate::endpoints::endpoint::Post;
use crate::endpoints::endpoint::ResponseAccepted;
use crate::endpoints::Endpoint;

use crate::endpoints::endpoint::ResponseOk;
use crate::error::forbidden_error;
use crate::error::issue_error;
use crate::error::payment_required_error;
use crate::error::resource_conflict_err;
use crate::error::resource_not_found_err;
use crate::error::unauthorized_error;
use crate::model::organization::plan::PlanKind;
use crate::model::organization::QueryOrganization;
use crate::model::user::InsertUser;
use crate::{
    context::{ApiContext, Body, ButtonBody, Message},
    model::organization::organization_role::InsertOrganizationRole,
    model::user::QueryUser,
    schema,
};

use super::AUTH_TOKEN_TTL;
use super::TOKEN_ARG;

#[allow(clippy::unused_async)]
#[endpoint {
    method = OPTIONS,
    path =  "/v0/auth/github",
    tags = ["auth"]
}]
pub async fn auth_github_options(
    _rqctx: RequestContext<ApiContext>,
) -> Result<CorsResponse, HttpError> {
    Ok(Endpoint::cors(&[Get.into(), Post.into()]))
}

#[endpoint {
    method = POST,
    path = "/v0/auth/github",
    tags = ["auth"]
}]
pub async fn auth_github_post(
    rqctx: RequestContext<ApiContext>,
    body: TypedBody<JsonOAuth>,
) -> Result<ResponseAccepted<JsonAuthUser>, HttpError> {
    let json = post_inner(&rqctx.log, rqctx.context(), body.into_inner()).await?;
    Ok(Post::pub_response_accepted(json))
}

async fn post_inner(
    log: &Logger,
    context: &ApiContext,
    json_oauth: JsonOAuth,
) -> Result<JsonAuthUser, HttpError> {
    let Some(github) = &context.github else {
        let err = "GitHub OAuth2 is not configured";
        slog::warn!(log, "{err}");
        return Err(payment_required_error(err));
    };
    let conn = &mut *context.conn().await;

    let github_user = github
        .oauth_user(json_oauth.code)
        .await
        .map_err(unauthorized_error)?;
    let email = github_user
        .email
        .ok_or_else(|| unauthorized_error("GitHub OAuth2 user does not have an email address"))?
        .parse()
        .map_err(unauthorized_error)?;

    let query_user = QueryUser::get_id_from_email(conn, &email);

    if query_user.is_ok() {
    } else {
        // If not on Bencher Cloud, then users must be invited to use OAuth2
        if !context.is_bencher_cloud() {
            let Some(invite) = &json_oauth.invite else {
                return Err(payment_required_error(
                    "You must be invited to join a Bencher Self-Hosted instance with GitHub OAuth2",
                ));
            };

            let claims = context
                .token_key
                .validate_invite(invite)
                .map_err(unauthorized_error)?;

            let query_organization = schema::organization::table
                .filter(schema::organization::uuid.eq(&claims.org.uuid))
                .first::<QueryOrganization>(conn)
                .map_err(resource_not_found_err!(Organization, claims))?;

            // Make sure org has a valid Bencher Plus plan
            PlanKind::new_for_organization(
                conn,
                context.biller.as_ref(),
                &context.licensor,
                &query_organization,
            )
            .await?;
        }

        let json_signup = JsonSignup {
            name: github_user.login.parse().map_err(unauthorized_error)?,
            slug: None,
            email,
            i_agree: true,
            invite: json_oauth.invite.clone(),
            plan: None,
        };

        let invited = json_signup.invite.is_some();
        let insert_user = InsertUser::insert_from_json(conn, &context.token_key, &json_signup)?;

        insert_user.notify(log, conn, &context.messenger, &context.endpoint, invited)?;
    }

    todo!()

    // let query_user = schema::user::table
    //     .filter(schema::user::email.eq(json_login.email.as_ref()))
    //     .first::<QueryUser>(conn)
    //     .map_err(resource_not_found_err!(User, json_login))?;

    // // Check to see if the user account has been locked
    // if query_user.locked {
    //     return Err(forbidden_error(format!(
    //         "Your account ({json_login:?}) has been locked. Please contact support.",
    //     )));
    // }

    // #[cfg(feature = "plus")]
    // let plan = json_login.plan;

    // if let Some(invite) = &json_login.invite {
    //     let insert_org_role =
    //         InsertOrganizationRole::from_jwt(conn, &context.token_key, invite, query_user.id)?;

    //     diesel::insert_into(schema::organization_role::table)
    //         .values(&insert_org_role)
    //         .execute(conn)
    //         .map_err(resource_conflict_err!(OrganizationRole, insert_org_role))?;
    // }

    // let token = context
    //     .token_key
    //     .new_auth(json_login.email.clone(), AUTH_TOKEN_TTL)
    //     .map_err(|e| {
    //         issue_error(
    //             StatusCode::INTERNAL_SERVER_ERROR,
    //             "Failed to create auth JWT at login",
    //             &format!(
    //                 "Failed failed to create auth JWT ({json_login:?} | {AUTH_TOKEN_TTL}) at login"
    //             ),
    //             e,
    //         )
    //     })?;

    // let token_string = token.to_string();
    // let body = Body::Button(Box::new(ButtonBody {
    //     title: "Confirm Bencher Login".into(),
    //     preheader: "Click the provided link to login.".into(),
    //     greeting: format!("Ahoy {},", query_user.name),
    //     pre_body: "Please, click the button below or use the provided code to login to Bencher."
    //         .into(),
    //     button_text: "Confirm Login".into(),
    //     button_url: context
    //         .endpoint
    //         .clone()
    //         .join("/auth/confirm")
    //         .map(|mut url| {
    //             #[cfg(feature = "plus")]
    //             if let Some(plan) = plan {
    //                 url.query_pairs_mut()
    //                     .append_pair(super::PLAN_ARG, plan.as_ref());
    //             }
    //             url.query_pairs_mut().append_pair(TOKEN_ARG, &token_string);
    //             url.into()
    //         })
    //         .unwrap_or_default(),
    //     clipboard_text: "Confirmation Code".into(),
    //     clipboard_target: token_string,
    //     post_body: String::new(),
    //     closing: "See you soon,".into(),
    //     signature: "The Bencher Team".into(),
    //     settings_url: context
    //         .endpoint
    //         .clone()
    //         .join("/help")
    //         .map(Into::into)
    //         .unwrap_or_default(),
    // }));
    // let message = Message {
    //     to_name: Some(query_user.name.into()),
    //     to_email: query_user.email.into(),
    //     subject: Some("Confirm Bencher Login".into()),
    //     body: Some(body),
    // };
    // context.messenger.send(log, message);

    // Ok(JsonAuth {
    //     email: json_login.email,
    // })
}