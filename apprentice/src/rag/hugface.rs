use candle_core::{Device, Tensor};
use hf_hub::{api::sync::Api, Repo, RepoType};
use tokenizers::Tokenizer;
use candle_transformers::models::bert::{BertModel, Config, HiddenAct, DTYPE};
use candle_nn::VarBuilder;

use crate::error::AppError;

use super::Embedding;


/// Embeddings generator.
pub struct GenEmbeddings {
    model: BertModel,
    tokenizer: Tokenizer,
}

impl GenEmbeddings {
    /// Create e new instance.
    pub(super) fn new(model_id: String, 
        revision: String,
        use_pth: bool,
        device: Device,
        approximate_gelu: bool) -> Result<Self, AppError> 
    {
        let repo = Repo::with_revision(model_id, RepoType::Model, revision);
        let (config_filename, tokenizer_filename, weights_filename) = {

            let api = Api::new()?;
            let api = api.repo(repo);
            let config = api.get("config.json")?;
            let tokenizer = api.get("tokenizer.json")?;
            let weights = if use_pth {
                api.get("pytorch_model.bin")?
            } else {
                api.get("model.safetensors")?
            };
            (config, tokenizer, weights)
        };

        let config = std::fs::read_to_string(config_filename.clone())
            .map_err(|e| AppError::Error(format!("Failed to load {}: {}", config_filename.to_string_lossy(), e)))?;
        let mut config: Config = serde_json::from_str(&config)
            .map_err(|e| AppError::Error(format!("Failed to parse json from {}: {}", config_filename.to_string_lossy(), e)))?;
        let tokenizer = Tokenizer::from_file(tokenizer_filename.clone())
            .map_err(|e| AppError::Error(format!("Failed to load tokenizer from {}: {}", tokenizer_filename.to_string_lossy(), e)))?;

        let vb = if use_pth {
            VarBuilder::from_pth(&weights_filename, DTYPE, &device)?
        } else {
            unsafe { VarBuilder::from_mmaped_safetensors(&[weights_filename], DTYPE, &device)? }
        };

        if approximate_gelu {
            config.hidden_act = HiddenAct::GeluApproximate;
        }
        let model = BertModel::load(vb, &config)?;

        Ok(GenEmbeddings {
            model,
            tokenizer,
        })
    }   

    fn normalize_l2(v: &Tensor) -> Result<Tensor, AppError> {
        Ok(v.broadcast_div(&v.sqr()?.sum_keepdim(1)?.sqrt()?)?)
    }
}

impl Embedding for GenEmbeddings {

    fn get_embeddings(&mut self, prompt: &str) -> Result<Vec<f32>, AppError>  {

        let device = &self.model.device;

        let tokenizer = self.tokenizer
            .with_padding(None)
            .with_truncation(None)
            .map_err(|e| AppError::Error(format!("tokenizer build error: {}", e)))?;

        let tokens = tokenizer
            .encode(prompt, true)
            .map_err(|e| AppError::Error(format!("tokenization error: {}", e)))?
            .get_ids()
            .to_vec();

        let token_ids = Tensor::new(&tokens[..], device)?.unsqueeze(0)?;
        let token_type_ids = token_ids.zeros_like()?;

        let embeddings =self.model.forward(&token_ids, &token_type_ids, None)?;

        let (_n_sentence, n_tokens, _hidden_size) = embeddings.dims3()?;
        let embeddings = (embeddings.sum(1)? / (n_tokens as f64))?;
        let embeddings = Self::normalize_l2(&embeddings)?;

        Ok(embeddings.squeeze(0)?.to_vec1::<f32>()?)
    }
}