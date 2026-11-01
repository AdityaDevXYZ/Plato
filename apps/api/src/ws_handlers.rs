use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use crate::state::{AppState, TelemetryStreamMessage};
use uuid::Uuid;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(tag = "action")]
pub enum ClientWsMessage {
    #[serde(rename = "subscribe")]
    Subscribe { satellite_id: Option<Uuid> },
    #[serde(rename = "ping")]
    Ping,
}

pub async fn telemetry_stream_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_telemetry_socket(socket, state))
}

async fn handle_telemetry_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.telemetry_tx.subscribe();
    let mut filter_satellite_id: Option<Uuid> = None;

    // Send initial connection confirmation frame
    let welcome = serde_json::json!({
        "type": "Connected",
        "message": "Subscribed to Plato Telemetry Real-Time Ingestion Stream",
        "timestamp": time::OffsetDateTime::now_utc().to_string()
    });
    let _ = sender.send(Message::Text(welcome.to_string())).await;

    loop {
        tokio::select! {
            // Receive messages from client (subscriptions, pings)
            client_msg = receiver.next() => {
                match client_msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(parsed) = serde_json::from_str::<ClientWsMessage>(&text) {
                            match parsed {
                                ClientWsMessage::Subscribe { satellite_id } => {
                                    filter_satellite_id = satellite_id;
                                    let ack = serde_json::json!({
                                        "type": "SubscriptionAck",
                                        "filter_satellite_id": filter_satellite_id
                                    });
                                    let _ = sender.send(Message::Text(ack.to_string())).await;
                                }
                                ClientWsMessage::Ping => {
                                    let pong = serde_json::json!({
                                        "type": "Pong",
                                        "timestamp": time::OffsetDateTime::now_utc().to_string()
                                    });
                                    let _ = sender.send(Message::Text(pong.to_string())).await;
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        let _ = sender.send(Message::Pong(payload)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    _ => {}
                }
            }
            // Stream broadcasted telemetry from the system
            telemetry_event = rx.recv() => {
                match telemetry_event {
                    Ok(msg) => {
                        let should_send = match &msg {
                            TelemetryStreamMessage::TelemetryUpdate { satellite, .. } => {
                                filter_satellite_id.map_or(true, |id| id == satellite.id)
                            }
                            TelemetryStreamMessage::AnomalyAlert { satellite_id, .. } => {
                                filter_satellite_id.map_or(true, |id| id == *satellite_id)
                            }
                            TelemetryStreamMessage::Heartbeat { .. } => true,
                        };

                        if should_send {
                            if let Ok(serialized) = serde_json::to_string(&msg) {
                                if sender.send(Message::Text(serialized)).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // Channel lagged behind, continue receiving fresh frames
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_ws_message_deserialization() {
        let json_ping = r#"{"action":"ping"}"#;
        let msg: ClientWsMessage = serde_json::from_str(json_ping).unwrap();
        matches!(msg, ClientWsMessage::Ping);

        let test_id = Uuid::new_v4();
        let json_sub = format!(r#"{{"action":"subscribe","satellite_id":"{}"}}"#, test_id);
        let msg2: ClientWsMessage = serde_json::from_str(&json_sub).unwrap();
        match msg2 {
            ClientWsMessage::Subscribe { satellite_id } => assert_eq!(satellite_id, Some(test_id)),
            _ => panic!("Expected Subscribe"),
        }
    }

    #[test]
    fn test_telemetry_stream_message_serialization() {
        let sat = twin::model::Satellite::new(Uuid::new_v4(), Uuid::new_v4().into(), "SAT-STREAM".into(), None, "v1.0.0".into());
        let stream_msg = TelemetryStreamMessage::TelemetryUpdate {
            satellite: sat,
            anomalies: vec![],
        };
        let serialized = serde_json::to_string(&stream_msg).unwrap();
        assert!(serialized.contains("TelemetryUpdate"));
        assert!(serialized.contains("SAT-STREAM"));
    }
}
