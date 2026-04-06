use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};
use tokio::{
    sync::{broadcast, oneshot},
    task,
    time::timeout,
};
use tract_onnx::prelude::{
    Framework, InferenceModelExt, IntoTensor, TValue, TypedModel, TypedRunnableModel,
    tract_ndarray::{ArrayD, IxDyn},
    tvec,
};

use crate::{
    arch::{
        strategy_base::handler::{alt_events::AltTensor, handler_core::InfraMsg},
        task_execution::task_general::LogLevel,
    },
    errors::{InfraError, InfraResult},
};

use super::AltTaskBuilder;

#[derive(Debug, Deserialize)]
struct OnnxRunnerConfig {
    model_path: String,
    output_index: Option<usize>,
    model_name: Option<String>,
}

impl OnnxRunnerConfig {
    fn load(config_path: &str) -> InfraResult<Self> {
        let config_path = PathBuf::from(config_path);

        if config_path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("onnx"))
        {
            return Ok(Self {
                model_name: config_path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(ToOwned::to_owned),
                model_path: config_path.to_string_lossy().into_owned(),
                output_index: None,
            });
        }

        let config_text = fs::read_to_string(&config_path)?;
        let mut config: Self = serde_json::from_str(&config_text)?;

        let model_path = PathBuf::from(&config.model_path);
        if model_path.is_relative() {
            let base_dir = config_path.parent().unwrap_or(Path::new("."));
            config.model_path = base_dir.join(model_path).to_string_lossy().into_owned();
        }

        if config.model_name.is_none() {
            config.model_name = config_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(ToOwned::to_owned);
        }

        Ok(config)
    }
}

#[derive(Debug)]
struct OnnxModelRunner {
    config: OnnxRunnerConfig,
    model: TypedRunnableModel<TypedModel>,
}

impl OnnxModelRunner {
    fn new(config_path: &str) -> InfraResult<Self> {
        let config = OnnxRunnerConfig::load(config_path)?;
        let model = tract_onnx::onnx()
            .model_for_path(&config.model_path)
            .map_err(|e| InfraError::Msg(format!("Failed to load ONNX model: {e}")))?
            .into_optimized()
            .map_err(|e| InfraError::Msg(format!("Failed to optimize ONNX model: {e}")))?
            .into_runnable()
            .map_err(|e| InfraError::Msg(format!("Failed to initialize ONNX model: {e}")))?;

        Ok(Self { config, model })
    }

    fn predict(&self, tensor: AltTensor) -> InfraResult<AltTensor> {
        let AltTensor {
            timestamp,
            data,
            shape,
            mut metadata,
        } = tensor;

        let expected_numel = shape_numel(&shape)?;
        if expected_numel != data.len() {
            return Err(InfraError::Msg(format!(
                "ONNX input shape/data mismatch: shape={shape:?}, expected_numel={expected_numel}, actual_numel={}",
                data.len()
            )));
        }

        let input = ArrayD::from_shape_vec(IxDyn(&shape), data).map_err(|e| {
            InfraError::Msg(format!(
                "Failed to build ONNX input tensor from AltTensor: {e}"
            ))
        })?;

        let mut outputs = self
            .model
            .run(tvec!(input.into_tensor().into()))
            .map_err(|e| InfraError::Msg(format!("ONNX inference failed: {e}")))?;

        if outputs.is_empty() {
            return Err(InfraError::Msg(
                "ONNX inference returned no outputs".to_string(),
            ));
        }

        let output_index = match self.config.output_index {
            Some(index) => index,
            None => select_default_output_index(&outputs).ok_or_else(|| {
                InfraError::Msg(format!(
                    "ONNX inference returned {} outputs, but none could be converted into AltTensor f32 data",
                    outputs.len()
                ))
            })?,
        };
        if output_index >= outputs.len() {
            return Err(InfraError::Msg(format!(
                "Configured ONNX output_index {} out of range, inference returned {} outputs",
                output_index,
                outputs.len()
            )));
        }

        let output_count = outputs.len();
        let output = outputs.remove(output_index);
        let (output_data, output_shape, output_dtype) = decode_output_to_f32(&output)?;

        metadata
            .entry("model_runner".to_string())
            .or_insert_with(|| "onnx".to_string());
        if let Some(model_name) = &self.config.model_name {
            metadata
                .entry("model_name".to_string())
                .or_insert_with(|| model_name.clone());
        }
        metadata
            .entry("output_index".to_string())
            .or_insert_with(|| output_index.to_string());
        metadata
            .entry("output_count".to_string())
            .or_insert_with(|| output_count.to_string());
        metadata
            .entry("output_dtype".to_string())
            .or_insert_with(|| output_dtype.to_string());

        Ok(AltTensor {
            timestamp,
            data: output_data,
            shape: output_shape,
            metadata,
        })
    }
}

struct OnnxWorkerRequest {
    tensor: AltTensor,
    response_tx: oneshot::Sender<InfraResult<AltTensor>>,
}

fn shape_numel(shape: &[usize]) -> InfraResult<usize> {
    shape.iter().try_fold(1usize, |acc, &dim| {
        acc.checked_mul(dim)
            .ok_or_else(|| InfraError::Msg(format!("Shape overflow for {shape:?}")))
    })
}

