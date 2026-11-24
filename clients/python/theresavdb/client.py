import json
import socket
from typing import Any, Dict, List
from .protocol import (
    MessageType,
    decode_frame,
    encode_frame,
)

class QueryResult:
    def __init__(self, columns: List[str], rows: List[List[Any]], rows_affected: int):
        self.columns = columns
        self.rows = rows
        self.rows_affected = rows_affected

    def __repr__(self):
        return f"<QueryResult rows={len(self.rows)} affected={self.rows_affected}>"

class Client:
    def __init__(self, host: str = "127.0.0.1", port: int = 7432):
        self.host = host
        self.port = port
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.connect((self.host, self.port))
        self.req_id = 1

    def query(self, sql: str) -> QueryResult:
        current_req = self.req_id
        self.req_id += 1

        payload = json.dumps({"sql": sql}).encode("utf-8")
        frame_bytes = encode_frame(MessageType.QUERY, current_req, payload)
        self.sock.sendall(frame_bytes)

        buffer = b""
        columns = []
        rows = []
        rows_affected = 0

        while True:
            chunk = self.sock.recv(4096)
            if not chunk:
                break
            buffer += chunk

            while len(buffer) > 0:
                decoded, bytes_read = decode_frame(buffer)
                if not decoded:
                    break
                buffer = buffer[bytes_read:]
                msg_type, req_id, payload_data = decoded

                if msg_type == MessageType.ROW_DESCRIPTION:
                    desc = json.loads(payload_data.decode("utf-8"))
                    columns = desc.get("columns", [])
                elif msg_type == MessageType.DATA_ROW:
                    data = json.loads(payload_data.decode("utf-8"))
                    rows.append(data.get("values", []))
                elif msg_type == MessageType.COMMAND_COMPLETE:
                    comp = json.loads(payload_data.decode("utf-8"))
                    rows_affected = comp.get("rows_affected", 0)
                    return QueryResult(columns, rows, rows_affected)
                elif msg_type == MessageType.ERROR:
                    err = json.loads(payload_data.decode("utf-8"))
                    raise RuntimeError(f"[{err.get('code')}] {err.get('message')}")

    def close(self):
        self.sock.close()
