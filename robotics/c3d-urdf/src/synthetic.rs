/// Synthetic two-link arm URDF used by tests and desktop mock bridge demos.
pub fn preview_arm_urdf() -> &'static str {
    r#"<?xml version="1.0"?>
<robot name="preview_arm">
  <link name="base_link">
    <visual name="base_visual">
      <origin xyz="0 0 0.05" rpy="0 0 0"/>
      <geometry>
        <box size="0.2 0.2 0.1"/>
      </geometry>
      <material name="base_gray">
        <color rgba="0.6 0.6 0.65 1"/>
      </material>
    </visual>
  </link>
  <link name="upper_arm">
    <visual name="arm_visual">
      <origin xyz="0 0 0.25" rpy="0 0 0"/>
      <geometry>
        <box size="0.08 0.08 0.5"/>
      </geometry>
      <material name="arm_orange">
        <color rgba="0.85 0.45 0.2 1"/>
      </material>
    </visual>
  </link>
  <joint name="shoulder" type="revolute">
    <parent link="base_link"/>
    <child link="upper_arm"/>
    <origin xyz="0 0 0.1" rpy="0 0 0"/>
    <axis xyz="0 0 1"/>
    <limit lower="-1.57" upper="1.57" effort="10" velocity="1"/>
  </joint>
</robot>
"#
}
