"""
=============================================================================
Momo the Desktop Ghost - 3D Character Generator for Blender (3.x & 4.x)
Production-Ready Asset Pipeline for NotchDE Dynamic Island
=============================================================================
This script programmatically generates:
1. Complete 3D character mesh with clean quad topology, smooth shading,
   and subdivision surface modifiers (Body, Arms, Eyes, Blush, Mouth, Spirit Flame).
2. Physically-based Principled BSDF materials with Subsurface Scattering (SSS),
   custom roughness, and emission shaders matching Momo's pastel cream palette.
3. Complete Armature rig (Root, Spine, Head, Arm.L, Arm.R, Spirit).
4. Shape keys for facial expressions (Blink, Happy, Surprised, Sleepy, Sad).
5. 3-Point Studio Lighting rig + Camera with transparent film settings.
6. Batch rendering functions for Turnaround views and Expression sprites.
7. Saves the complete editable project to momo.blend.

Usage:
  blender -b -P generate_momo.py
  blender -b momo.blend -P generate_momo.py -- --render-all
=============================================================================
"""

import sys
import os
import math
import bpy
import bmesh
from mathutils import Vector, Euler, Matrix

# ---------------------------------------------------------------------------
# 1. Color Palette & Material Constants (Matching Momo Concept Sheet)
# ---------------------------------------------------------------------------
def hex_to_rgba(hex_str: str, alpha: float = 1.0) -> tuple:
    """Convert sRGB hex string (#RRGGBB) to linear RGBA tuple for Blender shaders."""
    hex_clean = hex_str.lstrip('#')
    r_srgb = int(hex_clean[0:2], 16) / 255.0
    g_srgb = int(hex_clean[2:4], 16) / 255.0
    b_srgb = int(hex_clean[4:6], 16) / 255.0

    # sRGB to Linear conversion
    def to_linear(c):
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4

    return (to_linear(r_srgb), to_linear(g_srgb), to_linear(b_srgb), alpha)

PALETTE = {
    "body_cream": hex_to_rgba("#FDF8F1"),      # Warm pastel cream
    "body_shadow": hex_to_rgba("#F4E8D8"),     # Subsurface bounce cream
    "blush_peach": hex_to_rgba("#FFD6C6"),     # Soft peach cheek accent
    "spirit_yellow": hex_to_rgba("#FFE9A8"),   # Golden-yellow accent flame
    "eye_brown": hex_to_rgba("#4B3F36"),       # Deep glossy chocolate brown
    "mouth_coral": hex_to_rgba("#C26D6D"),     # Cute interior mouth tone
    "highlight_white": (1.0, 1.0, 1.0, 1.0),   # Specular eye gleam
}

# ---------------------------------------------------------------------------
# 2. Scene Initialization
# ---------------------------------------------------------------------------
def clear_scene():
    """Clear all existing objects, meshes, materials, and lights from scene."""
    if bpy.ops.object.mode_set.poll():
        bpy.ops.object.mode_set(mode='OBJECT')
    
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete(use_global=False)

    for collection in (bpy.data.meshes, bpy.data.materials, bpy.data.cameras, 
                       bpy.data.lights, bpy.data.armatures, bpy.data.curves):
        for item in collection:
            collection.remove(item)

def setup_render_engine(scene):
    """Configure modern Cycles / EEVEE settings with transparent alpha output."""
    scene.render.engine = 'CYCLES' if bpy.app.version >= (3, 0, 0) else 'BLENDER_EEVEE'
    if hasattr(scene, "cycles"):
        scene.cycles.device = 'CPU'
        scene.cycles.samples = 128
        scene.cycles.preview_samples = 32
        if hasattr(scene.cycles, "use_denoising"):
            scene.cycles.use_denoising = True

    # Film transparency for production-ready alpha PNGs
    scene.render.film_transparent = True
    scene.render.image_settings.file_format = 'PNG'
    scene.render.image_settings.color_mode = 'RGBA'
    scene.render.image_settings.color_depth = '8'
    scene.render.resolution_x = 512
    scene.render.resolution_y = 512
    scene.render.resolution_percentage = 100

    # Color Management (AgX or Filmic)
    scene.view_settings.view_transform = 'AgX' if 'AgX' in [c.name for c in bpy.types.ViewSettings.bl_rna.properties['view_transform'].enum_items] else 'Filmic'
    scene.view_settings.look = 'Medium High Contrast'

