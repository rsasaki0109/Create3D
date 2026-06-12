use std::path::Path;

use roxmltree::Document;

use crate::error::{UrdfError, UrdfResult};
use crate::model::{
    build_robot_joint, parse_joint_type, UrdfGeometry, UrdfImportPlan, UrdfJointSpec, UrdfLinkSpec,
    UrdfOrigin, UrdfVisualSpec,
};
use c3d_scene_schema::RobotJointLimits;

/// Parse URDF XML from a string.
pub fn parse_urdf(xml: &str) -> UrdfResult<UrdfImportPlan> {
    let document = Document::parse(xml).map_err(|err| UrdfError::Xml(err.to_string()))?;
    let robot = document.root_element();
    if robot.tag_name().name() != "robot" {
        return Err(UrdfError::Invalid("missing <robot> root".into()));
    }

    let robot_name = robot
        .attribute("name")
        .ok_or_else(|| UrdfError::Invalid("robot missing name attribute".into()))?
        .to_string();

    let mut links = Vec::new();
    let mut joints = Vec::new();

    for node in robot.children().filter(|node| node.is_element()) {
        match node.tag_name().name() {
            "link" => links.push(parse_link(node)?),
            "joint" => joints.push(parse_joint(node)?),
            _ => {}
        }
    }

    if links.is_empty() {
        return Err(UrdfError::Invalid("robot contains no links".into()));
    }

    let child_links: std::collections::HashSet<_> = joints
        .iter()
        .map(|joint| joint.joint.child_link.clone())
        .collect();
    let root_link = links
        .iter()
        .find(|link| !child_links.contains(&link.link_name))
        .map(|link| link.link_name.clone())
        .ok_or_else(|| UrdfError::Invalid("unable to determine root link".into()))?;

    Ok(UrdfImportPlan {
        robot_name,
        root_link,
        links,
        joints,
    })
}

/// Parse URDF XML from a file path.
pub fn parse_urdf_file(path: impl AsRef<Path>) -> UrdfResult<UrdfImportPlan> {
    let xml = std::fs::read_to_string(path)?;
    parse_urdf(&xml)
}

fn parse_link(node: roxmltree::Node<'_, '_>) -> UrdfResult<UrdfLinkSpec> {
    let link_name = node
        .attribute("name")
        .ok_or_else(|| UrdfError::Invalid("link missing name".into()))?
        .to_string();

    let mut visuals = Vec::new();
    for child in node.children().filter(|node| node.is_element()) {
        if child.tag_name().name() == "visual" {
            visuals.push(parse_visual(child, &link_name)?);
        }
    }

    Ok(UrdfLinkSpec { link_name, visuals })
}

fn parse_visual(node: roxmltree::Node<'_, '_>, link_name: &str) -> UrdfResult<UrdfVisualSpec> {
    let name = node
        .attribute("name")
        .map(str::to_string)
        .unwrap_or_else(|| format!("{link_name}_visual"));

    let origin = node
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "origin")
        .map(parse_origin)
        .transpose()?
        .unwrap_or_default();

    let geometry_node = node
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "geometry")
        .ok_or_else(|| UrdfError::Invalid(format!("visual `{name}` missing geometry")))?;
    let geometry = parse_geometry(geometry_node)?;

    let color = node
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "material")
        .and_then(|material| {
            material
                .children()
                .find(|node| node.is_element() && node.tag_name().name() == "color")
        })
        .and_then(|color| color.attribute("rgba"))
        .map(parse_rgba)
        .transpose()?
        .unwrap_or([0.7, 0.7, 0.7, 1.0]);

    Ok(UrdfVisualSpec {
        name,
        origin,
        geometry,
        color,
    })
}

fn parse_joint(node: roxmltree::Node<'_, '_>) -> UrdfResult<UrdfJointSpec> {
    let name = node
        .attribute("name")
        .ok_or_else(|| UrdfError::Invalid("joint missing name".into()))?;
    let joint_type = parse_joint_type(
        node.attribute("type")
            .ok_or_else(|| UrdfError::Invalid(format!("joint `{name}` missing type")))?,
    )?;

    let parent = node
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "parent")
        .and_then(|node| node.attribute("link"))
        .ok_or_else(|| UrdfError::Invalid(format!("joint `{name}` missing parent")))?;
    let child = node
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "child")
        .and_then(|node| node.attribute("link"))
        .ok_or_else(|| UrdfError::Invalid(format!("joint `{name}` missing child")))?;

    let origin = node
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "origin")
        .map(parse_origin)
        .transpose()?
        .unwrap_or_default();

    let axis = node
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "axis")
        .and_then(|node| node.attribute("xyz"))
        .map(parse_vec3)
        .transpose()?
        .unwrap_or([0.0, 0.0, 1.0]);

    let limits = node
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "limit")
        .map(parse_limits)
        .transpose()?
        .flatten();

    Ok(UrdfJointSpec {
        joint: build_robot_joint(name, joint_type, parent, child, origin, axis, limits),
    })
}

