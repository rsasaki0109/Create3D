use std::collections::HashMap;
use std::path::Path;

use c3d_asset_mesh::MeshAssetData;
use roxmltree::{Document, Node};

use crate::{ImportError, ImportResult};

/// Import a Collada mesh file from disk.
pub fn import_collada_path(path: impl AsRef<Path>) -> ImportResult<MeshAssetData> {
    let bytes = std::fs::read(path)?;
    import_collada_bytes(&bytes)
}

/// Import a Collada mesh from memory.
pub fn import_collada_bytes(bytes: &[u8]) -> ImportResult<MeshAssetData> {
    let text = std::str::from_utf8(bytes)
        .map_err(|err| ImportError::Invalid(format!("collada is not valid UTF-8: {err}")))?;
    let document = Document::parse(text)
        .map_err(|err| ImportError::Invalid(format!("collada xml parse error: {err}")))?;

    let collada = find_first_by_local_name(document.root_element(), "COLLADA")
        .ok_or_else(|| ImportError::Invalid("missing COLLADA root element".into()))?;

    let mut mesh = MeshAssetData {
        version: 1,
        ..MeshAssetData::default()
    };

    for library in matching_children(collada, "library_geometries") {
        for geometry in matching_children(library, "geometry") {
            let Some(geometry_mesh) = matching_child(geometry, "mesh") else {
                continue;
            };
            append_mesh(geometry_mesh, &mut mesh)?;
        }
    }

    if mesh.positions.is_empty() {
        return Err(ImportError::Invalid(
            "collada contains no triangle mesh geometry".into(),
        ));
    }

    Ok(mesh)
}

fn append_mesh(mesh_node: Node<'_, '_>, output: &mut MeshAssetData) -> ImportResult<()> {
    let sources = parse_sources(mesh_node)?;
    let vertices = parse_vertices(mesh_node, &sources)?;

    for primitive in matching_children(mesh_node, "triangles")
        .into_iter()
        .chain(matching_children(mesh_node, "polylist"))
    {
        append_primitive(primitive, &sources, &vertices, output)?;
    }

    Ok(())
}

struct SourceData {
    floats: Vec<f32>,
    stride: usize,
    offset: usize,
}

struct VertexInputs {
    position: String,
    normal: Option<String>,
    texcoord: Option<String>,
}

struct PrimitiveInput {
    semantic: String,
    source: String,
    offset: usize,
}

fn append_primitive(
    primitive: Node<'_, '_>,
    sources: &HashMap<String, SourceData>,
    vertices: &VertexInputs,
    output: &mut MeshAssetData,
) -> ImportResult<()> {
    let inputs = parse_primitive_inputs(primitive)?;
    if inputs.is_empty() {
        return Err(ImportError::Invalid(
            "collada primitive has no inputs".into(),
        ));
    }

    let stride = inputs.iter().map(|input| input.offset).max().unwrap_or(0) + 1;
    let indices = parse_index_data(primitive, stride)?;

    let triangle_counts = if matching_child(primitive, "vcount").is_some() {
        parse_vcount(primitive)?
    } else {
        let count = attribute(primitive, "count")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        vec![3; count]
    };

    let mut cursor = 0usize;
    for vertex_count in triangle_counts {
        if vertex_count != 3 {
            return Err(ImportError::Invalid(format!(
                "collada polylist face with {vertex_count} vertices is not supported in v0"
            )));
        }
        if cursor + stride * 3 > indices.len() {
            return Err(ImportError::Invalid(
                "collada primitive index data truncated".into(),
            ));
        }

        let mut corners = Vec::with_capacity(3);
        for corner in 0..3 {
            let tuple = &indices[cursor + corner * stride..cursor + corner * stride + stride];
            corners.push(resolve_corner(tuple, &inputs, sources, vertices)?);
        }
        cursor += stride * 3;

        for corner in corners {
            output.positions.push(corner.position);
            if let Some(normal) = corner.normal {
                output.normals.push(normal);
            }
            if let Some(uv) = corner.uv {
                output.uvs.push(uv);
            }
            output.indices.push(output.positions.len() as u32 - 1);
        }
    }

    Ok(())
}

struct ResolvedCorner {
    position: [f32; 3],
    normal: Option<[f32; 3]>,
    uv: Option<[f32; 2]>,
}

fn resolve_corner(
    tuple: &[usize],
    inputs: &[PrimitiveInput],
    sources: &HashMap<String, SourceData>,
    vertices: &VertexInputs,
) -> ImportResult<ResolvedCorner> {
    let mut position = None;
    let mut normal = None;
    let mut uv = None;

    for input in inputs {
        let index = *tuple
            .get(input.offset)
            .ok_or_else(|| ImportError::Invalid("collada index tuple too short".into()))?;
        match input.semantic.as_str() {
            "VERTEX" => {
                position = Some(read_vec3(index, Some(vertices.position.as_str()), sources)?);
            }
            "NORMAL" => {
                let source = vertices
                    .normal
                    .as_deref()
                    .or_else(|| Some(input.source.trim_start_matches('#')));
                normal = Some(read_vec3(index, source, sources)?);
            }
            "TEXCOORD" => {
                let source = vertices
                    .texcoord
                    .as_deref()
                    .or_else(|| Some(input.source.trim_start_matches('#')));
                uv = Some(read_vec2_from_source(index, source, sources)?);
            }
            _ => {}
        }
    }

    let position = position
        .ok_or_else(|| ImportError::Invalid("collada primitive missing POSITION indices".into()))?;
    Ok(ResolvedCorner {
        position,
        normal,
        uv,
    })
}

