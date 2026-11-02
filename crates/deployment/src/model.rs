use types::ReleaseId;
use uuid::Uuid;
use time::OffsetDateTime;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeploymentState {
    Queued,
    Initializing,
    Executing,
    Waiting,
    Retrying,
    RollingBack,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RollbackStatus {
    None,
    InProgress,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    #[serde(with = "time::serde::iso8601")]
    pub timestamp: OffsetDateTime,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub backoff_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelecommandPacket {
    pub id: Uuid,
    pub session_id: Uuid,
    pub sequence_number: u32,
    pub packet_type: String, // e.g. "FIRMWARE_CHUNK", "VERIFY_INTEGRITY", "BOOT_SELECT_BANK_A", "BOOT_SELECT_BANK_B"
    pub target_partition: String,
    pub frame_hex: String,
    pub crc32: u32,
    pub status: String, // "Queued", "Transmitted", "Acknowledged", "Failed"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentSession {
    pub session_id: Uuid,
    pub release_id: ReleaseId,
    pub correlation_id: Uuid,
    #[serde(with = "time::serde::iso8601")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601::option")]
    pub completed_at: Option<OffsetDateTime>,
    pub status: DeploymentState,
    pub execution_log: Vec<LogEntry>,
    pub failure_reason: Option<String>,
    pub rollback_status: RollbackStatus,
    #[serde(default)]
    pub target_satellite_id: Option<Uuid>,
    #[serde(default)]
    pub target_partition: Option<String>,
    #[serde(default)]
    pub packets_total: u32,
    #[serde(default)]
    pub packets_transmitted: u32,
    #[serde(default)]
    pub packets_acknowledged: u32,
}

impl DeploymentSession {
    pub fn new(release_id: ReleaseId, correlation_id: Uuid) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            release_id,
            correlation_id,
            started_at: OffsetDateTime::now_utc(),
            completed_at: None,
            status: DeploymentState::Queued,
            execution_log: Vec::new(),
            failure_reason: None,
            rollback_status: RollbackStatus::None,
            target_satellite_id: None,
            target_partition: Some("bank_a".to_string()),
            packets_total: 0,
            packets_transmitted: 0,
            packets_acknowledged: 0,
        }
    }

    pub fn log(&mut self, level: &str, message: &str) {
        self.execution_log.push(LogEntry {
            timestamp: OffsetDateTime::now_utc(),
            level: level.into(),
            message: message.into(),
        });
    }
}

/// Computes standard IEEE 802.3 CRC-32 checksum
pub fn compute_crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB88320 & (!((crc & 1).wrapping_sub(1))));
        }
    }
    !crc
}

/// Constructs a structured CCSDS-style telecommand frame with packet ID, sequence number, payload, and trailing CRC-32
pub fn frame_telecommand_packet(
    packet_id: u16,
    sequence_number: u32,
    target_satellite_id: Uuid,
    target_partition: &str,
    payload_chunk: &[u8],
) -> (String, u32) {
    let mut frame = Vec::with_capacity(16 + payload_chunk.len() + 4);
    // Header (10 bytes): packet_id (2 bytes) + sequence_number (4 bytes) + target partition code (4 bytes)
    frame.extend_from_slice(&packet_id.to_be_bytes());
    frame.extend_from_slice(&sequence_number.to_be_bytes());
    let mut part_bytes = [0u8; 4];
    let src_bytes = target_partition.as_bytes();
    let copy_len = src_bytes.len().min(4);
    part_bytes[..copy_len].copy_from_slice(&src_bytes[..copy_len]);
    frame.extend_from_slice(&part_bytes);
    // First 4 bytes of satellite ID
    frame.extend_from_slice(&target_satellite_id.as_bytes()[0..4]);
    // Payload chunk
    frame.extend_from_slice(payload_chunk);
    // Trailing CRC-32 (4 bytes)
    let crc = compute_crc32(&frame);
    frame.extend_from_slice(&crc.to_be_bytes());

    (hex::encode(frame), crc)
}

/// Segments a binary payload into standardized telecommand packets
pub fn segment_binary_into_packets(
    session_id: Uuid,
    target_satellite_id: Uuid,
    target_partition: &str,
    binary_payload: &[u8],
    chunk_size: usize,
) -> Vec<TelecommandPacket> {
    let mut packets = Vec::new();
    let mut seq = 0;
    for chunk in binary_payload.chunks(chunk_size) {
        let (frame_hex, crc32) = frame_telecommand_packet(
            0x1F00, // Telecommand application process ID
            seq,
            target_satellite_id,
            target_partition,
            chunk,
        );
        packets.push(TelecommandPacket {
            id: Uuid::new_v4(),
            session_id,
            sequence_number: seq,
            packet_type: "FIRMWARE_CHUNK".to_string(),
            target_partition: target_partition.to_string(),
            frame_hex,
            crc32,
            status: "Queued".to_string(),
        });
        seq += 1;
    }
    packets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_crc32() {
        let data = b"123456789";
        let crc = compute_crc32(data);
        assert_eq!(crc, 0xCBF43926); // Standard CRC-32 test vector
    }

    #[test]
    fn test_frame_telecommand_packet() {
        let sat_id = Uuid::new_v4();
        let payload = b"FIRMWARE_CHUNK_DATA_TEST";
        let (hex_frame, crc) = frame_telecommand_packet(0x1F00, 1, sat_id, "bank_a", payload);

        let decoded = hex::decode(&hex_frame).unwrap();
        // Check frame length: 2 + 4 + 4 + 4 + payload.len() + 4 = 18 + payload.len()
        assert_eq!(decoded.len(), 18 + payload.len());
        // Verify CRC at the end
        let end_crc = u32::from_be_bytes(decoded[decoded.len() - 4..].try_into().unwrap());
        assert_eq!(end_crc, crc);
    }

    #[test]
    fn test_segment_binary_into_packets() {
        let session_id = Uuid::new_v4();
        let sat_id = Uuid::new_v4();
        let binary = vec![0xAB; 1000];
        let packets = segment_binary_into_packets(session_id, sat_id, "bank_a", &binary, 256);

        // 1000 / 256 = 4 chunks (256, 256, 256, 232)
        assert_eq!(packets.len(), 4);
        assert_eq!(packets[0].sequence_number, 0);
        assert_eq!(packets[3].sequence_number, 3);
        assert_eq!(packets[0].target_partition, "bank_a");
    }
}
