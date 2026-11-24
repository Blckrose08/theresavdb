import struct

PROTOCOL_MAGIC = b"THRP"
PROTOCOL_VERSION = 1
FRAME_HEADER_SIZE = 20

class MessageType:
    STARTUP = 0x0001
    AUTH = 0x0002
    AUTH_OK = 0x0004
    CLOSE = 0x000E
    QUERY = 0x0010
    ROW_DESCRIPTION = 0x0020
    DATA_ROW = 0x0021
    COMMAND_COMPLETE = 0x0022
    ERROR = 0x00FF

def encode_frame(message_type: int, request_id: int, payload: bytes) -> bytes:
    header = struct.pack(
        "<4sHHQI",
        PROTOCOL_MAGIC,
        PROTOCOL_VERSION,
        message_type,
        request_id,
        len(payload),
    )
    # Correct alignment packing: magic 4B, version 2B, msg_type 2B, payload_len 4B, request_id 8B
    packed_header = struct.pack(
        "<4sHHIQ",
        PROTOCOL_MAGIC,
        PROTOCOL_VERSION,
        message_type,
        len(payload),
        request_id,
    )
    return packed_header + payload

def decode_frame(buf: bytes):
    if len(buf) < FRAME_HEADER_SIZE:
        return None, 0

    magic, version, msg_type, payload_len, req_id = struct.unpack(
        "<4sHHIQ", buf[:FRAME_HEADER_SIZE]
    )

    if magic != PROTOCOL_MAGIC:
        raise ValueError("Invalid protocol magic bytes")

    total_len = FRAME_HEADER_SIZE + payload_len
    if len(buf) < total_len:
        return None, 0

    payload = buf[FRAME_HEADER_SIZE:total_len]
    return (msg_type, req_id, payload), total_len
