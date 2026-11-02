//! Low-resolution view, text, and panels.

pub mod font;
pub mod palette;
pub mod typing;
pub mod wipe;

use bevy::app::MainScheduleOrder;
use bevy::camera::RenderTarget;
use bevy::camera::visibility::RenderLayers;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::window::{PrimaryWindow, WindowResized};

use font::{CELL_H, CELL_W, FontImage, build_font_image, glyph_rect};
use palette::Pal;

pub const VIEW_W: u32 = 320;
pub const VIEW_H: u32 = 200;
/// Pixels from one line of text to the next.
pub const CELL_LINE: f32 = CELL_H;
const DISPLAY_LAYER: usize = 1;

/// Z bands, so draw order is decided in one place.
pub mod z {
    pub const BACKGROUND: f32 = 0.0;
    pub const FIELD: f32 = 10.0;
    pub const SHIP: f32 = 20.0;
    pub const WEATHER: f32 = 30.0;
    pub const HUD: f32 = 40.0;
    pub const PANEL: f32 = 50.0;
    pub const TEXT: f32 = 60.0;
}

/// Transform for a point in view pixels, with (0, 0) at the top left.
pub fn at(x: f32, y: f32, z: f32) -> Transform {
    Transform::from_xyz(x - VIEW_W as f32 / 2.0, VIEW_H as f32 / 2.0 - y, z)
}

/// A solid palette rectangle whose top-left corner is at (x, y).
pub fn rect(x: f32, y: f32, w: f32, h: f32, color: Pal, z: f32) -> impl Bundle {
    (
        Sprite {
            color: color.color(),
            custom_size: Some(Vec2::new(w, h)),
            ..default()
        },
        at(x + w / 2.0, y + h / 2.0, z),
    )
}

/// Bitmap text. Change `text` through [`Label::set`] to redraw it.
#[derive(Component, Debug, Clone, PartialEq)]
#[require(Transform, Visibility)]
pub struct Label {
    pub text: String,
    pub color: Pal,
    pub scale: f32,
}

impl Label {
    pub fn new(text: impl Into<String>, color: Pal) -> Self {
        Self {
            text: text.into(),
            color,
            scale: 1.0,
        }
    }

    pub fn scaled(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Update the text, marking it changed only when it differs.
    pub fn set(&mut self, text: impl Into<String>) {
        let text = text.into();
        if self.text != text {
            self.text = text;
        }
    }
}

pub fn label(text: impl Into<String>, x: f32, y: f32, color: Pal) -> impl Bundle {
    (Label::new(text, color), at(x, y, z::TEXT))
}

/// Text horizontally centered in the whole view.
pub fn centered(text: impl Into<String>, y: f32, color: Pal, scale: f32) -> impl Bundle {
    let text = text.into();
    let width = text.len() as f32 * CELL_W * scale;
    let x = ((VIEW_W as f32 - width) / 2.0).floor();
    (Label::new(text, color).scaled(scale), at(x, y, z::TEXT))
}

/// A boxed panel, drawn as a 2px frame around a dark interior.
pub fn panel_parts(x: f32, y: f32, w: f32, h: f32) -> [impl Bundle; 2] {
    [
        rect(x, y, w, h, Pal::Foam, z::PANEL),
        rect(
            x + 2.0,
            y + 2.0,
            w - 4.0,
            h - 4.0,
            Pal::Night,
            z::PANEL + 1.0,
        ),
    ]
}

fn draw_labels(
    mut commands: Commands,
    font: Res<FontImage>,
    labels: Query<(Entity, &Label), Changed<Label>>,
) {
    for (entity, label) in &labels {
        let color = label.color.color();
        let scale = label.scale;
        let image = font.0.clone();
        let characters: Vec<char> = label.text.chars().collect();
        commands
            .entity(entity)
            .despawn_related::<Children>()
            .with_children(|parent| {
                for (index, character) in characters.into_iter().enumerate() {
                    if character == ' ' {
                        continue;
                    }
                    parent.spawn((
                        Sprite {
                            image: image.clone(),
                            rect: Some(glyph_rect(character)),
                            color,
                            custom_size: Some(Vec2::new(CELL_W * scale, CELL_H * scale)),
                            ..default()
                        },
                        Transform::from_xyz(
                            (index as f32 + 0.5) * CELL_W * scale,
                            -CELL_H * scale / 2.0,
                            0.0,
                        ),
                    ));
                }
            });
    }
}

/// A horizontal gauge. Set `value` from 0 to 1.
#[derive(Component, Debug, Clone, Copy)]
pub struct Bar {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub value: f32,
}

impl Bar {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            x,
            y,
            w,
            h,
            value: 0.0,
        }
    }

    /// Filled width in whole pixels.
    pub fn filled(&self) -> f32 {
        (self.w * self.value.clamp(0.0, 1.0)).round()
    }
}

