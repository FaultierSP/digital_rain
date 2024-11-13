use once_cell::sync::Lazy;
use std::sync::{Arc,RwLock};
use rand::Rng;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow,WindowMode,Window,WindowResolution,WindowPlugin};
use bevy::input::ButtonInput;
use bevy::time::common_conditions::on_timer;
use bevy::utils::Duration;

const DEFAULT_SCREEN_WIDTH: f32 = 800.0;
const DEFAULT_SCREEN_HEIGHT: f32 = 600.0;
const AMOUNT_OF_COLUMNS: u16 = 60;
const FONT_FILE : &str = "NotoSansJP-SemiBold.ttf";
const MONITORING_WINDOW_SIZE_FREQUENCY_IN_HZ: f64 = 1.0;
const MAXIMUM_AMOUNT_OF_CHARACTERS: u16 = 750;
//Time in milliseconds
const DROPLETS_SPAWN_PERIOD: u64 = 3;
const DROPLETS_MOVE_PERIOD: u64 = 90; //90
const ROUGH_LENGTH_OF_A_DROPLET: i16 = 15;
const DROPLETS_LENGTH_DEVIATION: i16 = 4; //Don't declare this value to be greater or equal than ROUGH_LENGTH_OF_A_DROPLET / 2
const DROPLETS_FADE_PERIOD: f32 = 0.03; //Alpha value decrease per frame


static WINDOW_WIDTH: Lazy<Arc<RwLock<u32>>> = Lazy::new(|| Arc::new(RwLock::new(0)));
static WINDOW_HEIGHT: Lazy<Arc<RwLock<u32>>> = Lazy::new(|| Arc::new(RwLock::new(0)));
static CELL_DIMENSION: Lazy<Arc<RwLock<f32>>> = Lazy::new(|| Arc::new(RwLock::new(0.0)));
static AMOUNT_OF_ROWS: Lazy<Arc<RwLock<u16>>> = Lazy::new(|| Arc::new(RwLock::new(0)));
static DROPLETS_COUNTER: Lazy<Arc<RwLock<u8>>> = Lazy::new(|| Arc::new(RwLock::new(0)));
static CHARACTERS_COUNTER: Lazy<Arc<RwLock<u64>>> = Lazy::new(|| Arc::new(RwLock::new(0)));
static TEXT_STYLE: Lazy<Arc<RwLock<TextStyle>>> = Lazy::new(|| Arc::new(RwLock::new(TextStyle::default())));

#[derive(Component)]
struct Character {
    column: u16,
    row: u16,
    is_in_the_trail: bool,
    remaining_spawns: u8,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Digital Rain".to_string(),
                resolution: WindowResolution::new(DEFAULT_SCREEN_WIDTH, DEFAULT_SCREEN_HEIGHT),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_systems(Startup,setup)
        .add_systems(FixedUpdate, monitor_window_size)
        .add_systems(Update, toggle_fullscreen)
        .add_systems(Update, spawn_droplet.run_if(on_timer(Duration::from_millis(DROPLETS_SPAWN_PERIOD))))
        .add_systems(Update, move_droplets.run_if(on_timer(Duration::from_millis(DROPLETS_MOVE_PERIOD))))
        .insert_resource(Time::<Fixed>::from_hz(MONITORING_WINDOW_SIZE_FREQUENCY_IN_HZ))
        .add_systems(Update, fade_droplets)
        .run();
}

fn setup(
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut commands: Commands, asset_server: Res<AssetServer>,droplets_query: Query<Entity, With<Character>>
) {
    commands.spawn(Camera2dBundle::default());

    let mut text_style_guard = TEXT_STYLE.write().unwrap();
    *text_style_guard = TextStyle {
        font: asset_server.load(FONT_FILE),
        font_size: 100.0,
        color: Color::srgba(30.0, 255.0, 30.0, 1.0),
    };
    drop(text_style_guard);

    monitor_window_size(window_query, commands, droplets_query);
}

fn reset_screen(commands: Commands, droplets_query: Query<Entity, With<Character>>) {
    let cell_dimension = *WINDOW_WIDTH.read().unwrap() as f32 / AMOUNT_OF_COLUMNS as f32;

    *CELL_DIMENSION.write().unwrap() = cell_dimension;
    *AMOUNT_OF_ROWS.write().unwrap() = (*WINDOW_HEIGHT.read().unwrap() as f32 / cell_dimension) as u16;
    TEXT_STYLE.write().unwrap().font_size = cell_dimension;

    *DROPLETS_COUNTER.write().unwrap() = 0;

    despawn_droplets(commands,droplets_query);

}

fn monitor_window_size(
    window_query: Query<&Window, With<PrimaryWindow>>,
    commands: Commands,
    droplets_query: Query<Entity, With<Character>>
) {
    let window = window_query.single();
    
    let width = window.physical_width();
    let height = window.physical_height();
    let mut size_changed = false;

    if width != *WINDOW_WIDTH.read().unwrap() {
        *WINDOW_WIDTH.write().unwrap() = width;
        size_changed = true;
    }

    if height != *WINDOW_HEIGHT.read().unwrap() {
        *WINDOW_HEIGHT.write().unwrap() = height;
        size_changed = true;
    }

    if size_changed {
        reset_screen(commands,droplets_query);
    }

    //info!("Current amount of characters: {}", *CHARACTERS_COUNTER.read().unwrap());
}

