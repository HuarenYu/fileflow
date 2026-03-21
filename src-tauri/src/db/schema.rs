use arrow_schema::{DataType, Field, Fields, Schema, TimeUnit};
use std::sync::Arc;

pub fn file_chunks_schema() -> Schema {
    Schema::new(Fields::from(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("file_id", DataType::Utf8, false),
        Field::new("path", DataType::Utf8, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("extension", DataType::Utf8, false),
        Field::new("size", DataType::Int64, false),
        Field::new("modified_at", DataType::Timestamp(TimeUnit::Millisecond, None), false),
        Field::new("category", DataType::Utf8, false),
        Field::new("user_category", DataType::Utf8, true),
        Field::new("chunk_index", DataType::Int32, false),
        Field::new("content_text", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 384),
            false,
        ),
        Field::new("thumbnail_path", DataType::Utf8, true),
        Field::new("indexed_at", DataType::Timestamp(TimeUnit::Millisecond, None), false),
        Field::new("deleted_at", DataType::Timestamp(TimeUnit::Millisecond, None), true),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_schema_has_required_fields() {
        let schema = file_chunks_schema();
        assert!(schema.field_with_name("id").is_ok());
        assert!(schema.field_with_name("file_id").is_ok());
        assert!(schema.field_with_name("vector").is_ok());
        assert!(schema.field_with_name("deleted_at").is_ok());
    }
}
