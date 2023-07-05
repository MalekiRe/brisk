use crate::networking::model_client::ModelMsgClient;
use crate::networking::{ModelData, ModelData2};
use bevy_app::App;
use bevy_ecs::prelude::{Commands, EventReader, ResMut, World};
use bevy_ecs::system::SystemState;
use bevy_quinnet::server::{ConnectionEvent, Server};
use bevy_quinnet::shared::channel::ChannelType;
use bevy_quinnet::shared::ClientId;
use leknet::{ClientEntity, LekServer, ServerEntity, ServerMessage, TypeName};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ModelMsgServer {
    ModelAdded(ClientEntity, ModelData),
    ModelChanged(ServerEntity, ModelData2),
    AllModelData(ClientId, Vec<(ServerEntity, ModelData)>),
}

impl TypeName for ModelMsgServer {
    fn get_type_name() -> String {
        "stereokit_bevy::networking::ModelMsgServer".to_string()
    }
}

impl ServerMessage for ModelMsgServer {
    fn server(self, world: &mut World, client_id: ClientId) {
        match self {
            ModelMsgServer::ModelAdded(client_entity, model_data) => {
                model_added_msg(world, client_id, client_entity, model_data)
            }
            ModelMsgServer::ModelChanged(server_entity, model_data) => {
                model_changed_msg(world, client_id, server_entity, model_data)
            }
            ModelMsgServer::AllModelData(client_id, all_model_data) => {
                let mut endpoint: SystemState<ResMut<Server>> = SystemState::new(world);
                let mut endpoint = endpoint.get_mut(world);
                let endpoint = endpoint.endpoint_mut();
                for (entity, model_data) in all_model_data {
                    endpoint
                        .send_lek_msg(
                            client_id.clone(),
                            ModelMsgClient::ModelAdded(entity, model_data),
                        )
                        .unwrap();
                }
            }
        }
    }

    fn _server(world: &mut World, msg_bytes: &[u8], client_id: ClientId) {
        bincode::deserialize::<Self>(msg_bytes)
            .unwrap()
            .server(world, client_id);
    }

    fn channel_type(&self) -> ChannelType {
        match self {
            ModelMsgServer::ModelAdded(_, _) => ChannelType::OrderedReliable,
            ModelMsgServer::ModelChanged(_, _) => ChannelType::Unreliable,
            ModelMsgServer::AllModelData(_, _) => ChannelType::OrderedReliable,
        }
    }

    fn plugin(app: &mut App) {
        app.add_system(new_client_connected);
    }
}

fn model_changed_msg(world: &mut World, client_id: ClientId, server_entity: ServerEntity, model_data: ModelData2) {
    let mut system_state: SystemState<ResMut<Server>> = SystemState::new(world);
    let mut server = system_state.get_mut(world);
    let endpoint = server.endpoint_mut();
    for client_id2 in endpoint.clients() {
        if client_id2 == client_id {
            continue;
        }
        endpoint
            .send_lek_msg(
                client_id2.clone(),
                ModelMsgClient::ModelChanged(
                    server_entity,
                    model_data.clone(),
                ),
            )
            .unwrap();
    }
}

fn model_added_msg(world: &mut World, client_id: ClientId, client_entity: ClientEntity, model_data: ModelData) {
    let mut system_state: SystemState<(ResMut<Server>, Commands)> =
        SystemState::new(world);
    let (mut server, mut commands) = system_state.get_mut(world);
    let mut server: ResMut<Server> = server;
    let mut commands: Commands = commands;
    let server_entity = ServerEntity(commands.spawn_empty().id());
    let endpoint = server.get_endpoint_mut().expect("no server endpoint");
    endpoint
        .send_lek_msg(
            client_id,
            ModelMsgClient::EntityMap(server_entity, client_entity),
        )
        .unwrap();
    for client_id2 in endpoint.clients() {
        if client_id2 == client_id {
            continue;
        }
        endpoint
            .send_lek_msg(
                client_id2.clone(),
                ModelMsgClient::ModelAdded(
                    server_entity,
                    model_data.clone(),
                ),
            )
            .unwrap()
    }
    system_state.apply(world);
}

fn new_client_connected(mut connected: EventReader<ConnectionEvent>, mut server: ResMut<Server>) {
    let endpoint = server.endpoint_mut();
    for client in connected.iter() {
        let client_id: ClientId = client.id.clone();
        for client_id2 in endpoint.clients() {
            if client_id2 == client_id {
                continue;
            }
            endpoint
                .send_lek_msg(
                    client_id2,
                    ModelMsgClient::GetAllModelData(client_id.clone()),
                )
                .unwrap();
        }
    }
}
