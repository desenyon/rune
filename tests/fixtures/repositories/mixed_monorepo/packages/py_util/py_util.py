"""Python side of the mixed monorepo fixture."""


def normalize_name(value: str) -> str:
    return value.strip().lower()
