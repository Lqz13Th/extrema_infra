use std::sync::Arc;

use crate::arch::{
    market_assets::market_core::Market,
    strategy_base::handler::{events::InfraMsg, task_channel::TaskEvent},
    task_execution::task_general::LogLevel,
    traits::{
        conversion::IntoWsData,
        market_lob::{WsDecoders, WsFrameRunner},
    },
};

use super::{WsStream, WsTaskRunner};

impl WsTaskRunner {
    pub(super) async fn ws_channel_custom<D: WsDecoders>(
        &mut self,
        ws_stream: &mut WsStream,
        decoders: &D,
    ) {
        let ws_info = Arc::clone(&self.ws_info);
        let Market::Custom(name) = &ws_info.market else {
            return;
        };
        let Some(index) = decoders
            .markets()
            .iter()
            .position(|market| *market == name.as_ref())
        else {
            self.log(
                LogLevel::Error,
                &format!("No websocket decoder registered for custom market: {name}"),
            );
            return;
        };

        decoders
            .ws_channel_at(
                index,
                &ws_info.ws_channel,
                CustomFrames {
                    task: self,
                    ws_stream,
                },
            )
            .await;
    }
}

struct CustomFrames<'a> {
    task: &'a mut WsTaskRunner,
    ws_stream: &'a mut WsStream,
}

impl WsFrameRunner for CustomFrames<'_> {
    async fn ws_loop<WsData, IntoEvent, Decode>(self, into_event: IntoEvent, decode: Decode)
    where
        WsData: IntoWsData + Send + 'static,
        WsData::Output: Send + Sync + 'static,
        IntoEvent: Fn(InfraMsg<WsData::Output>) -> TaskEvent + Copy + Send,
        Decode: Fn(&[u8]) -> serde_json::Result<WsData> + Copy + Send,
    {
        WsTaskRunner::ws_loop(self.task, into_event, self.ws_stream, decode).await;
    }
}