fn read_vec2_from_source(
    index: usize,
    source_id: Option<&str>,
    sources: &HashMap<String, SourceData>,
) -> ImportResult<[f32; 2]> {
    let source_id = source_id.ok_or_else(|| ImportError::Invalid("missing vec2 source".into()))?;
    let source = sources
        .get(source_id)
        .ok_or_else(|| ImportError::Invalid(format!("collada source `{source_id}` not found")))?;
    let start = source.offset + index * source.stride;
    if start + 2 > source.floats.len() {
        return Err(ImportError::Invalid(format!(
            "collada source `{source_id}` index {index} out of range"
        )));
    }
    Ok([source.floats[start], source.floats[start + 1]])
}

fn read_vec3(
    index: usize,
    source_id: Option<&str>,
    sources: &HashMap<String, SourceData>,
) -> ImportResult<[f32; 3]> {
    let source_id = source_id.ok_or_else(|| ImportError::Invalid("missing vec3 source".into()))?;
    let source = sources
        .get(source_id)
        .ok_or_else(|| ImportError::Invalid(format!("collada source `{source_id}` not found")))?;
    let start = source.offset + index * source.stride;
    if start + 3 > source.floats.len() {
        return Err(ImportError::Invalid(format!(
            "collada source `{source_id}` index {index} out of range"
        )));
    }
    Ok([
        source.floats[start],
        source.floats[start + 1],
        source.floats[start + 2],
    ])
}

fn parse_sources(mesh_node: Node<'_, '_>) -> ImportResult<HashMap<String, SourceData>> {
    let mut sources = HashMap::new();
    for source in matching_children(mesh_node, "source") {
        let Some(id) = attribute(source, "id") else {
            continue;
        };
        let Some(float_array) = find_first_by_local_name(source, "float_array") else {
            continue;
        };
        let Some(text) = float_array.text() else {
            continue;
        };
        let floats = parse_floats(text)?;
        let accessor = find_first_by_local_name(source, "accessor");
        let stride = accessor
            .and_then(|node| attribute(node, "stride"))
            .and_then(|value| value.parse().ok())
            .unwrap_or(1);
        let offset = accessor
            .and_then(|node| attribute(node, "offset"))
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        sources.insert(
            id.to_string(),
            SourceData {
                floats,
                stride,
                offset,
            },
        );
    }
    Ok(sources)
}

fn parse_vertices(
    mesh_node: Node<'_, '_>,
    sources: &HashMap<String, SourceData>,
) -> ImportResult<VertexInputs> {
    let vertices = matching_child(mesh_node, "vertices")
        .ok_or_else(|| ImportError::Invalid("collada mesh missing vertices element".into()))?;
    let mut position = None;
    let mut normal = None;
    let mut texcoord = None;
    for input in matching_children(vertices, "input") {
        let Some(semantic) = attribute(input, "semantic") else {
            continue;
        };
        let Some(source) = attribute(input, "source") else {
            continue;
        };
        let source_id = source.trim_start_matches('#').to_string();
        match semantic.as_str() {
            "POSITION" => position = Some(source_id),
            "NORMAL" => normal = Some(source_id),
            "TEXCOORD" => texcoord = Some(source_id),
            _ => {}
        }
    }
    let position = position
        .ok_or_else(|| ImportError::Invalid("collada vertices missing POSITION input".into()))?;
    if !sources.contains_key(&position) {
        return Err(ImportError::Invalid(format!(
            "collada position source `{position}` not found"
        )));
    }
    Ok(VertexInputs {
        position,
        normal,
        texcoord,
    })
}

fn parse_primitive_inputs(primitive: Node<'_, '_>) -> ImportResult<Vec<PrimitiveInput>> {
    let mut inputs = Vec::new();
    for input in matching_children(primitive, "input") {
        let Some(semantic) = attribute(input, "semantic") else {
            continue;
        };
        let Some(source) = attribute(input, "source") else {
            continue;
        };
        let offset = attribute(input, "offset")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        inputs.push(PrimitiveInput {
            semantic: semantic.to_string(),
            source: source.to_string(),
            offset,
        });
    }
    Ok(inputs)
}

