use crate::networking::model_server::ModelMsgServer;
use crate::networking::{IgnoreModelAdd, IgnoreModelChanged, ModelData, ModelData2};
use crate::{ModelBundle, ModelInfo};
use bevy_app::App;
use bevy_ecs::prelude::{
    Added, Changed, Commands, Entity, NonSend, Or, Query, Res, ResMut, With, World,
};
use bevy_ecs::query::Without;
use bevy_ecs::system::SystemState;
use bevy_quinnet::client::Client;
use bevy_quinnet::shared::channel::ChannelType;
use bevy_quinnet::shared::ClientId;
use bevy_transform::prelude::Transform;
use leknet::{
    ClientEntity, ClientMessage, EntityMap, LekClient, Networked, ServerEntity, TypeName,
};
use serde::{Deserialize, Serialize};
use stereokit::{Color128, Material, Model, RenderLayer, SkDraw, StereoKitMultiThread};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ModelMsgClient {
    ModelAdded(ServerEntity, ModelData),
    ModelChanged(ServerEntity, ModelData2),
    EntityMap(ServerEntity, ClientEntity),
    GetAllModelData(ClientId),
}

impl TypeName for ModelMsgClient {
    fn get_type_name() -> String {
        "stereokit_bevy::networking::ModelMsgClient".to_string()
    }
}

impl ClientMessage for ModelMsgClient {
    fn client(self, world: &mut World) {
        match self {
            ModelMsgClient::ModelAdded(server_entity, model_data) => {
                model_added_msg(world, server_entity, model_data)
            }
            ModelMsgClient::ModelChanged(server_entity, model_data) => {
                model_changed_msg(world, server_entity, model_data)
            }
            ModelMsgClient::EntityMap(server_entity, client_entity) => {
                let mut system_state: SystemState<ResMut<EntityMap>> = SystemState::new(world);
                let mut entity_map: ResMut<EntityMap> = system_state.get_mut(world);
                entity_map.0.insert(client_entity, server_entity);
            }
            ModelMsgClient::GetAllModelData(client_id) => get_all_model_data_msg(world, client_id),
        }
    }

    fn _client(world: &mut World, msg_bytes: &[u8]) {
        bincode::deserialize::<Self>(msg_bytes)
            .unwrap()
            .client(world)
    }

    fn channel_type(&self) -> ChannelType {
        match self {
            ModelMsgClient::ModelAdded(_, _) => ChannelType::OrderedReliable,
            ModelMsgClient::ModelChanged(_, _) => ChannelType::Unreliable,
            ModelMsgClient::EntityMap(_, _) => ChannelType::OrderedReliable,
            ModelMsgClient::GetAllModelData(_) => ChannelType::OrderedReliable,
        }
    }

    fn plugin(app: &mut App) {
        app.add_system(model_added);
        app.add_system(model_changed);
    }
}

fn get_all_model_data_msg(world: &mut World, client_id: ClientId) {
    let mut system_state: SystemState<(
        Query<
            (Entity, &ModelInfo, &Transform, &Color128, &RenderLayer),
            (With<Networked>, Without<IgnoreModelChanged>),
        >,
        ResMut<Client>,
        Res<EntityMap>,
    )> = SystemState::new(world);
    let (query, mut client, entity_map) = system_state.get_mut(world);
    let mut client: ResMut<Client> = client;
    let entity_map: Res<EntityMap> = entity_map;
    let mut models = vec![];
    for (entity, model_info, transform, color128, render_layer) in query.iter() {
        let server_entity = match entity_map.get_by_left(&ClientEntity(entity)) {
            None => continue,
            Some(server_entity) => server_entity.clone(),
        };
        models.push((
            server_entity,
            ModelData {
                model_info: model_info.clone(),
                transform: *transform,
                color128: *color128,
                render_layer: *render_layer,
            },
        ))
    }
    client
        .connection_mut()
        .send_lek_msg(ModelMsgServer::AllModelData(client_id, models))
        .unwrap();
}

fn model_changed_msg(world: &mut World, server_entity: ServerEntity, model_data: ModelData2) {
    let mut client_entity = None;
    {
        let mut system_state: SystemState<ResMut<EntityMap>> = SystemState::new(world);
        let mut entity_map = system_state.get_mut(world);
        client_entity = entity_map.get_by_right(&server_entity).map(|a| a.clone());
    }
    if let Some(client_entity) = client_entity {
        let mut world_entity = world.entity_mut(client_entity.0);
        match model_data {
            ModelData2 {
                transform,
                color128,
                render_layer,
            } => {
                *world_entity.get_mut().unwrap() = transform;
                *world_entity.get_mut().unwrap() = color128;
                *world_entity.get_mut().unwrap() = render_layer;
            }
        }
    }
}

fn model_added_msg(world: &mut World, server_entity: ServerEntity, model_data: ModelData) {
    let mut system_state: SystemState<(ResMut<EntityMap>, Commands, NonSend<SkDraw>)> =
        SystemState::new(world);
    let (entity_map, commands, sk) = system_state.get_mut(world);
    let mut entity_map: ResMut<EntityMap> = entity_map;
    let mut commands: Commands = commands;
    let sk: NonSend<SkDraw> = sk;
    let model = match model_data.model_info.clone() {
        ModelInfo::Mem { .. } => {
            todo!()
        }
        ModelInfo::Cube(size) => sk.model_create_mesh(sk.mesh_gen_cube(size, 1), Material::DEFAULT),
    };
    let client_entity = ClientEntity(
        commands
            .spawn(ModelBundle::new(
                model,
                model_data.model_info,
                model_data.transform,
                model_data.color128,
                model_data.render_layer,
            ))
            .insert(IgnoreModelAdd)
            .id(),
    );
    entity_map.insert(client_entity, server_entity);
    system_state.apply(world);
}

fn model_added(
    query: Query<
        (Entity, &ModelInfo, &Transform, &Color128, &RenderLayer),
        (Added<Networked>, Without<IgnoreModelAdd>),
    >,
    mut client: ResMut<Client>,
) {
    if let Some(connection) = client.get_connection_mut() {
        for (entity, model_info, transform, color128, render_layer) in query.iter() {
            connection
                .send_lek_msg(ModelMsgServer::ModelAdded(
                    ClientEntity(entity),
                    ModelData {
                        model_info: model_info.clone(),
                        transform: *transform,
                        color128: *color128,
                        render_layer: *render_layer,
                    },
                ))
                .unwrap()
        }
    }
}

fn model_changed(
    query: Query<
        (Entity, &ModelInfo, &Transform, &Color128, &RenderLayer),
        (
            Or<(Changed<Transform>, Changed<Color128>, Changed<RenderLayer>)>,
            Without<IgnoreModelAdd>,
            With<Networked>,
        ),
    >,
    mut client: ResMut<Client>,
    entity_map: Res<EntityMap>,
) {
    if let Some(connection) = client.get_connection_mut() {
        for (entity, _, transform, color128, render_layer) in query.iter() {
            if let Some(server_entity) = entity_map.get_by_left(&ClientEntity(entity)) {
                connection
                    .send_lek_msg(ModelMsgServer::ModelChanged(
                        *server_entity,
                        ModelData2 {
                            transform: *transform,
                            color128: *color128,
                            render_layer: *render_layer,
                        },
                    ))
                    .unwrap()
            }
        }
    }
}
