//! RAG-related primitives.

use candle_core::Device;
use hugface::GenEmbeddings;

use crate::error::Error;

mod hugface;


/// Implementations.
pub enum Type {
    /// Hugging Face.
    HuggingFace,
}

/// Embedding generation.
pub trait Embedding {
    /// Return the embeddings for the prompt.
    fn get_embeddings(&mut self, prompt: &str) -> Result<Vec<f32>, Error>;
}

/// Return embedding generator.
pub fn get_embedding(t: Type) -> Result<Box<dyn Embedding>, Error> {
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