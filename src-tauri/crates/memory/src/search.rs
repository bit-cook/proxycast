//! Semantic search using vector embeddings

use crate::models::{MemoryCategory, UnifiedMemory};
use rusqlite::{params, Connection};
use serde_json;

/// Calculate cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for i in 0..a.len() {
        dot_product += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denominator = norm_a.sqrt() * norm_b.sqrt();
    if denominator == 0.0 {
        return 0.0;
    }

    dot_product / denominator
}

/// Semantic search using vector similarity
///
/// # Parameters
///
/// * `db` - Database connection
/// * `query_embedding` - Query vector (1536-dim for text-embedding-3-small)
/// * `category` - Optional category filter
/// * `min_similarity` - Minimum similarity threshold (0.0-1.0)
///
/// # Returns
///
/// Vector of memories sorted by similarity score
pub fn semantic_search(
    db: &Connection,
    query_embedding: &[f32],
    category: Option<&MemoryCategory>,
    min_similarity: f32,
) -> Result<Vec<UnifiedMemory>, Box<dyn std::error::Error + Send + Sync>> {
    tracing::debug!(
        "[Semantic Search] Query dim: {}, min_sim: {}",
        query_embedding.len(),
        min_similarity
    );

    // Query all memories with embeddings
    let sql = "SELECT
            id, session_id, memory_type, category, title, content, summary, tags,
            confidence, importance, access_count, last_accessed_at, source, embedding,
            created_at, updated_at, archived
        FROM unified_memory
        WHERE embedding IS NOT NULL
        AND archived = 0";

    let sql = if let Some(_cat) = category {
        format!("{sql} AND category = ?")
    } else {
        sql.to_string()
    };

    let mut stmt = db.prepare(&sql)?;

    // Execute query and collect rows
    let mut memories = Vec::new();
    let mut rows = if let Some(cat) = category {
        let cat_str = serde_json::to_string(cat).unwrap_or_default();
        stmt.query(params![cat_str])?
    } else {
        stmt.query([])?
    };

    while let Ok(Some(row)) = rows.next() {
        let memory = parse_memory_from_row(row)?;
        memories.push(memory);
    }

    // Calculate cosine similarity and filter by threshold
    let scored: Vec<UnifiedMemory> = memories
        .into_iter()
        .filter_map(|memory| {
            // Check if embedding exists
            if let Some(ref embedding) = &memory.metadata.embedding {
                let similarity = cosine_similarity(query_embedding, embedding);
                if similarity >= min_similarity {
                    tracing::debug!("[Semantic Search] Similarity: {}", similarity);
                    Some(memory)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    tracing::info!("[Semantic Search] Returning {} results", scored.len());

    Ok(scored)
}

/// Parse memory from database row (simplified version)
fn parse_memory_from_row(
    row: &rusqlite::Row,
) -> Result<UnifiedMemory, Box<dyn std::error::Error + Send + Sync>> {
    let id: String = row.get(0)?;
    let session_id: String = row.get(1)?;
    let memory_type_json: String = row.get(2)?;
    let category_json: String = row.get(3)?;
    let title: String = row.get(4)?;
    let content: String = row.get(5)?;
    let summary: String = row.get(6)?;
    let tags_json: String = row.get(7)?;

    let confidence: f32 = row.get(8)?;
    let importance: i64 = row.get(9)?;
    let access_count: i64 = row.get(10)?;
    let last_accessed_at: Option<i64> = row.get(11)?;
    let source_json: String = row.get(12)?;
    let embedding_blob: Option<Vec<u8>> = row.get(13)?;
    let created_at: i64 = row.get(14)?;
    let updated_at: i64 = row.get(15)?;
    let archived: i64 = row.get(16)?;

    // Parse JSON fields
    let memory_type: crate::models::MemoryType =
        serde_json::from_str(&memory_type_json).map_err(|e| format!("Invalid memory type: {e}"))?;
    let category: crate::models::MemoryCategory =
        serde_json::from_str(&category_json).map_err(|e| format!("Invalid category: {e}"))?;
    let tags: Vec<String> =
        serde_json::from_str(&tags_json).map_err(|e| format!("Invalid tags: {e}"))?;
    let source: crate::models::MemorySource =
        serde_json::from_str(&source_json).map_err(|e| format!("Invalid source: {e}"))?;

    // Parse embedding from BLOB (f32 array)
    let embedding = if let Some(blob) = embedding_blob {
        if blob.len() % 4 == 0 {
            let vec_len = blob.len() / 4;
            let mut vec = Vec::with_capacity(vec_len);
            for chunk in blob.chunks_exact(4) {
                let bytes: [u8; 4] = match chunk.try_into() {
                    Ok(arr) => arr,
                    Err(_) => [0; 4],
                };
                let val = f32::from_le_bytes(bytes);
                vec.push(val);
            }
            Some(vec)
        } else {
            None
        }
    } else {
        None
    };

    let metadata = crate::models::MemoryMetadata {
        confidence,
        importance: importance as u8,
        access_count: access_count as u32,
        last_accessed_at,
        source,
        embedding,
    };

    Ok(UnifiedMemory {
        id,
        session_id,
        memory_type,
        category,
        title,
        content,
        summary,
        tags,
        metadata,
        created_at,
        updated_at,
        archived: archived != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let vec1 = vec![1.0, 2.0, 3.0];
        let vec2 = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&vec1, &vec2);
        assert!((sim - 1.0).abs() < 0.001); // Should be approximately 1.0

        let vec3 = vec![1.0, 0.0, 0.0];
        let vec4 = vec![0.0, 1.0, 0.0];
        let sim2 = cosine_similarity(&vec3, &vec4);
        assert_eq!(sim2, 0.0); // Should be 0 (orthogonal)
    }
}
