use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};

use crate::{schema, schema::branch_version as branch_version_table, util::query::fn_get};

use super::{branch::BranchId, version::VersionId};

crate::util::typed_id::typed_id!(BranchVersionId);

#[derive(diesel::Queryable)]
pub struct QueryBranchVersion {
    pub id: BranchVersionId,
    pub branch_id: BranchId,
    pub version_id: VersionId,
}

impl QueryBranchVersion {
    fn_get!(branch_version);
}

#[derive(Debug, diesel::Insertable)]
#[diesel(table_name = branch_version_table)]
pub struct InsertBranchVersion {
    pub branch_id: BranchId,
    pub version_id: VersionId,
}