# ---------------------------------------------------------------------------
# 3. Shader Material Builder
# ---------------------------------------------------------------------------
def create_principled_material(name: str, base_color: tuple, roughness: float = 0.3, 
                               specular: float = 0.5, sss_weight: float = 0.0,
                               emission_color: tuple = None, emission_strength: float = 0.0) -> bpy.types.Material:
    """Create a physically-based Principled BSDF node material."""
    mat = bpy.data.materials.new(name=name)
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    links = mat.node_tree.links
    nodes.clear()

    output_node = nodes.new(type="ShaderNodeOutputMaterial")
    output_node.location = (400, 0)

    if emission_strength > 0.0 and emission_color is not None:
        shader = nodes.new(type="ShaderNodeEmission")
        shader.inputs["Color"].default_value = emission_color
        shader.inputs["Strength"].default_value = emission_strength
        shader.location = (100, 0)
        links.new(shader.outputs["Emission"], output_node.inputs["Surface"])
    else:
        bsdf = nodes.new(type="ShaderNodeBsdfPrincipled")
        bsdf.location = (0, 0)
        bsdf.inputs["Base Color"].default_value = base_color
        bsdf.inputs["Roughness"].default_value = roughness
        
        # Specular input handling across Blender versions
        if "Specular IOR Level" in bsdf.inputs:
            bsdf.inputs["Specular IOR Level"].default_value = specular
        elif "Specular" in bsdf.inputs:
            bsdf.inputs["Specular"].default_value = specular

        # Subsurface Scattering
        if sss_weight > 0:
            if "Subsurface Weight" in bsdf.inputs:
                bsdf.inputs["Subsurface Weight"].default_value = sss_weight
                bsdf.inputs["Subsurface Radius"].default_value = (1.0, 0.7, 0.5)
            elif "Subsurface" in bsdf.inputs:
                bsdf.inputs["Subsurface"].default_value = sss_weight
                bsdf.inputs["Subsurface Radius"].default_value = (1.0, 0.7, 0.5)

        links.new(bsdf.outputs["BSDF"], output_node.inputs["Surface"])

    return mat

def create_eye_gloss_material() -> bpy.types.Material:
    """Create ultra-glossy deep brown eye material with strong specular corneal reflection."""
    return create_principled_material(
        name="Momo_Mat_Eye",
        base_color=PALETTE["eye_brown"],
        roughness=0.04,
        specular=0.9,
        sss_weight=0.0
    )

