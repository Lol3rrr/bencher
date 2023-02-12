#![cfg(feature = "plus")]

use bencher_billing::Biller;
use bencher_json::system::config::{JsonBilling, JsonPlus};
use bencher_license::Licensor;
use url::Url;

use crate::ApiError;

pub struct Plus {
    pub biller: Option<Biller>,
    pub licensor: Licensor,
}

impl Plus {
    pub fn new(endpoint: &Url, plus: Option<JsonPlus>) -> Result<Plus, ApiError> {
        let Some(plus) = plus else {
            return Ok(Self {
                biller: None,
                licensor: Licensor::self_hosted().map_err(ApiError::License)?,
            });
        };

        // The only endpoint that should be using the `plus` section is https://bencher.dev
        if !bencher_plus::is_bencher_dev(endpoint) {
            return Err(ApiError::BencherPlus(endpoint.clone()));
        }

        let JsonBilling { secret_key } = plus.billing;
        let biller = Some(Biller::new(secret_key));

        let licensor = Licensor::bencher_cloud(plus.license_pem).map_err(ApiError::License)?;

        Ok(Self { biller, licensor })
    }
}