fn parse_geometry(node: roxmltree::Node<'_, '_>) -> UrdfResult<UrdfGeometry> {
    if let Some(child) = node.children().find(|node| node.is_element()) {
        return match child.tag_name().name() {
            "box" => {
                let size = child
                    .attribute("size")
                    .ok_or_else(|| UrdfError::Invalid("box missing size".into()))?;
                Ok(UrdfGeometry::Box {
                    size: parse_vec3(size)?,
                })
            }
            "cylinder" => Ok(UrdfGeometry::Cylinder {
                radius: parse_f64(
                    child
                        .attribute("radius")
                        .ok_or_else(|| UrdfError::Invalid("cylinder missing radius".into()))?,
                )?,
                length: parse_f64(
                    child
                        .attribute("length")
                        .ok_or_else(|| UrdfError::Invalid("cylinder missing length".into()))?,
                )?,
            }),
            "sphere" => Ok(UrdfGeometry::Sphere {
                radius: parse_f64(
                    child
                        .attribute("radius")
                        .ok_or_else(|| UrdfError::Invalid("sphere missing radius".into()))?,
                )?,
            }),
            "mesh" => Ok(UrdfGeometry::Mesh {
                filename: child
                    .attribute("filename")
                    .ok_or_else(|| UrdfError::Invalid("mesh missing filename".into()))?
                    .to_string(),
                scale: child
                    .attribute("scale")
                    .map(parse_vec3)
                    .transpose()?
                    .unwrap_or([1.0, 1.0, 1.0]),
            }),
            other => Err(UrdfError::Invalid(format!(
                "unsupported geometry `{other}`"
            ))),
        };
    }
    Err(UrdfError::Invalid("geometry node is empty".into()))
}

fn parse_origin(node: roxmltree::Node<'_, '_>) -> UrdfResult<UrdfOrigin> {
    Ok(UrdfOrigin {
        xyz: node
            .attribute("xyz")
            .map(parse_vec3)
            .transpose()?
            .unwrap_or([0.0, 0.0, 0.0]),
        rpy: node
            .attribute("rpy")
            .map(parse_vec3)
            .transpose()?
            .unwrap_or([0.0, 0.0, 0.0]),
    })
}

fn parse_limits(node: roxmltree::Node<'_, '_>) -> UrdfResult<Option<RobotJointLimits>> {
    Ok(Some(RobotJointLimits {
        lower: parse_f64(
            node.attribute("lower")
                .ok_or_else(|| UrdfError::Invalid("limit missing lower".into()))?,
        )?,
        upper: parse_f64(
            node.attribute("upper")
                .ok_or_else(|| UrdfError::Invalid("limit missing upper".into()))?,
        )?,
        effort: parse_f64(node.attribute("effort").unwrap_or("0"))?,
        velocity: parse_f64(node.attribute("velocity").unwrap_or("0"))?,
    }))
}

fn parse_vec3(value: &str) -> UrdfResult<[f64; 3]> {
    let parts: Vec<_> = value.split_whitespace().collect();
    if parts.len() != 3 {
        return Err(UrdfError::Invalid(format!("expected vec3, got `{value}`")));
    }
    Ok([
        parse_f64(parts[0])?,
        parse_f64(parts[1])?,
        parse_f64(parts[2])?,
    ])
}

fn parse_rgba(value: &str) -> UrdfResult<[f32; 4]> {
    let parts: Vec<_> = value.split_whitespace().collect();
    if parts.len() != 4 {
        return Err(UrdfError::Invalid(format!("expected rgba, got `{value}`")));
    }
    Ok([
        parse_f64(parts[0])? as f32,
        parse_f64(parts[1])? as f32,
        parse_f64(parts[2])? as f32,
        parse_f64(parts[3])? as f32,
    ])
}

fn parse_f64(value: &str) -> UrdfResult<f64> {
    value
        .parse()
        .map_err(|err| UrdfError::Invalid(format!("invalid float `{value}`: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::preview_arm_urdf;

    #[test]
    fn preview_arm_parses() {
        let plan = parse_urdf(preview_arm_urdf()).expect("parse preview arm");
        assert_eq!(plan.robot_name, "preview_arm");
        assert_eq!(plan.root_link, "base_link");
        assert_eq!(plan.links.len(), 2);
        assert_eq!(plan.joints.len(), 1);
        assert_eq!(plan.joints[0].joint.joint_name, "shoulder");
    }
}
