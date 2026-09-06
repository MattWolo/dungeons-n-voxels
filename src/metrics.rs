use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::render::mesh::Indices;
use crate::generation::biome_recipes::{BiomeRecipe};
use crate::generation::voxel_type::{CHUNK_X, CHUNK_Z};
use crate::generation::WORLD_SEED;
use crate::generation::worldgen::sample_biome;
use crate::PerfUi;
use crate::player::Player;

pub struct MetricsPlugin;

#[derive(Component)]
pub struct CoordinateText;

#[derive(Resource, Debug)]
pub struct CurrentBiome(pub Option<&'static BiomeRecipe>);

impl Default for CurrentBiome {
    fn default() -> Self {
        Self(None)
    }
}

impl Plugin for MetricsPlugin {
    fn build(&self, app: &mut App){
        app
            .add_systems(Startup, setup_ui)
            .init_resource::<CurrentBiome>()
            .add_systems(Update, (update_perf_stats, update_player_info));
    }
}

fn setup_ui(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text::new("FPS: -- | Triangles: --"),
        TextFont{
            font_size: FontSize::Px(33.0),
            ..default()
        },
        PerfUi,
    ));
    commands.spawn((
        Text::new("X: -- | Y: -- \nBiome: --\nTemp: -- | Moist: --"),
        TextFont{
            font_size: FontSize::Px(22.0),
            ..default()
        },
        TextLayout::justify(Justify::Right),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            right: Val::Px(12.0),
            ..default()
        },
        CoordinateText,
    ));
}

fn update_player_info(
    player_query: Query<&Transform, With<Player>>,
    mut current_biome: ResMut<CurrentBiome>,
    mut info_query: Query<&mut Text, With<CoordinateText>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let player_chunk_x = player_transform.translation.x;
    let player_chunk_z = player_transform.translation.z;

    let (biome, temp, moist) = sample_biome(player_chunk_x as f32, player_chunk_z as f32, WORLD_SEED);

    current_biome.0 = Some(biome);

    if let Ok(mut text) = info_query.single_mut() {
        let chunk_x = (player_chunk_x / CHUNK_X as f32).floor() as i32;
        let chunk_z = (player_chunk_z / CHUNK_Z as f32).floor() as i32;

        **text = format!(
            "X: {chunk_x} | Z: {chunk_z} \nBiome: {}\nTemp: {:.2} | Moist: {:.2}",
            biome.name, temp, moist);
    }
}

fn update_perf_stats(
    diagnostics: Res<DiagnosticsStore>,
    meshes: Res<Assets<Mesh>>,
    mesh_query: Query<&Mesh3d>,
    mut text_query: Query<&mut Text, With<PerfUi>>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|diag| diag.smoothed())
        .unwrap_or(0.0);

    let mut total_triangles = 0;
    for mesh_handle in &mesh_query {
        if let Some(mesh) = meshes.get(mesh_handle) {
            if let Some(indices) = mesh.indices() {
                let index_count = match indices {
                    Indices::U16(vec) => vec.len(),
                    Indices::U32(vec) => vec.len(),
                };
                total_triangles += index_count / 3;
            }
        }
    }

    for mut text in &mut text_query {
        **text = format!("FPS: {fps:.0} | Triangles: {total_triangles}");
    }
}