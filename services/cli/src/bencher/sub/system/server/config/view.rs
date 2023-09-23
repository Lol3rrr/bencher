use std::convert::TryFrom;

use async_trait::async_trait;
use bencher_json::JsonConfig;

use crate::{
    bencher::{backend::Backend, sub::SubCmd},
    parser::system::server::CliConfigView,
    CliError,
};

#[derive(Debug, Clone)]
pub struct View {
    pub backend: Backend,
}

impl TryFrom<CliConfigView> for View {
    type Error = CliError;

    fn try_from(view: CliConfigView) -> Result<Self, Self::Error> {
        let CliConfigView { backend } = view;
        Ok(Self {
            backend: backend.try_into()?,
        })
    }
}

#[async_trait]
impl SubCmd for View {
    async fn exec(&self) -> Result<(), CliError> {
        let _json: JsonConfig = self
            .backend
            .send_with(
                |client| async move { client.server_config_get().send().await },
                true,
            )
            .await?;
        Ok(())
    }
}