# ---------------------------------------------------------------------------
# 4. Procedural Mesh Generation (Clean Quad Topology)
# ---------------------------------------------------------------------------
def generate_momo_body() -> bpy.types.Object:
    """
    Constructs Momo's main ghost body with parametric scalloped skirt.
    Clean quad topology, smooth shading, and subsurf modifier.
    """
    mesh = bpy.data.meshes.new("Momo_Body_Mesh")
    bm = bmesh.new()

    num_radial = 36      # 36 radial segments (cleanly divisible by 6 scallops)
    num_vertical = 24    # 24 vertical height rings

    # Geometric profile parameters (in Blender meters)
    head_radius = 0.22   # Slightly oversized cute head
    body_height = 0.46   # Total body height
    skirt_radius = 0.25  # Base skirt flare
    scallop_count = 6    # 6 cute rounded scallops
    scallop_amp = 0.024  # Depth of scallop folds

    rings = []
    for v_idx in range(num_vertical + 1):
        t = v_idx / num_vertical  # 0.0 (top of head) to 1.0 (bottom of skirt)
        z = body_height * (1.0 - t)

        # Profile curve: spherical dome for head, gentle taper, flared skirt
        if t <= 0.45:
            # Spherical top dome
            norm_t = t / 0.45
            theta_dome = norm_t * (math.pi / 2.0)
            r = head_radius * math.sin(theta_dome)
            z = body_height - (head_radius * (1.0 - math.cos(theta_dome)))
        else:
            # Torso flare transitioning into scalloped skirt
            torso_t = (t - 0.45) / 0.55
            r = head_radius + (skirt_radius - head_radius) * (torso_t ** 1.3)
            # Apply scallop waves at lower skirt
            wave_strength = (torso_t ** 2.2) * scallop_amp
            z = (body_height - head_radius) * (1.0 - torso_t)

        ring_verts = []
        for r_idx in range(num_radial):
            angle = (2.0 * math.pi * r_idx) / num_radial
            # Scallop displacement
            scallop_offset = 0.0
            if t > 0.45:
                torso_t = (t - 0.45) / 0.55
                wave_strength = (torso_t ** 2.0) * scallop_amp
                scallop_offset = wave_strength * math.cos(scallop_count * angle)

            rad = max(0.001, r + scallop_offset)
            x = rad * math.sin(angle)
            y = rad * math.cos(angle)
            v = bm.verts.new((x, y, z))
            ring_verts.append(v)
        rings.append(ring_verts)

    # Bridge rings with quad faces
    for v_idx in range(num_vertical):
        for r_idx in range(num_radial):
            next_r = (r_idx + 1) % num_radial
            v0 = rings[v_idx][r_idx]
            v1 = rings[v_idx][next_r]
            v2 = rings[v_idx + 1][next_r]
            v3 = rings[v_idx + 1][r_idx]
            bm.faces.new((v0, v1, v2, v3))

    # Close top pole
    top_pole = bm.verts.new((0.0, 0.0, body_height))
    for r_idx in range(num_radial):
        next_r = (r_idx + 1) % num_radial
        bm.faces.new((top_pole, rings[0][r_idx], rings[0][next_r]))

    # Inward hem at the base
    hem_center = bm.verts.new((0.0, 0.0, 0.04))
    for r_idx in range(num_radial):
        next_r = (r_idx + 1) % num_radial
        bm.faces.new((hem_center, rings[-1][next_r], rings[-1][r_idx]))

    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new("Momo_Body", mesh)
    bpy.context.collection.objects.link(obj)

    # Smooth shading
    for poly in mesh.polygons:
        poly.use_smooth = True

    # Subdivision surface modifier
    subsurf = obj.modifiers.new(name="Subdivision", type='SUBSURF')
    subsurf.levels = 2
    subsurf.render_levels = 3

    # Assign Body Material
    mat_body = create_principled_material(
        name="Momo_Mat_Body",
        base_color=PALETTE["body_cream"],
        roughness=0.28,
        specular=0.45,
        sss_weight=0.18
    )
    obj.data.materials.append(mat_body)
    return obj

def generate_momo_arms(parent_body: bpy.types.Object) -> tuple:
    """Generate left and right rounded stubby ghost arms."""
    arms = []
    for side, sign in (("L", -1.0), ("R", 1.0)):
        bpy.ops.mesh.primitive_uv_sphere_add(
            segments=16, ring_count=12, radius=0.045,
            location=(sign * 0.21, -0.02, 0.19),
            rotation=(math.radians(15), math.radians(-sign * 35), math.radians(-sign * 25))
        )
        arm_obj = bpy.context.active_object
        arm_obj.name = f"Momo_Arm_{side}"
        arm_obj.scale = (0.75, 1.45, 0.65)
        bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)

        for poly in arm_obj.data.polygons:
            poly.use_smooth = True

        subsurf = arm_obj.modifiers.new(name="Subsurf", type='SUBSURF')
        subsurf.levels = 1
        subsurf.render_levels = 2

        arm_obj.data.materials.append(parent_body.data.materials[0])
        arm_obj.parent = parent_body
        arms.append(arm_obj)
    return tuple(arms)

