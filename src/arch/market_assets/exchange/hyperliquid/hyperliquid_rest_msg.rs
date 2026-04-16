use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::arch::traits::conversion::IntoInfraVec;
use crate::errors::{InfraError, InfraResult};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RestResHyperliquid<T> {
    Null,
    Order {
        status: String,
        order: T,
    },
    Exchange {
        status: String,
        response: Option<RestResHyperliquidPayload<T>>,
    },
    StatusOnly {
        status: String,
    },
    Data(Vec<T>),
    Object(T),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RestResHyperliquidPayload<T> {
    Typed {
        #[serde(rename = "type")]
        kind: String,
        data: Option<T>,
    },
    Data(Vec<T>),
    Object(T),
}

impl<T> IntoInfraVec<T> for RestResHyperliquid<T>
where
    T: std::fmt::Debug,
{
    fn into_vec(self) -> InfraResult<Vec<T>> {
        match self {
            Self::Null => Ok(vec![]),
            Self::Order { status, order } => {
                if status == "order" {
                    Ok(vec![order])
                } else {
                    warn!("Hyperliquid REST error {}: order payload", status);
                    Err(InfraError::ApiCliError(format!(
                        "Hyperliquid REST error (status={}): order payload",
                        status
                    )))
                }
            },
            Self::Exchange { status, response } => {
                if status != "ok" {
                    warn!("Hyperliquid REST error {}: {:?}", status, response);
                    return Err(InfraError::ApiCliError(format!(
                        "Hyperliquid REST error (status={}): {:?}",
                        status, response
                    )));
                }

                match response {
                    Some(RestResHyperliquidPayload::Typed {
                             data: Some(data), ..
                         }) => Ok(vec![data]),
                    Some(RestResHyperliquidPayload::Typed { data: None, .. }) => Ok(vec![]),
                    Some(RestResHyperliquidPayload::Data(v)) => Ok(v),
                    Some(RestResHyperliquidPayload::Object(o)) => Ok(vec![o]),
                    None => Ok(vec![]),
                }
            },
            Self::StatusOnly { status } => {
                if status == "unknownOid" {
                    Ok(vec![])
                } else {
                    warn!("Hyperliquid REST error {}: None", status);
                    Err(InfraError::ApiCliError(format!(
                        "Hyperliquid REST error (status={}): None",
                        status
                    )))
                }
            },
            Self::Data(v) => Ok(v),
            Self::Object(o) => Ok(vec![o]),
        }
    }
}
