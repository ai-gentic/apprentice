use candle_core::Device;
use hugface::GenEmbeddings;

use crate::error::AppError;

pub mod hugface;


/// Implementations.
pub enum Type {
    HuggingFace,
}

/// Embedding generation.
pub trait Embedding {
    /// Return the embeddings for the prompt.
    fn get_embeddings(&mut self, prompt: &str) -> Result<Vec<f32>, AppError>;
}

/// Return embedding generator.
pub fn get_embedding(t: Type) -> Result<Box<dyn Embedding>, AppError> {
    match t {
        Type::HuggingFace => Ok(Box::new(GenEmbeddings::new(
            "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            "refs/pr/21".to_string(),
            true,
            Device::Cpu,
            false
        )?))
    }
}