use bevy_app::{App, Plugin};
use bevy_ecs::prelude::{Commands, Component, Entity, Query, Res, ResMut, Without, World};
use bevy_ecs::query;
use bevy_ecs::system::{NonSend, NonSendMut, Resource, SystemState};
use bevy_hierarchy::{BuildChildren, Children};
use bevy_quinnet::client::Client;
use bevy_quinnet::server::Server;
use bevy_quinnet::shared::channel::{ChannelId, ChannelType};
use bevy_quinnet::shared::ClientId;
use bevy_transform::prelude::{GlobalTransform, Transform};
use glam::Vec3;
use leknet::{connect_to_server, start_server, LekClient, LekServer, ClientMessageMap, ClientMessage, ServerEntity, EntityMap, TypeName, ClientEntity, ServerMessage};
use opus::{Application, Channels, Decoder, Encoder};
use serde::{Deserialize, Serialize, Serializer};
use std::ops::{Deref, DerefMut};
use bevy_transform::TransformBundle;
use stereokit::{Material, Mesh, Sk, SkDraw, Sound, SoundInstance, StereoKitMultiThread};
use stereokit_bevy::networking::{Player, StereoKitBevyClientPlugins, StereoKitBevyServerPlugins};
use stereokit_bevy::{ModelBundle, ModelInfo};
use stereokit_bevy::networking::player_client::LocalPlayer;

fn stereokit_audio_send(
    mut client: ResMut<Client>,
    sk: NonSend<SkDraw>,
    mut encoder: NonSendMut<MicrophoneEncoder>,
    entity_map: Res<EntityMap>,
    mut player: Query<(Entity, &LocalPlayer)>
) {
    if !sk.mic_is_recording() {
        if sk.mic_device_count() != 0 {
            println!("{:?}", sk.mic_device_name(1));
            sk.mic_start(sk.mic_device_name(1));
        }
    }
    if !sk.mic_is_recording() {
        return;
    }
    let sound = sk.mic_get_stream();
    let mut samples = [0.0; 2880];
    let mut audio_frames = vec![];
    while sk.sound_unread_samples(&sound) >= 2880 {
        sk.sound_read_samples(&sound, &mut samples);
        audio_frames.push(
            encoder
                .encode_vec_float(samples.as_slice(), 2880)
                .expect("couldn't encode audio"),
        );
    }
    for (e, _) in player.iter() {
        let player = entity_map.get_by_left(&ClientEntity(e.clone()));
        let player = match player {
            None => return,
            Some(player) => player,
        }.clone();
        let vm = VoiceMessage {
            player,
            voice_message: audio_frames,
        };
        client.connection_mut().send_lek_msg(vm).unwrap();
        return;
    }
}

#[derive(Resource)]
pub struct MicrophoneEncoder(pub Encoder);
#[derive(Resource)]
pub struct MicrophoneDecoder(pub Decoder);

unsafe impl Sync for MicrophoneEncoder {}
unsafe impl Sync for MicrophoneDecoder {}