fn draw_bars(mut bars: Query<(&Bar, &mut Sprite, &mut Transform), Changed<Bar>>) {
    for (bar, mut sprite, mut transform) in &mut bars {
        let width = bar.filled();
        sprite.custom_size = Some(Vec2::new(width, bar.h));
        let placed = at(
            bar.x + width / 2.0,
            bar.y + bar.h / 2.0,
            transform.translation.z,
        );
        transform.translation = placed.translation;
    }
}

/// A gauge's backing and its fill, in that order.
pub fn bar_parts(bar: Bar, fill: Pal, back: Pal) -> (impl Bundle, impl Bundle) {
    (
        rect(bar.x, bar.y, bar.w, bar.h, back, z::HUD),
        (bar, rect(bar.x, bar.y, 0.0, bar.h, fill, z::HUD + 1.0)),
    )
}

/// Runs after every `Update` command has been applied and before `PostUpdate`.
/// Labels despawn and respawn their glyphs. Before this point a label might be
/// removed later in the same frame, and in `PostUpdate` Bevy inserts a mesh on
/// each newly added sprite, which could target a glyph just removed.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct DrawLabels;

#[derive(Component)]
struct ViewQuad;

fn setup_view(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let target = images.add(Image::new_target_texture(
        VIEW_W,
        VIEW_H,
        TextureFormat::Bgra8UnormSrgb,
        None,
    ));
    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            clear_color: ClearColorConfig::Custom(Pal::Black.color()),
            ..default()
        },
        RenderTarget::Image(target.clone().into()),
        Msaa::Off,
    ));
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Pal::Black.color()),
            ..default()
        },
        RenderLayers::layer(DISPLAY_LAYER),
        Msaa::Off,
    ));
    commands.spawn((
        ViewQuad,
        Sprite::from_image(target),
        RenderLayers::layer(DISPLAY_LAYER),
    ));
}

/// The largest whole-number scale that fits the view in a window.
pub fn integer_scale(physical_width: u32, physical_height: u32) -> u32 {
    (physical_width / VIEW_W)
        .min(physical_height / VIEW_H)
        .max(1)
}

fn fit_view(
    mut resized: MessageReader<WindowResized>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut quad: Single<&mut Transform, With<ViewQuad>>,
    mut first_frame: Local<bool>,
) {
    if resized.read().count() == 0 && *first_frame {
        return;
    }
    *first_frame = true;
    let scale = integer_scale(window.physical_width(), window.physical_height()) as f32
        / window.scale_factor();
    quad.scale = Vec3::new(scale, scale, 1.0);
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        let font = {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();
            images.add(build_font_image())
        };
        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_after(Update, DrawLabels);
        app.add_plugins((typing::TypingPlugin, wipe::WipePlugin));
        app.insert_resource(FontImage(font))
            .add_systems(Startup, setup_view)
            .add_systems(Update, fit_view)
            .add_systems(DrawLabels, draw_labels)
            .add_systems(PostUpdate, draw_bars);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_is_whole_and_fits() {
        assert_eq!(integer_scale(960, 600), 3);
        assert_eq!(integer_scale(1000, 610), 3);
        assert_eq!(integer_scale(1920, 1080), 5);
    }

    #[test]
    fn bars_fill_in_whole_pixels_and_clamp() {
        let mut bar = Bar::new(0.0, 0.0, 40.0, 4.0);
        bar.value = 0.33;
        assert_eq!(bar.filled(), 13.0);
        bar.value = 2.0;
        assert_eq!(bar.filled(), 40.0);
    }

    #[test]
    fn scale_never_drops_below_one() {
        assert_eq!(integer_scale(100, 100), 1);
    }
}