fn select_default_output_index(outputs: &[TValue]) -> Option<usize> {
    outputs
        .iter()
        .position(|output| {
            output.to_array_view::<f32>().is_ok() || output.to_array_view::<f64>().is_ok()
        })
        .or_else(|| {
            outputs
                .iter()
                .position(|output| decode_output_to_f32(output).is_ok())
        })
}

fn decode_output_to_f32(output: &TValue) -> InfraResult<(Vec<f32>, Vec<usize>, &'static str)> {
    if let Ok(view) = output.to_array_view::<f32>() {
        return Ok((view.iter().copied().collect(), view.shape().to_vec(), "f32"));
    }
    if let Ok(view) = output.to_array_view::<f64>() {
        return Ok((
            view.iter().map(|&value| value as f32).collect(),
            view.shape().to_vec(),
            "f64",
        ));
    }
    if let Ok(view) = output.to_array_view::<i64>() {
        return Ok((
            view.iter().map(|&value| value as f32).collect(),
            view.shape().to_vec(),
            "i64",
        ));
    }
    if let Ok(view) = output.to_array_view::<i32>() {
        return Ok((
            view.iter().map(|&value| value as f32).collect(),
            view.shape().to_vec(),
            "i32",
        ));
    }
    if let Ok(view) = output.to_array_view::<i16>() {
        return Ok((
            view.iter().map(|&value| value as f32).collect(),
            view.shape().to_vec(),
            "i16",
        ));
    }
    if let Ok(view) = output.to_array_view::<i8>() {
        return Ok((
            view.iter().map(|&value| value as f32).collect(),
            view.shape().to_vec(),
            "i8",
        ));
    }
    if let Ok(view) = output.to_array_view::<u64>() {
        return Ok((
            view.iter().map(|&value| value as f32).collect(),
            view.shape().to_vec(),
            "u64",
        ));
    }
    if let Ok(view) = output.to_array_view::<u32>() {
        return Ok((
            view.iter().map(|&value| value as f32).collect(),
            view.shape().to_vec(),
            "u32",
        ));
    }
    if let Ok(view) = output.to_array_view::<u16>() {
        return Ok((
            view.iter().map(|&value| value as f32).collect(),
            view.shape().to_vec(),
            "u16",
        ));
    }
    if let Ok(view) = output.to_array_view::<u8>() {
        return Ok((
            view.iter().map(|&value| value as f32).collect(),
            view.shape().to_vec(),
            "u8",
        ));
    }
    if let Ok(view) = output.to_array_view::<bool>() {
        return Ok((
            view.iter()
                .map(|&value| if value { 1.0 } else { 0.0 })
                .collect(),
            view.shape().to_vec(),
            "bool",
        ));
    }

    Err(InfraError::Msg(format!(
        "Unsupported ONNX output tensor type for AltTensor conversion: {:?}",
        output.datum_type()
    )))
}

fn onnx_worker_loop(runner: OnnxModelRunner, request_rx: mpsc::Receiver<OnnxWorkerRequest>) {
    while let Ok(request) = request_rx.recv() {
        let result = runner.predict(request.tensor);
        let _ = request.response_tx.send(result);
    }
}

impl AltTaskBuilder {
    pub(super) async fn model_preds_onnx(
        &mut self,
        tx: broadcast::Sender<InfraMsg<AltTensor>>,
        config_path: String,
    ) {
        self.log(
            LogLevel::Info,
            &format!("Loading ONNX runner config from {config_path}..."),
        );

        let runner = match OnnxModelRunner::new(&config_path) {
            Ok(runner) => runner,
            Err(e) => {
                self.log(
                    LogLevel::Error,
                    &format!("Failed to initialize ONNX model runner: {e}"),
                );
                return;
            },
        };

        self.log(
            LogLevel::Info,
            &format!("ONNX model initialized from {}.", runner.config.model_path),
        );

        let (request_tx, request_rx) = mpsc::channel::<OnnxWorkerRequest>();
        let worker_handle = task::spawn_blocking(move || onnx_worker_loop(runner, request_rx));
        let model_inference_timeout = Duration::from_secs(20);

        loop {
            let Some(tensor) = self.recv_feat_input().await else {
                break;
            };

            let (response_tx, response_rx) = oneshot::channel();
            if let Err(e) = request_tx.send(OnnxWorkerRequest {
                tensor,
                response_tx,
            }) {
                self.log(
                    LogLevel::Error,
                    &format!("Failed to send ONNX request to worker thread: {e}"),
                );
                break;
            }

            match timeout(model_inference_timeout, response_rx).await {
                Ok(Ok(Ok(matrix))) => self.emit_model_preds(&tx, matrix),
                Ok(Ok(Err(e))) => {
                    self.log(LogLevel::Error, &format!("ONNX inference error: {e}"));
                },
                Ok(Err(e)) => {
                    self.log(
                        LogLevel::Error,
                        &format!("ONNX worker response channel error: {e}"),
                    );
                    break;
                },
                Err(_) => {
                    self.log(
                        LogLevel::Warn,
                        "Model prediction TIMEOUT - skipping this tick",
                    );
                    continue;
                },
            };
        }

        drop(request_tx);
        if let Err(e) = worker_handle.await {
            self.log(LogLevel::Error, &format!("ONNX worker join error: {e}"));
        }
    }
}
