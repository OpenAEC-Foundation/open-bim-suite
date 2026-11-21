#!/usr/bin/env python3
"""Extracts the generic 2D components of Project Ocondat for `bim-library`.

    python3 tools/ocondat-components/extract.py PATH/TO/project-ocondat COMMIT

reads Ocondat's own index (`DXF Library/component-library/data/components.json`) and writes
`crates/bim-library/data/ocondat-components.json`: every component of the maker `Generic`,
grouped by its series. Run it again when Ocondat changes; nothing is copied by hand.

- Only `Generic` is taken: Ocondat gives it the LGPL-3.0. What a maker supplied (Halfen,
  ArcelorMittal: "free for engineering use") and the community's sets ("mixed") are counted
  in `docs/library/ocondat-inventory.md` and not taken over until their terms are checked.
- A series gives its components their IFC class, predefined type and NL-SfB code, as the
  index has them; a component keeps its id, its name, its size on paper (millimetres) and
  the path of its DXF in Ocondat.
- The drawings themselves are not copied: the catalogue points at them.
"""
import json
import sys
from pathlib import Path

MAKER = "Generic"
INDEX = "DXF Library/component-library/data/components.json"


def main() -> None:
    root, commit = Path(sys.argv[1]), sys.argv[2]
    index = json.loads((root / INDEX).read_text(encoding="utf-8"))
    series: dict[str, dict] = {}
    left: dict[str, int] = {}
    for c in index["components"]:
        if c["manufacturer"] != MAKER:
            left[c["manufacturer"]] = left.get(c["manufacturer"], 0) + 1
            continue
        code = c["classification"]["nl_sfb"]
        s = series.setdefault(c["serie"], {
            "name": c["serie"],
            "names": {"nl": c["classification"]["description_nl"], "en": c["classification"]["description_en"]},
            "class": c["ifc_class"],
            "predefined": c["ifc_predefined_type"],
            "nlsfb": code,
            "standard": c["properties"].get("NL", {}).get("standard", ""),
            "components": [],
        })
        same = (s["class"], s["predefined"], s["nlsfb"]) == (c["ifc_class"], c["ifc_predefined_type"], code)
        assert same, f"{c['id']}: a series has one class and one code"
        prefix = f"components/{MAKER}/{c['serie']}/"
        assert c["dxf_source"].startswith(prefix), c["dxf_source"]
        assert (root / "DXF Library" / c["dxf_source"]).is_file(), c["dxf_source"]
        g = c["geometry"]
        s["components"].append({
            "id": c["id"],
            "name": c["name"],
            "size": [round(g.get("width_mm") or 0.0, 1), round(g.get("height_mm") or 0.0, 1)],
            "dxf": c["dxf_source"][len(prefix):],
        })
    out = {
        "source": f"Project Ocondat of the OpenAEC Foundation (github.com/OpenAEC-Foundation/project-ocondat), commit {commit}: {INDEX}, the components of the maker {MAKER}",
        "licence": "LGPL-3.0",
        "maker": MAKER.lower(),
        "unit": "a component's size is its drawing's width and height in millimetres; its dxf is its path under DXF Library/components/Generic/<series>/ in Ocondat",
        "not_taken": dict(sorted(left.items())),
        "series": sorted(series.values(), key=lambda s: (s["nlsfb"], s["name"])),
    }
    for s in out["series"]:
        s["components"].sort(key=lambda c: c["id"])
    target = Path(__file__).resolve().parents[2] / "crates/bim-library/data/ocondat-components.json"
    lines = ["{"]
    for key in ["source", "licence", "maker", "unit", "not_taken"]:
        lines.append(f"  {json.dumps(key)}: {json.dumps(out[key], ensure_ascii=False)},")
    lines.append('  "series": [')
    for i, s in enumerate(out["series"]):
        head = {k: v for k, v in s.items() if k != "components"}
        lines.append("    " + json.dumps(head, ensure_ascii=False)[:-1] + ', "components": [')
        lines += ["      " + json.dumps(c, ensure_ascii=False) + ("," if j + 1 < len(s["components"]) else "") for j, c in enumerate(s["components"])]
        lines.append("    ]}" + ("," if i + 1 < len(out["series"]) else ""))
    lines += ["  ]", "}"]
    target.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"{sum(len(s['components']) for s in out['series'])} components in {len(out['series'])} series; not taken: {out['not_taken']}")


if __name__ == "__main__":
    main()
