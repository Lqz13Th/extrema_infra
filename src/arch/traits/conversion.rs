use crate::errors::{InfraError, InfraResult};

pub trait IntoWsData {
    type Output;
    fn into_ws(self) -> Self::Output;
}

pub trait IntoInfraData<T> {
    fn into_vec(self) -> InfraResult<Vec<T>>;

    fn into_one(self) -> InfraResult<T>;
}

/// The single element of a list payload; none or several is an error.
pub fn exactly_one<T>(items: Vec<T>) -> InfraResult<T> {
    let mut items = items.into_iter();
    match (items.next(), items.next()) {
        (Some(item), None) => Ok(item),
        (None, _) => Err(InfraError::ApiCliError(
            "expected exactly one item, got none".to_string(),
        )),
        (Some(_), Some(_)) => Err(InfraError::ApiCliError(
            "expected exactly one item, got several".to_string(),
        )),
    }
}
