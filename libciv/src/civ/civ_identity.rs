/// Built-in civilization identifiers for gating unique components.
///
/// `None` on a `Civilization.civ_identity` means custom/modded civ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BuiltinCiv {
    Rome,
    Greece,
    Egypt,
    Babylon,
    Germany,
    Japan,
    India,
    Arabia,
}

/// Built-in leader identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BuiltinLeader {
    Trajan,
    Pericles,
    Cleopatra,
    Hammurabi,
    Barbarossa,
    Hojo,
    Gandhi,
    Saladin,
}