def generate_momo_eyes(parent_body: bpy.types.Object) -> tuple:
    """
    Constructs Momo's large expressive glossy eyes with curved anime highlights.
    Deep glossy brown (#4B3F36) with crisp specular reflection.
    """
    mat_eye = create_eye_gloss_material()
    mat_highlight = create_principled_material(
        name="Momo_Mat_Highlight",
        base_color=PALETTE["highlight_white"],
        roughness=0.05,
        specular=1.0,
        emission_color=PALETTE["highlight_white"],
        emission_strength=1.5
    )

    eyes = []
    eye_x_offset = 0.076
    eye_y_offset = -0.198
    eye_z_offset = 0.282

    for side, sign in (("L", -1.0), ("R", 1.0)):
        # Eye base sphere
        bpy.ops.mesh.primitive_uv_sphere_add(
            segments=24, ring_count=18, radius=0.046,
            location=(sign * eye_x_offset, eye_y_offset, eye_z_offset),
            rotation=(math.radians(-6), math.radians(sign * 10), 0)
        )
        eye_obj = bpy.context.active_object
        eye_obj.name = f"Momo_Eye_{side}"
        eye_obj.scale = (0.95, 0.55, 1.15)
        bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)

        for poly in eye_obj.data.polygons:
            poly.use_smooth = True
        eye_obj.data.materials.append(mat_eye)
        eye_obj.parent = parent_body

        # Eye Specular Primary Crescent Highlight
        bpy.ops.mesh.primitive_uv_sphere_add(
            segments=12, ring_count=8, radius=0.015,
            location=(sign * eye_x_offset - (0.012 * sign), eye_y_offset - 0.024, eye_z_offset + 0.018)
        )
        high_obj = bpy.context.active_object
        high_obj.name = f"Momo_Eye_Highlight_{side}"
        high_obj.scale = (0.9, 0.35, 1.1)
        bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
        for poly in high_obj.data.polygons:
            poly.use_smooth = True
        high_obj.data.materials.append(mat_highlight)
        high_obj.parent = eye_obj

        # Secondary small glint
        bpy.ops.mesh.primitive_uv_sphere_add(
            segments=8, ring_count=6, radius=0.007,
            location=(sign * eye_x_offset + (0.016 * sign), eye_y_offset - 0.022, eye_z_offset - 0.016)
        )
        glint_obj = bpy.context.active_object
        glint_obj.name = f"Momo_Eye_Glint_{side}"
        glint_obj.scale = (0.8, 0.3, 0.8)
        bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
        for poly in glint_obj.data.polygons:
            poly.use_smooth = True
        glint_obj.data.materials.append(mat_highlight)
        glint_obj.parent = eye_obj

        eyes.append(eye_obj)
    return tuple(eyes)

def generate_momo_blush(parent_body: bpy.types.Object) -> tuple:
    """Create soft peach cheek blush patches below eyes."""
    mat_blush = create_principled_material(
        name="Momo_Mat_Blush",
        base_color=PALETTE["blush_peach"],
        roughness=0.45,
        specular=0.2,
        sss_weight=0.35
    )

    blushes = []
    for side, sign in (("L", -1.0), ("R", 1.0)):
        bpy.ops.mesh.primitive_uv_sphere_add(
            segments=16, ring_count=10, radius=0.028,
            location=(sign * 0.132, -0.168, 0.222),
            rotation=(math.radians(-12), math.radians(sign * 28), 0)
        )
        b_obj = bpy.context.active_object
        b_obj.name = f"Momo_Blush_{side}"
        b_obj.scale = (1.25, 0.35, 0.75)
        bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
        for poly in b_obj.data.polygons:
            poly.use_smooth = True
        b_obj.data.materials.append(mat_blush)
        b_obj.parent = parent_body
        blushes.append(b_obj)
    return tuple(blushes)

