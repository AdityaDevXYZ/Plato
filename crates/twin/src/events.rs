use eventbus::Event;
use std::any::Any;
use uuid::Uuid;

macro_rules! define_event {
    ($name:ident { $($field:ident: $type:ty),* }) => {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct $name {
            $(pub $field: $type),*
        }
        impl Event for $name {
            fn event_type(&self) -> &'static str { stringify!($name) }
            fn as_any(&self) -> &dyn Any { self }
        }
    };
}

define_event!(TwinCreated { entity_id: Uuid, entity_type: String });
define_event!(TwinUpdated { entity_id: Uuid, version: u64 });
define_event!(SatelliteHealthChanged { entity_id: Uuid, new_score: u8 });
define_event!(SatelliteAnomalyDetected { entity_id: Uuid, anomaly_type: String, severity: String, description: String });
define_event!(GroundStationStatusChanged { entity_id: Uuid, availability: bool });
