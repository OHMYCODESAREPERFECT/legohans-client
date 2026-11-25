//! Client-side world state: chunks, palettes, lighting and entities.

/// Width/depth of a chunk column in blocks.
pub const CHUNK_SIZE: usize = 16;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_size_is_sixteen() {
        assert_eq!(CHUNK_SIZE, 16);
    }
}
