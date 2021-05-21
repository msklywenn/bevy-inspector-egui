use std::marker::PhantomData;

use bevy::{
    ecs::query::{FilterFetch, WorldQuery},
    prelude::*,
};
use bevy_egui::{egui, EguiContext, EguiPlugin};

use super::{WorldInspectorParams, WorldUIContext};
use crate::InspectableRegistry;

/// Plugin for displaying an inspector window of all entites in the world and their components.
/// ```rust,no_run
/// use bevy::prelude::*;
/// use bevy_inspector_egui::WorldInspectorPlugin;
///
/// fn main() {
///     App::build()
///         .add_plugins(DefaultPlugins)
///         .add_plugin(WorldInspectorPlugin::new())
///         .add_startup_system(setup.system())
///         .run();
/// }
///
/// fn setup(mut commands: Commands) {
///   // setup your scene
///   // adding `Name` components will make the inspector more readable
/// }
/// ```
///
/// To be able to edit custom components in inspector, they need to be registered first with
/// [`crate::InspectableRegistry`], to do that they need to implement [`crate::Inspectable`].
///
/// ```rust,no_run
/// use bevy::prelude::*;
/// use bevy_inspector_egui::{Inspectable, InspectableRegistry};
///
/// #[derive(Inspectable)]
/// pub struct MyComponent {
///     foo: f32,
///     bar: usize
/// }
///
/// pub struct MyPlugin;
///
/// impl Plugin for MyPlugin {
///     fn build(&self, app: &mut AppBuilder) {
///         let mut registry = app
///             .world_mut()
///             .get_resource_or_insert_with(InspectableRegistry::default);
///
///         registry.register::<MyComponent>();
///     }
/// }
/// ```
///
/// Components can be registered in `main` function aswell, just use your [`bevy::app::AppBuilder`]
/// instance to do so.

pub struct WorldInspectorPlugin<F = ()>(PhantomData<fn() -> F>);
impl Default for WorldInspectorPlugin {
    fn default() -> Self {
        WorldInspectorPlugin::new()
    }
}

impl WorldInspectorPlugin {
    /// Create new `WorldInpsectorPlugin`
    pub fn new() -> Self {
        WorldInspectorPlugin(PhantomData)
    }

    /// Constrain the world inspector to only show entities matching the query filter `F`
    ///
    /// ```rust,no_run
    /// # use bevy::prelude::*;
    /// # use bevy_inspector_egui::WorldInspectorPlugin;
    /// struct Show;
    ///
    /// App::build()
    ///   .add_plugin(WorldInspectorPlugin::new().filter::<With<Show>>())
    ///   .run();
    /// ```
    pub fn filter<F>(self) -> WorldInspectorPlugin<F> {
        WorldInspectorPlugin(PhantomData)
    }
}

impl<F> Plugin for WorldInspectorPlugin<F>
where
    F: WorldQuery + 'static,
    F::Fetch: FilterFetch,
{
    fn build(&self, app: &mut AppBuilder) {
        if !app.world_mut().contains_resource::<EguiContext>() {
            app.add_plugin(EguiPlugin);
        }

        let world = app.world_mut();
        world.get_resource_or_insert_with(WorldInspectorParams::default);
        world.get_resource_or_insert_with(InspectableRegistry::default);

        app.add_system(world_inspector_ui::<F>.exclusive_system());
    }
}

fn world_inspector_ui<F>(world: &mut World)
where
    F: WorldQuery,
    F::Fetch: FilterFetch,
{
    let world_ptr = world as *mut _;

    let params = world.get_resource::<WorldInspectorParams>().unwrap();
    if !params.enabled {
        return;
    }

    let egui_context = world.get_resource::<EguiContext>().expect("EguiContext");
    let ctx = match egui_context.try_ctx_for_window(params.window) {
        Some(ctx) => ctx,
        None => return,
    };

    let mut entity = params.entity;
    let mut is_open = true;

    let world: &mut World = unsafe { &mut *world_ptr };
    {
        let mut ui_context = WorldUIContext::new(Some(egui_context.ctx()), world);
        ui_context.selected_entity = entity;
        if params.panel {
            egui::SidePanel::left("World", 200.0).show(ctx, |ui| {
                crate::plugin::default_settings(ui);
                ui.spacing_mut().indent *= 0.65;
                ui.heading("Hierarchy");
                ui.separator();
                ui_context.world_ui::<F>(ui, &params);
                entity = ui_context.selected_entity;
            });
        } else {
            egui::Window::new("World")
                .open(&mut is_open)
                .scroll(true)
                .show(ctx, |ui| {
                    crate::plugin::default_settings(ui);
                    ui_context.world_ui::<F>(ui, &params);
                    entity = ui_context.selected_entity;
                });
        }

        if params.panel {
            if let Some(entity) = entity {
                egui::SidePanel::left("Inspector", 200.0).show(ctx, |ui| {
                    let entity_ref = match ui_context.world.get_entity(entity) {
                        Some(entity_ref) => entity_ref,
                        None => {
                            ui.label("Entity does not exist");
                            return false;
                        }
                    };
                    let entity_location = entity_ref.location();
                    let archetype = entity_ref.archetype();

                    let id = egui::Id::new(entity);
                    let mut changed = false;

                    crate::plugin::default_settings(ui);
                    changed |= ui_context.component_kind_ui(
                        ui,
                        archetype.table_components(),
                        "Components",
                        entity,
                        entity_location,
                        params,
                        id,
                    );
                    changed |= ui_context.component_kind_ui(
                        ui,
                        archetype.sparse_set_components(),
                        "Components (Sparse)",
                        entity,
                        entity_location,
                        params,
                        id,
                    );

                    changed
                });
            }
        }
    }

    let mut params = world.get_resource_mut::<WorldInspectorParams>().unwrap();
    params.enabled = is_open;
    params.entity = entity;
}
