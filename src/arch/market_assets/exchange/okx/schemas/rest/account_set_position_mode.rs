use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestAccountSetPositionModeOkx {
    pub posMode: String,
}
