use crate::errors::InfraResult;

pub trait IntoWsData {
    type Output;
    fn into_ws(self) -> Self::Output;
}

pub trait IntoInfraVec<T> {
    fn into_vec(self) -> InfraResult<Vec<T>>;
}