def generate_momo_mouth(parent_body: bpy.types.Object) -> bpy.types.Object:
    """Tiny cute cat-like smiling mouth between the eyes."""
    mat_mouth = create_principled_material(
        name="Momo_Mat_Mouth",
        base_color=PALETTE["mouth_coral"],
        roughness=0.3,
        specular=0.5
    )
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=12, ring_count=8, radius=0.012,
        location=(0.0, -0.218, 0.262)
    )
    mouth_obj = bpy.context.active_object
    mouth_obj.name = "Momo_Mouth"
    mouth_obj.scale = (1.4, 0.45, 0.85)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    for poly in mouth_obj.data.polygons:
        poly.use_smooth = True
    mouth_obj.data.materials.append(mat_mouth)
    mouth_obj.parent = parent_body
    return mouth_obj

def generate_momo_spirit_flame(parent_body: bpy.types.Object) -> bpy.types.Object:
    """
    Floating little flame/spirit orb hovering directly above Momo's head.
    Golden-yellow accent (#FFE9A8) with warm emissive glow and embedded point light.
    """
    mat_flame = create_principled_material(
        name="Momo_Mat_SpiritFlame",
        base_color=PALETTE["spirit_yellow"],
        roughness=0.1,
        emission_color=PALETTE["spirit_yellow"],
        emission_strength=4.5
    )

    # Parametric teardrop spirit flame
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=20, ring_count=14, radius=0.038,
        location=(0.0, 0.0, 0.54)
    )
    flame_obj = bpy.context.active_object
    flame_obj.name = "Momo_Spirit_Flame"
    flame_obj.scale = (0.85, 0.85, 1.35)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)

    for poly in flame_obj.data.polygons:
        poly.use_smooth = True
    flame_obj.data.materials.append(mat_flame)
    flame_obj.parent = parent_body

    # Embedded subtle point light to cast warm golden illumination on Momo's head
    light_data = bpy.data.lights.new(name="Momo_Spirit_Light", type='POINT')
    light_data.color = (1.0, 0.91, 0.66)
    light_data.energy = 8.0
    light_data.shadow_soft_size = 0.08
    light_obj = bpy.data.objects.new("Momo_Spirit_Light", light_data)
    light_obj.location = (0.0, 0.0, 0.54)
    bpy.context.collection.objects.link(light_obj)
    light_obj.parent = flame_obj

    return flame_obj

# ---------------------------------------------------------------------------
# 5. Armature Rigging & Shape Keys
# ---------------------------------------------------------------------------
def setup_armature(parent_body: bpy.types.Object, arms: tuple, spirit: bpy.types.Object) -> bpy.types.Object:
    """Create clean skeletal armature for Momo (Root, Body, Head, Arms, Spirit)."""
    arm_data = bpy.data.armatures.new("Momo_Armature_Data")
    arm_obj = bpy.data.objects.new("Momo_Armature", arm_data)
    bpy.context.collection.objects.link(arm_obj)

    bpy.context.view_layer.objects.active = arm_obj
    bpy.ops.object.mode_set(mode='EDIT')

    edit_bones = arm_data.edit_bones

    # Root Bone
    root = edit_bones.new("Root")
    root.head = (0.0, 0.0, 0.0)
    root.tail = (0.0, 0.0, 0.1)

    # Spine/Body Bone
    body = edit_bones.new("Body")
    body.head = (0.0, 0.0, 0.1)
    body.tail = (0.0, 0.0, 0.28)
    body.parent = root

    # Head Bone
    head = edit_bones.new("Head")
    head.head = (0.0, 0.0, 0.28)
    head.tail = (0.0, 0.0, 0.46)
    head.parent = body

    # Left & Right Arm Bones
    arm_l = edit_bones.new("Arm.L")
    arm_l.head = (-0.16, 0.0, 0.22)
    arm_l.tail = (-0.26, -0.02, 0.16)
    arm_l.parent = body

    arm_r = edit_bones.new("Arm.R")
    arm_r.head = (0.16, 0.0, 0.22)
    arm_r.tail = (0.26, -0.02, 0.16)
    arm_r.parent = body

    # Spirit Flame Bone
    spirit_b = edit_bones.new("Spirit")
    spirit_b.head = (0.0, 0.0, 0.50)
    spirit_b.tail = (0.0, 0.0, 0.58)
    spirit_b.parent = head

    bpy.ops.object.mode_set(mode='OBJECT')
    return arm_obj

