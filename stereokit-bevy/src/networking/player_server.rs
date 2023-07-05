use bevy_app::App;
use bevy_ecs::prelude::{Commands, EventReader, ResMut, World};
use bevy_ecs::system::SystemState;
use bevy_quinnet::server::{ConnectionEvent, Server};
use bevy_quinnet::shared::channel::ChannelType;
use bevy_quinnet::shared::channel::ChannelType::{OrderedReliable, Unreliable};
use bevy_quinnet::shared::ClientId;
use bevy_transform::components::Transform;
use leknet::{ClientEntity, LekServer, ServerEntity, ServerMessage, TypeName};
use serde::{Serialize, Deserialize};
use crate::networking::player_client::PlayerMsgClient;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PlayerMsgServer {
    PlayerAdded(ClientEntity, Transform),
    PlayerChanged(ServerEntity, Transform),
    AllPlayerData(ClientId, Vec<(ServerEntity, Transform)>),
}
impl TypeName for PlayerMsgServer {
    fn get_type_name() -> String {
        "stereokit_bevy::networking::PlayerMsgServer".to_string()
    }
}
impl ServerMessage for PlayerMsgServer {
    fn server(self, world: &mut World, client_id: ClientId) {
        match self {
            PlayerMsgServer::PlayerAdded(client_entity, player_data) => {
                player_added_msg(world, client_id, client_entity, player_data)
            }
            PlayerMsgServer::PlayerChanged(server_entity, player_data) => {
                player_changed_msg(world, client_id, server_entity, player_data)
            }
            PlayerMsgServer::AllPlayerData(client_id, all_player_data) => {
                let mut endpoint: SystemState<ResMut<Server>> = SystemState::new(world);
                let mut endpoint = endpoint.get_mut(world);
                let endpoint = endpoint.endpoint_mut();
                for (entity, player_data) in all_player_data {
                    endpoint
                        .send_lek_msg(
                            client_id.clone(),
                            PlayerMsgClient::PlayerAdded(entity, player_data),
                        )
                        .unwrap();
                }
            }
        }
    }

    fn _server(world: &mut World, msg_bytes: &[u8], client_id: ClientId) {
        bincode::deserialize::<Self>(msg_bytes).unwrap().server(world, client_id);
    }

    fn channel_type(&self) -> ChannelType {
        match self {
            PlayerMsgServer::PlayerAdded(_, _) => OrderedReliable,
            PlayerMsgServer::PlayerChanged(_, _) => Unreliable,
            PlayerMsgServer::AllPlayerData(_, _) => OrderedReliable,
        }
    }

    fn plugin(app: &mut App) {
        app.add_system(new_client_connected);
    }
}

fn player_changed_msg(world: &mut World, client_id: ClientId, server_entity: ServerEntity, player_data: Transform) {
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
                PlayerMsgClient::PlayerChanged(
                    server_entity,
                    player_data.clone(),
                ),
            )
            .unwrap();
    }
}

fn player_added_msg(world: &mut World, client_id: ClientId, client_entity: ClientEntity, player_data: Transform) {
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
            PlayerMsgClient::EntityMap(server_entity, client_entity),
        )
        .unwrap();
    for client_id2 in endpoint.clients() {
        if client_id2 == client_id {
            continue;
        }
        endpoint
            .send_lek_msg(
                client_id2.clone(),
                PlayerMsgClient::PlayerAdded(
                    server_entity,
                    player_data.clone(),
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
                    PlayerMsgClient::GetAllPlayers(client_id.clone()),
                )
                .unwrap();
        }
    }
}