impl Deref for MicrophoneEncoder {
    type Target = Encoder;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Deref for MicrophoneDecoder {
    type Target = Decoder;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for MicrophoneEncoder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl DerefMut for MicrophoneDecoder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoiceMessage{
    player: ServerEntity,
    voice_message: Vec<Vec<u8>>
}

impl TypeName for VoiceMessage {
    fn get_type_name() -> String {
        "stereokit_voice_chat::VoiceMessage".to_string()
    }
}

impl ServerMessage for VoiceMessage {
    fn server(self, world: &mut World, client_id: ClientId) {
        let mut server = world.get_resource_mut::<Server>().unwrap();
        let endpoint = server.endpoint_mut();
        for client in endpoint.clients() {
            if client == client_id {
               continue;
            }
            endpoint.send_lek_msg(client, self.clone()).unwrap();
        }
    }

    fn _server(world: &mut World, msg_bytes: &[u8], client_id: ClientId) {
        bincode::deserialize::<Self>(msg_bytes).unwrap().server(world, client_id);
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Unreliable
    }

    fn plugin(app: &mut App) {

    }
}

impl ClientMessage for VoiceMessage {
    fn client(self, world: &mut World) {
        let mut samples: [f32; 2880] = [0.0; 2880];
        let mut system_state: SystemState<Res<EntityMap>> = SystemState::new(world);
        let entity_map = system_state.get_mut(world);
        let client_entity = match entity_map.get_by_right(&self.player) {
            None => return,
            Some(client_entity) => client_entity.clone()
        };
        let mut system_state: SystemState<(NonSend<SkDraw>, NonSendMut<MicrophoneDecoder>, Query<(Entity, &Children)>, Query<&Sound>)> = SystemState::new(world);
        let (sk, mut microphone_decoder, query, query2) = system_state.get_mut(world);
        let sk: NonSend<SkDraw> = sk;
        for (entity, children) in query.iter() {
            if entity == client_entity.0 {
                for child in children.iter() {
                    if let Ok(sound) = query2.get(*child) {
                        for audio in self.voice_message {
                            microphone_decoder.decode_float(&audio, &mut samples, false).unwrap();
                            sk.sound_write_samples(sound, &mut samples);
                        }
                        break;
                    }

                }
                break;
            }
        }
    }

    fn _client(world: &mut World, msg_bytes: &[u8]) {
        bincode::deserialize::<Self>(msg_bytes).unwrap().client(world);
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Unreliable
    }

    fn plugin(app: &mut App) {
        app.insert_non_send_resource(MicrophoneDecoder(Decoder::new(48000, Channels::Mono).unwrap()));
        app.insert_non_send_resource(MicrophoneEncoder(Encoder::new(48000, Channels::Mono, Application::LowDelay).unwrap()));
        app.add_system(set_sound_pos);
        app.add_system(stereokit_audio_send);
    }
}

fn set_sound_pos(query: Query<(&Sound, &SoundInstanceWrapper, &GlobalTransform)>, sk: NonSend<SkDraw>) {
    for (sound, sound_instance, transform) in query.iter() {
        sk.sound_inst_set_pos(*sound_instance.clone(), transform.translation());
        if !sk.sound_inst_is_playing(*sound_instance.clone()) {
            panic!("not playing!");
        }
    }
}

#[test]
fn client_test() {
    let mut app = App::default();
    app.add_plugins(StereoKitBevyClientPlugins);
    VoiceMessage::add_plugin_client(&mut app);
    app.add_startup_system(connect_to_server);
    app.add_system(add_sphere_to_all_players);
    app.run();
}

#[test]
fn server_test() {
    let mut app = App::default();
    app.add_plugins(StereoKitBevyServerPlugins);
    VoiceMessage::add_plugin_server(&mut app);
    app.add_startup_system(start_server);
    app.run();
}

#[derive(Component)]
pub struct PlayerHead;

#[derive(Component, Copy, Clone, Debug)]
pub struct SoundInstanceWrapper(pub SoundInstance);
impl Deref for SoundInstanceWrapper {
    type Target = SoundInstance;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn add_sphere_to_all_players(
    sk: NonSend<SkDraw>,
    query: Query<(Entity, &Player, &Transform, &GlobalTransform), Without<Children>>,
    mut commands: Commands,
) {
    for (entity, _, transform, _) in query.iter() {
        let entity: Entity = entity;
        let model = sk.model_create_mesh(sk.mesh_gen_cube(Vec3::splat(0.1), 1), Material::DEFAULT);
        let child = commands
            .spawn(ModelBundle::new(
                model,
                ModelInfo::Cube(Vec3::splat(0.1)),
                Transform::from_scale(Vec3::splat(1.0)).with_translation([0.0, 0.0, 0.0].into()),
                Default::default(),
                Default::default(),
            ))
            .insert(PlayerHead)
            .id();
        commands.entity(entity).push_children(&[child]);
        let sound = sk.sound_create_stream(2.0);
        let sound_instance = sk.sound_play(&sound, transform.translation, 1.0);
        let sound_child = commands
            .spawn(sound)
            .insert(TransformBundle::default())
            .insert(SoundInstanceWrapper(sound_instance))
            .id();
        commands.entity(entity).push_children(&[sound_child]);
    }
}

