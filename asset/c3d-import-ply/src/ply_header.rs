use crate::error::{ImportError, ImportResult};

/// Supported PLY on-disk formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlyFormat {
    /// ASCII 1.0 vertices.
    Ascii,
    /// Binary little-endian 1.0 vertices.
    BinaryLittleEndian,
}

/// Supported scalar property kinds for point cloud import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlyPropertyType {
    /// 32-bit float.
    Float,
    /// 8-bit unsigned integer.
    UChar,
}

/// One vertex property declared in the PLY header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlyProperty {
    /// Scalar type.
    pub property_type: PlyPropertyType,
    /// Property name such as `x` or `red`.
    pub name: String,
}

impl PlyProperty {
    /// Returns the packed byte size of this property.
    pub fn byte_size(&self) -> usize {
        match self.property_type {
            PlyPropertyType::Float => 4,
            PlyPropertyType::UChar => 1,
        }
    }
}

/// Parsed PLY header metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlyHeader {
    /// Vertex storage format.
    pub format: PlyFormat,
    /// Number of vertex records.
    pub vertex_count: usize,
    /// Ordered vertex properties.
    pub properties: Vec<PlyProperty>,
    /// Byte offset immediately after the header newline.
    pub data_offset: usize,
}

impl PlyHeader {
    /// Packed vertex record size in bytes.
    pub fn vertex_stride(&self) -> usize {
        self.properties.iter().map(PlyProperty::byte_size).sum()
    }
}

/// Parse a PLY header from file bytes.
pub fn parse_ply_header(bytes: &[u8]) -> ImportResult<PlyHeader> {
    if !bytes.starts_with(b"ply") {
        return Err(ImportError::Invalid("missing ply header".into()));
    }

    let header_end = bytes
        .windows(11)
        .position(|window| window == b"end_header\n")
        .ok_or_else(|| ImportError::Invalid("missing end_header".into()))?
        + "end_header\n".len();

    let header_text = std::str::from_utf8(&bytes[..header_end])
        .map_err(|err| ImportError::Invalid(err.to_string()))?;

    let mut lines = header_text.lines();
    let first = lines
        .next()
        .ok_or_else(|| ImportError::Invalid("empty ply".into()))?;
    if first.trim() != "ply" {
        return Err(ImportError::Invalid("missing ply magic".into()));
    }

    let mut format = PlyFormat::Ascii;
    let mut vertex_count = 0usize;
    let mut properties = Vec::new();

    for line in lines {
        let line = line.trim();
        if line.starts_with("format ascii 1.0") {
            format = PlyFormat::Ascii;
        } else if line.starts_with("format binary_little_endian 1.0") {
            format = PlyFormat::BinaryLittleEndian;
        } else if line.starts_with("element vertex ") {
            vertex_count = line
                .split_whitespace()
                .nth(2)
                .and_then(|value| value.parse().ok())
                .ok_or_else(|| ImportError::Invalid("invalid vertex count".into()))?;
        } else if line.starts_with("property ") {
            let mut parts = line.split_whitespace();
            let _ = parts.next();
            let type_name = parts
                .next()
                .ok_or_else(|| ImportError::Invalid(format!("invalid property line `{line}`")))?;
            let name = parts
                .next()
                .ok_or_else(|| ImportError::Invalid(format!("invalid property line `{line}`")))?
                .to_string();
            let property_type = parse_property_type(type_name)?;
            properties.push(PlyProperty {
                property_type,
                name,
            });
        } else if line.starts_with("element face ") || line == "end_header" {
            break;
        }
    }

    if vertex_count == 0 {
        return Err(ImportError::Invalid("ply has zero vertices".into()));
    }
    if properties.is_empty() {
        return Err(ImportError::Invalid("ply has no vertex properties".into()));
    }

    Ok(PlyHeader {
        format,
        vertex_count,
        properties,
        data_offset: header_end,
    })
}

fn parse_property_type(type_name: &str) -> ImportResult<PlyPropertyType> {
    match type_name {
        "float" | "float32" => Ok(PlyPropertyType::Float),
        "uchar" | "uint8" => Ok(PlyPropertyType::UChar),
        other => Err(ImportError::Invalid(format!(
            "unsupported ply property type `{other}`"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ascii_header() {
        let ply = b"ply\nformat ascii 1.0\nelement vertex 2\nproperty float x\nproperty float y\nproperty float z\nend_header\n";
        let header = parse_ply_header(ply).expect("parse header");
        assert_eq!(header.format, PlyFormat::Ascii);
        assert_eq!(header.vertex_count, 2);
        assert_eq!(header.vertex_stride(), 12);
    }

    #[test]
    fn parses_binary_header() {
        let ply = b"ply\nformat binary_little_endian 1.0\nelement vertex 1\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nend_header\n";
        let header = parse_ply_header(ply).expect("parse header");
        assert_eq!(header.format, PlyFormat::BinaryLittleEndian);
        assert_eq!(header.vertex_stride(), 13);
    }
}