fn toggle_fullscreen(
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let mut window = windows.single_mut();
    
    if keyboard_input.just_pressed(KeyCode::F11) {
        if window.mode == WindowMode::Fullscreen {
            window.mode = WindowMode::Windowed;
        } else {
            window.mode = WindowMode::Fullscreen;
        }
    }
}

fn spawn_droplet(
    mut commands: Commands,
    query_character: Query<&Character>,
) {
    //Spawn first character
    let mut thread_rng = rand::thread_rng();

    let (column_index, row_index) = get_coordinates_of_a_free_cell(query_character, &mut thread_rng);

    //Don't start a droplet if the maximum amount of characters has been reached
    if *CHARACTERS_COUNTER.read().unwrap() >= MAXIMUM_AMOUNT_OF_CHARACTERS as u64 {
        //info!("Maximum amount of characters has been reached");

        return;
    }

    let length_deviation = thread_rng.gen_range(-DROPLETS_LENGTH_DEVIATION..DROPLETS_LENGTH_DEVIATION);
    let length_of_the_droplet = ROUGH_LENGTH_OF_A_DROPLET + length_deviation;
    
    spawn_character(&mut commands, column_index, row_index,length_of_the_droplet as u8);
}

fn move_droplets(
    mut commands: Commands,
    mut droplets_query: Query<(Entity, &mut Character, &mut Text), With<Character>>,
) {
    for (_entity, mut character, mut text) in droplets_query.iter_mut() {
        if !character.is_in_the_trail {
            if character.remaining_spawns>0 {
                spawn_character(&mut commands, character.column, character.row + 1,character.remaining_spawns-1);
                character.remaining_spawns = character.remaining_spawns - 1;               
            }

            character.is_in_the_trail = true;
            text.sections[0].style.color = Color::srgba(0.0, 255.0, 0.0, 1.0);
        }
    }
}

fn fade_droplets(
    mut commands: Commands,
    mut droplets_query: Query<(Entity, &mut Text, &mut Character), With<Character>>,
) {
    for (entity, mut text, character) in droplets_query.iter_mut() {
        if character.is_in_the_trail {
            if text.sections[0].style.color.is_fully_transparent() {
                commands.entity(entity).despawn();

                decrease_characters_counter_by_one();
            } else {
                let current_opacity = text.sections[0].style.color.alpha();
                
                text.sections[0].style.color.set_alpha(current_opacity - DROPLETS_FADE_PERIOD);
            }
        }
    }
}

fn spawn_character(
    commands: &mut Commands,
    column: u16,
    row: u16,
    remaining_spawns: u8
) {    
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                get_random_char(),
                TEXT_STYLE.read().unwrap().clone(),
            ),
            transform: {
                Transform {
                    translation: Vec3::new(translate_column_to_x(column),translate_row_to_y(row), 0.0),
                    scale: Vec3::new(-1.0, 1.0, 1.0),
                    ..Default::default()
                }
            },
            ..Default::default()
        },
        Character {
            column: column,
            row: row,
            is_in_the_trail: false,
            remaining_spawns: remaining_spawns,
        },
    ));

    increase_characters_counter_by_one();
}

fn increase_characters_counter_by_one() {
    let current_amount_of_characters = *CHARACTERS_COUNTER.read().unwrap();

    if current_amount_of_characters < MAXIMUM_AMOUNT_OF_CHARACTERS as u64 {
        *CHARACTERS_COUNTER.write().unwrap() = current_amount_of_characters + 1;
    }
}

fn decrease_characters_counter_by_one() {
    let current_amount_of_characters = *CHARACTERS_COUNTER.read().unwrap();

    if current_amount_of_characters > 0 {
        *CHARACTERS_COUNTER.write().unwrap() = current_amount_of_characters - 1;
    }
}

fn despawn_droplets(mut commands: Commands, query: Query<Entity, With<Character>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }

    *CHARACTERS_COUNTER.write().unwrap() = 0;
}

fn get_coordinates_of_a_free_cell(
    query_character: Query<&Character>,
    thread_rng: &mut rand::rngs::ThreadRng,
) -> (u16, u16) {
    loop {
        let column_index = thread_rng.gen_range(0..AMOUNT_OF_COLUMNS);
        let row_index = thread_rng.gen_range(0..*AMOUNT_OF_ROWS.read().unwrap());

        if !query_character.iter().any(|cell| cell.column == column_index && cell.row == row_index) {
            return (column_index, row_index);
        }
    }
}

fn translate_column_to_x(column: u16) -> f32 {
    let cell_dimension = *CELL_DIMENSION.read().unwrap();
    column as f32 * cell_dimension - *WINDOW_WIDTH.read().unwrap() as f32 / 2.0 + cell_dimension / 2.0
}

fn translate_row_to_y(row: u16) -> f32 {
    let cell_dimension = *CELL_DIMENSION.read().unwrap();
    *WINDOW_HEIGHT.read().unwrap() as f32 / 2.0 - row as f32 * cell_dimension + cell_dimension / 2.0
}

fn get_random_char() -> char {
    let mut thread_rng = rand::thread_rng();
    let char_ranges = [
        (0x30A0..=0x30FF).filter_map(char::from_u32).collect::<Vec<_>>(),
        ('0'..='9').collect::<Vec<_>>(),
    ];
    let selected_range = &char_ranges[thread_rng.gen_range(0..char_ranges.len())];
    selected_range[thread_rng.gen_range(0..selected_range.len())]
}