fn parse_index_data(primitive: Node<'_, '_>, stride: usize) -> ImportResult<Vec<usize>> {
    let Some(index_node) = matching_child(primitive, "p") else {
        return Err(ImportError::Invalid(
            "collada primitive missing index data".into(),
        ));
    };
    let Some(text) = index_node.text() else {
        return Err(ImportError::Invalid(
            "collada primitive index data is empty".into(),
        ));
    };
    let values = parse_usizes(text)?;
    if !values.len().is_multiple_of(stride) {
        return Err(ImportError::Invalid(
            "collada primitive index count is not a multiple of input stride".into(),
        ));
    }
    Ok(values)
}

fn parse_vcount(primitive: Node<'_, '_>) -> ImportResult<Vec<usize>> {
    let Some(vcount_node) = matching_child(primitive, "vcount") else {
        return Err(ImportError::Invalid(
            "collada polylist missing vcount element".into(),
        ));
    };
    let Some(text) = vcount_node.text() else {
        return Err(ImportError::Invalid(
            "collada polylist vcount is empty".into(),
        ));
    };
    parse_usizes(text)
}

fn parse_floats(text: &str) -> ImportResult<Vec<f32>> {
    text.split_whitespace()
        .map(|value| {
            value.parse::<f32>().map_err(|err| {
                ImportError::Invalid(format!("invalid collada float `{value}`: {err}"))
            })
        })
        .collect()
}

fn parse_usizes(text: &str) -> ImportResult<Vec<usize>> {
    text.split_whitespace()
        .map(|value| {
            value.parse::<usize>().map_err(|err| {
                ImportError::Invalid(format!("invalid collada integer `{value}`: {err}"))
            })
        })
        .collect()
}

fn attribute(node: Node<'_, '_>, name: &str) -> Option<String> {
    node.attribute((NS, name))
        .or_else(|| node.attribute(name))
        .map(str::to_string)
}

fn matching_child<'a, 'input: 'a>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    matching_children(node, name).into_iter().next()
}

fn matching_children<'a, 'input: 'a>(node: Node<'a, 'input>, name: &str) -> Vec<Node<'a, 'input>> {
    node.children()
        .filter(|child| child.is_element() && child.tag_name().name() == name)
        .collect()
}

fn find_first_by_local_name<'a, 'input: 'a>(
    node: Node<'a, 'input>,
    name: &str,
) -> Option<Node<'a, 'input>> {
    if node.is_element() && node.tag_name().name() == name {
        return Some(node);
    }
    node.descendants()
        .filter(|node| node.is_element())
        .find(|child| child.tag_name().name() == name)
}

const NS: &str = "http://www.w3.org/XML/1998/namespace";

#[cfg(test)]
mod tests {
    use super::*;

    const TRIANGLE_DAE: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<COLLADA xmlns="http://www.collada.org/2005/11/COLLADASchema" version="1.4.1">
  <library_geometries>
    <geometry id="mesh" name="mesh">
      <mesh>
        <source id="mesh-positions">
          <float_array id="mesh-positions-array" count="9">0 0 0 1 0 0 0 1 0</float_array>
          <technique_common>
            <accessor source="#mesh-positions-array" count="3" stride="3">
              <param name="X" type="float"/>
              <param name="Y" type="float"/>
              <param name="Z" type="float"/>
            </accessor>
          </technique_common>
        </source>
        <vertices id="mesh-vertices">
          <input semantic="POSITION" source="#mesh-positions"/>
        </vertices>
        <triangles count="1">
          <input semantic="VERTEX" source="#mesh-vertices" offset="0"/>
          <p>0 1 2</p>
        </triangles>
      </mesh>
    </geometry>
  </library_geometries>
</COLLADA>"##;

    #[test]
    fn imports_collada_triangle_mesh() {
        let mesh = import_collada_bytes(TRIANGLE_DAE.as_bytes()).expect("import collada");
        assert_eq!(mesh.positions.len(), 3);
        assert_eq!(mesh.indices, vec![0, 1, 2]);
        assert_eq!(mesh.positions[1], [1.0, 0.0, 0.0]);
    }

    #[test]
    fn imports_collada_polylist_mesh() {
        let dae = r##"<?xml version="1.0" encoding="UTF-8"?>
<COLLADA xmlns="http://www.collada.org/2005/11/COLLADASchema" version="1.4.1">
  <library_geometries>
    <geometry id="mesh">
      <mesh>
        <source id="positions">
          <float_array id="positions-array" count="9">0 0 0 1 0 0 0 1 0</float_array>
          <technique_common>
            <accessor source="#positions-array" count="3" stride="3"/>
          </technique_common>
        </source>
        <vertices id="vertices">
          <input semantic="POSITION" source="#positions"/>
        </vertices>
        <polylist count="1">
          <input semantic="VERTEX" source="#vertices" offset="0"/>
          <vcount>3</vcount>
          <p>0 1 2</p>
        </polylist>
      </mesh>
    </geometry>
  </library_geometries>
</COLLADA>"##;
        let mesh = import_collada_bytes(dae.as_bytes()).expect("import polylist");
        assert_eq!(mesh.positions.len(), 3);
        assert_eq!(mesh.indices.len(), 3);
    }
}