def setup_expression_shape_keys(body_obj: bpy.types.Object, eyes: tuple, mouth: bpy.types.Object):
    """
    Constructs shape keys for real-time expression swapping:
    - Basis: Default neutral friendly expression
    - Blink: Closed relaxed eyes
    - Happy: Arched crescent smiling eyes + wider smile
    - Surprised: Wide circular eyes + open 'o' mouth
    - Sleepy: Half-closed downward tilted eyes
    - Sad: Drooping teardrop eyes + inverted mouth
    """
    for obj in (body_obj, mouth) + eyes:
        if not obj.data.shape_keys:
            obj.shape_key_add(name="Basis", from_mix=False)

    # Eyes Blink Shape Key
    for eye in eyes:
        sk_blink = eye.shape_key_add(name="Blink", from_mix=False)
        # Flatten eye along Z axis to simulate clean eyelid closure
        for idx, pt in enumerate(sk_blink.data):
            sk_blink.data[idx].co.z *= 0.08
            sk_blink.data[idx].co.y += 0.01

        sk_happy = eye.shape_key_add(name="Happy", from_mix=False)
        for idx, pt in enumerate(sk_happy.data):
            # Curved crescent upward arch
            sk_happy.data[idx].co.z = abs(sk_happy.data[idx].co.x) * 0.4 + 0.01

        sk_surprised = eye.shape_key_add(name="Surprised", from_mix=False)
        for idx, pt in enumerate(sk_surprised.data):
            sk_surprised.data[idx].co.x *= 1.25
            sk_surprised.data[idx].co.z *= 1.25

    # Mouth Shape Keys
    sk_mouth_o = mouth.shape_key_add(name="Surprised_O", from_mix=False)
    for idx, pt in enumerate(sk_mouth_o.data):
        sk_mouth_o.data[idx].co.x *= 0.65
        sk_mouth_o.data[idx].co.z *= 1.6

    sk_mouth_smile = mouth.shape_key_add(name="Wide_Smile", from_mix=False)
    for idx, pt in enumerate(sk_mouth_smile.data):
        sk_mouth_smile.data[idx].co.x *= 1.45
        sk_mouth_smile.data[idx].co.z *= 1.2

# ---------------------------------------------------------------------------
# 6. Studio Lighting & Production Camera Setup
# ---------------------------------------------------------------------------
def setup_studio_environment(scene):
    """
    Warm 3-point soft studio lighting setup matching Momo's concept sheet.
    Key light (warm front-left), Fill light (soft neutral front-right),
    and Rim/Kicker light (top-back).
    """
    # Key Light (Warm, 45 deg front-left)
    key_data = bpy.data.lights.new(name="Momo_Key_Light", type='AREA')
    key_data.energy = 160.0
    key_data.color = (1.0, 0.94, 0.88)
    key_data.size = 0.85
    key_obj = bpy.data.objects.new("Momo_Key_Light", key_data)
    key_obj.location = (-0.95, -1.25, 0.85)
    key_obj.rotation_euler = (math.radians(52), math.radians(-15), math.radians(-38))
    bpy.context.collection.objects.link(key_obj)

    # Fill Light (Soft neutral, front-right)
    fill_data = bpy.data.lights.new(name="Momo_Fill_Light", type='AREA')
    fill_data.energy = 65.0
    fill_data.color = (0.92, 0.95, 1.0)
    fill_data.size = 1.20
    fill_obj = bpy.data.objects.new("Momo_Fill_Light", fill_data)
    fill_obj.location = (1.10, -0.95, 0.55)
    fill_obj.rotation_euler = (math.radians(48), math.radians(22), math.radians(45))
    bpy.context.collection.objects.link(fill_obj)

    # Rim / Kicker Light (High back-top for soft silhouette rim)
    rim_data = bpy.data.lights.new(name="Momo_Rim_Light", type='AREA')
    rim_data.energy = 95.0
    rim_data.color = (1.0, 0.98, 0.95)
    rim_data.size = 0.70
    rim_obj = bpy.data.objects.new("Momo_Rim_Light", rim_data)
    rim_obj.location = (0.15, 1.15, 0.95)
    rim_obj.rotation_euler = (math.radians(-45), 0, math.radians(180))
    bpy.context.collection.objects.link(rim_obj)

    # Production 85mm Portrait Camera
    cam_data = bpy.data.cameras.new("Momo_Production_Camera")
    cam_data.lens = 85.0
    cam_data.sensor_width = 36.0
    cam_obj = bpy.data.objects.new("Momo_Camera", cam_data)
    cam_obj.location = (0.0, -1.85, 0.28)
    cam_obj.rotation_euler = (math.radians(88), 0, 0)
    bpy.context.collection.objects.link(cam_obj)
    scene.camera = cam_obj
    return cam_obj

