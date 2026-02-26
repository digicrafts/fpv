from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


@dataclass(slots=True)
class Item:
    name: str
    score: float


def top_items(items: Iterable[Item], limit: int = 3) -> list[Item]:
    ordered = sorted(items, key=lambda i: i.score, reverse=True)
    return ordered[:limit]


def render_markdown(items: list[Item]) -> str:
    lines = ["# Ranking", ""]
    lines.extend(f"- **{i.name}**: `{i.score:.2f}`" for i in items)
    return "\n".join(lines)


if __name__ == "__main__":
    dataset = [
        Item("alpha", 9.2),
        Item("beta", 7.8),
        Item("gamma", 8.6),
        Item("delta", 9.5),
    ]
    output = render_markdown(top_items(dataset))
    Path("ranking.md").write_text(output, encoding="utf-8")
    print(output)
