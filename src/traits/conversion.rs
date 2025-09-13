
pub trait IntoWsData {
    type Output;
    fn into_ws(self) -> Self::Output;
}