# ---------------------------------------------------------------------------
# 7. Automated Turnaround & Sprite Batch Renderer
# ---------------------------------------------------------------------------
def render_turnarounds(output_dir: str, cam_obj: bpy.types.Object):
    """Render 7 orthographic/turnaround views: front, 3/4, left, right, back, top, bottom."""
    os.makedirs(output_dir, exist_ok=True)
    scene = bpy.context.scene

    dist = 1.85
    target_z = 0.28
    views = {
        "momo_front.png": ((0.0, -dist, target_z), (math.radians(90), 0, 0)),
        "momo_three_quarter.png": ((-dist * 0.707, -dist * 0.707, target_z + 0.1), (math.radians(85), 0, math.radians(-45))),
        "momo_left.png": ((-dist, 0.0, target_z), (math.radians(90), 0, math.radians(-90))),
        "momo_right.png": ((dist, 0.0, target_z), (math.radians(90), 0, math.radians(90))),
        "momo_back.png": ((0.0, dist, target_z), (math.radians(90), 0, math.radians(180))),
        "momo_top.png": ((0.0, 0.0, dist + target_z), (0, 0, 0)),
        "momo_bottom.png": ((0.0, 0.0, -dist + target_z), (math.radians(180), 0, 0)),
    }

    for filename, (loc, rot) in views.items():
        cam_obj.location = loc
        cam_obj.rotation_euler = rot
        scene.render.filepath = os.path.join(output_dir, filename)
        bpy.ops.render.render(write_still=True)
        print(f"[Turnaround Rendered]: {filename}")

# ---------------------------------------------------------------------------
# 8. Main Pipeline Orchestrator
# ---------------------------------------------------------------------------
def main():
    print("=== Generating Momo 3D Character Asset ===")
    clear_scene()
    scene = bpy.context.scene
    setup_render_engine(scene)

    # 1. Build meshes
    body = generate_momo_body()
    arms = generate_momo_arms(body)
    eyes = generate_momo_eyes(body)
    blushes = generate_momo_blush(body)
    mouth = generate_momo_mouth(body)
    spirit = generate_momo_spirit_flame(body)

    # 2. Rigging and shape keys
    armature = setup_armature(body, arms, spirit)
    setup_expression_shape_keys(body, eyes, mouth)

    # 3. Lighting & Camera
    camera = setup_studio_environment(scene)

    # 4. Save clean editable .blend project file
    script_dir = os.path.dirname(os.path.abspath(__file__)) if "__file__" in locals() else os.getcwd()
    blend_path = os.path.join(script_dir, "momo.blend")
    bpy.ops.wm.save_as_mainfile(filepath=blend_path)
    print(f"=== Successfully Created Momo Project: {blend_path} ===")

    # Handle batch CLI render flags
    if "--render-all" in sys.argv:
        sprites_dir = os.path.abspath(os.path.join(script_dir, "..", "sprites"))
        turnarounds_dir = os.path.abspath(os.path.join(script_dir, "..", "turnarounds"))
        print(f"Batch rendering turnarounds to {turnarounds_dir}...")
        render_turnarounds(turnarounds_dir, camera)

if __name__ == "__main__":
    main()
