"""Greeting helpers for the Python indexer fixture."""


def greet(name: str) -> str:
    if not name.strip():
        raise ValueError("name is required")
    return f"hello {name}"


def parse_port(value: str) -> int:
    port = int(value)
    if port <= 0 or port > 65535:
        raise ValueError("port out of range")
    return port
