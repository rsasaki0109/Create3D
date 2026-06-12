/// Viewport shading modes for mesh drawables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ViewportShadingMode {
    /// Flat untextured shading using vertex colors.
    Solid,
    /// Edge-only wireframe overlay style.
    Wireframe,
    /// Full material preview with baked base color and textures.
    #[default]
    Material,
}

impl ViewportShadingMode {
    /// Human-readable label for UI controls.
    pub fn label(self) -> &'static str {
        match self {
            Self::Solid => "Solid",
            Self::Wireframe => "Wireframe",
            Self::Material => "Material",
        }
    }

    /// Iterate all supported viewport shading modes.
    pub fn all() -> [Self; 3] {
        [Self::Solid, Self::Wireframe, Self::Material]
    }
}
