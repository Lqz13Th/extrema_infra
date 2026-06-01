use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned, de::Error as DeError};
use serde_json::Value;
use tracing::warn;

use crate::arch::traits::conversion::IntoInfraVec;
use crate::errors::{InfraError, InfraResult};

#[derive(Clone, Debug, Serialize)]
pub enum RestResBinance<T> {
    CodeMsg(BinanceCodeMsg),
    Data(Vec<T>),
    Object(T),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BinanceCodeMsg {
    pub code: i64,
    pub msg: String,
}

impl<T> IntoInfraVec<T> for RestResBinance<T> {
    fn into_vec(self) -> InfraResult<Vec<T>> {
        match self {
            Self::Data(v) => Ok(v),
            Self::Object(o) => Ok(vec![o]),
            Self::CodeMsg(BinanceCodeMsg { code, msg }) => {
                warn!("Binance REST error {}: {}", code, msg);
                Err(InfraError::ApiCliError(format!(
                    "Binance REST error (code={}): {}",
                    code, msg
                )))
            },
        }
    }
}

impl<'de, T> Deserialize<'de> for RestResBinance<T>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;

        match value {
            Value::Array(_) => serde_json::from_value(value)
                .map(Self::Data)
                .map_err(D::Error::custom),
            Value::Object(_) => {
                let code_msg = value
                    .get("code")
                    .and_then(Value::as_i64)
                    .zip(value.get("msg").and_then(Value::as_str));

                if let Some((code, _)) = code_msg
                    && code != 200
                {
                    return serde_json::from_value(value)
                        .map(Self::CodeMsg)
                        .map_err(D::Error::custom);
                }

                serde_json::from_value(value)
                    .map(Self::Object)
                    .map_err(D::Error::custom)
            },
            other => serde_json::from_value(other)
                .map(Self::Object)
                .map_err(D::Error::custom),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, Deserialize, PartialEq)]
    struct CodeMsgAck {
        code: i64,
        msg: String,
    }

    #[test]
    fn parses_success_code_msg_as_object() {
        let res: RestResBinance<CodeMsgAck> =
            serde_json::from_str(r#"{"code":200,"msg":"success"}"#).unwrap();

        let data = res.into_vec().unwrap();
        assert_eq!(
            data,
            vec![CodeMsgAck {
                code: 200,
                msg: "success".into()
            }]
        );
    }

    #[test]
    fn parses_non_success_code_msg_as_error() {
        let res: RestResBinance<CodeMsgAck> =
            serde_json::from_str(r#"{"code":-4061,"msg":"position side mismatch"}"#).unwrap();

        assert!(matches!(
            res,
            RestResBinance::CodeMsg(BinanceCodeMsg { code: -4061, .. })
        ));
    }

    #[test]
    fn does_not_swallow_code_msg_error_as_value() {
        let res: RestResBinance<Value> =
            serde_json::from_str(r#"{"code":-4061,"msg":"position side mismatch"}"#).unwrap();

        assert!(res.into_vec().is_err());
    }

    #[test]
    fn parses_array_as_data() {
        let res: RestResBinance<CodeMsgAck> =
            serde_json::from_str(r#"[{"code":200,"msg":"first"},{"code":200,"msg":"second"}]"#)
                .unwrap();

        let data = res.into_vec().unwrap();
        assert_eq!(
            data,
            vec![
                CodeMsgAck {
                    code: 200,
                    msg: "first".into()
                },
                CodeMsgAck {
                    code: 200,
                    msg: "second".into()
                }
            ]
        );
    }
}
