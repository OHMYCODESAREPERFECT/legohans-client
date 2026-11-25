//! Game registries: blocks, items, biomes and tags, loaded from data generated on the user's machine.

/// Identifier namespace used by vanilla content.
pub const VANILLA_NAMESPACE: &str = "minecraft";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vanilla_namespace_is_minecraft() {
        assert_eq!(VANILLA_NAMESPACE, "minecraft");
    }
}
