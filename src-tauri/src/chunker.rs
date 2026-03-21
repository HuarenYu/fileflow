/// Splits text into representative chunks using a token-approximate sliding window.
/// Approximates tokens as whitespace-separated words (1 word ≈ 1.3 tokens).
/// Returns at most 3 chunks: first, middle, last.
pub fn chunk_text(text: &str, chunk_size_tokens: usize, overlap_tokens: usize) -> Vec<String> {
    if text.trim().is_empty() {
        return vec![];
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    let words_per_chunk = (chunk_size_tokens as f32 / 1.3) as usize;
    let words_overlap = (overlap_tokens as f32 / 1.3) as usize;
    let step = words_per_chunk.saturating_sub(words_overlap).max(1);

    let mut chunks: Vec<String> = Vec::new();
    let mut start = 0;
    while start < words.len() {
        let end = (start + words_per_chunk).min(words.len());
        chunks.push(words[start..end].join(" "));
        if end == words.len() {
            break;
        }
        start += step;
    }

    if chunks.len() <= 3 {
        return chunks;
    }
    // select first, middle, last
    let mid = chunks.len() / 2;
    vec![
        chunks[0].clone(),
        chunks[mid].clone(),
        chunks[chunks.len() - 1].clone(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_text_single_chunk() {
        let chunks = chunk_text("hello world", 400, 50);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "hello world");
    }

    #[test]
    fn test_representative_chunks_capped_at_3() {
        // simulate long text (>1200 tokens)
        let words: Vec<&str> = vec!["word"; 2000];
        let text = words.join(" ");
        let chunks = chunk_text(&text, 400, 50);
        assert!(chunks.len() <= 3);
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_empty_text_returns_empty() {
        let chunks = chunk_text("", 400, 50);
        assert!(chunks.is_empty());
    }
}
