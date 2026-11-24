"""TheresavDB Client Library for Python."""

from .client import Client, QueryResult

def connect(host: str = "127.0.0.1", port: int = 7432) -> Client:
    """Connect to a running TheresavDB instance."""
    return Client(host, port)

__all__ = ["Client", "QueryResult", "connect"]
