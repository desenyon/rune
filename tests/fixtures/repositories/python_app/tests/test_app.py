from python_app import greet, parse_port
import pytest


def test_greet():
    assert greet("rune") == "hello rune"


def test_greet_rejects_blank():
    with pytest.raises(ValueError):
        greet("  ")


def test_parse_port():
    assert parse_port("8080") == 8080
