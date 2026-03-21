use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use once_cell::sync::OnceCell;

static EMBEDDER: OnceCell<TextEmbedding> = OnceCell::new();

pub struct Embedder;

impl Embedder {
    pub fn new() -> Result<Self> {
        EMBEDDER.get_or_try_init(|| {
            TextEmbedding::try_new(
                InitOptions::new(EmbeddingModel::AllMiniLML6V2)
                    .with_show_download_progress(false),
            )
        })?;
        Ok(Self)
    }

    /// Embed a batch of text strings. Returns Vec<Vec<f32>>, one per input.
    pub fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let model = EMBEDDER.get().unwrap();
        let embeddings = model.embed(texts.to_vec(), None)?;
        Ok(embeddings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embed_returns_384_dims() {
        let embedder = Embedder::new().unwrap();
        let vecs = embedder.embed(&["hello world"]).unwrap();
        assert_eq!(vecs.len(), 1);
        assert_eq!(vecs[0].len(), 384);
    }

    #[test]
    fn test_embed_batch() {
        let embedder = Embedder::new().unwrap();
        let vecs = embedder.embed(&["first", "second", "third"]).unwrap();
        assert_eq!(vecs.len(), 3);
    }
}
