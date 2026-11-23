#!/usr/bin/env python3
"""Extracts the drawings of the generic 2D components of Project Ocondat for `bim-library`.

    python3 tools/ocondat-components/drawings.py PATH/TO/project-ocondat COMMIT [MOST]

reads the DXF of every component in `crates/bim-library/data/ocondat-components.json` and
writes `crates/bim-library/data/ocondat-drawings.json`: per component its lines, as runs of
points in millimetres, and its arcs. Run it after `extract.py`.

- A component's DXF holds one block and an insert of it; the block's lines, arcs, circles,
  polylines (a bulge is drawn as its arc), ellipses and splines (through their fit or control
  points) are taken, and an insert of another block is followed. Hatches, points and texts are
  left out.
- Coordinates are moved so the drawing's lower left corner is at the origin, and rounded to a
  tenth of a millimetre.
- Only drawings of at most MOST numbers (default 400) are compiled in, so the catalogue stays
  small: the others are named in `left_out` and stay in Ocondat.
"""
import json
import math
import sys
from pathlib import Path

CATALOGUE = "crates/bim-library/data/ocondat-components.json"
OUT = "crates/bim-library/data/ocondat-drawings.json"
MAKER = "Generic"


def pairs(path: Path):
    lines = path.read_text(encoding="utf-8", errors="replace").split("\n")
    return [(lines[i].strip(), lines[i + 1].strip()) for i in range(0, len(lines) - 1, 2)]


def entities(tags):
    """The entities of a run of tags: (kind, [(code, value), ...])."""
    out, now = [], None
    for code, value in tags:
        if code == "0":
            now = (value, [])
            out.append(now)
        elif now is not None:
            now[1].append((code, value))
    return out


def blocks_and_inserts(path: Path):
    section, name, blocks, top, run = None, None, {}, [], []
    tags = pairs(path)
    i = 0
    while i < len(tags):
        code, value = tags[i]
        if code == "0" and value == "SECTION":
            section = tags[i + 1][1]
            i += 2
            continue
        if code == "0" and value == "ENDSEC":
            if section == "ENTITIES":
                top = entities(run)
            section, run = None, []
        elif section == "BLOCKS":
            if code == "0" and value == "BLOCK":
                name, run = None, []
            elif code == "0" and value == "ENDBLK":
                if name is not None:
                    blocks[name] = entities(run)
                name, run = None, []
            elif name is None and code == "2":
                name, run = value, []
            elif name is not None:
                run.append((code, value))
        elif section == "ENTITIES":
            run.append((code, value))
        i += 1
    return blocks, top


def first(tags, code, default=0.0):
    for c, v in tags:
        if c == code:
            return float(v)
    return default


def every(tags, code):
    return [float(v) for c, v in tags if c == code]


def bulge_arc(p, q, bulge):
    """The arc from p to q of a polyline's bulge: (cx, cy, r, from, to) in degrees, anticlockwise."""
    chord = math.dist(p, q)
    if chord < 1e-9:
        return None
    angle = 4 * math.atan(bulge)
    r = chord / (2 * math.sin(abs(angle) / 2))
    mid = ((p[0] + q[0]) / 2, (p[1] + q[1]) / 2)
    h = math.sqrt(max(r * r - (chord / 2) ** 2, 0.0))
    nx, ny = -(q[1] - p[1]) / chord, (q[0] - p[0]) / chord
    side = 1 if (bulge > 0) == (abs(angle) < math.pi) else -1
    c = (mid[0] + side * nx * h, mid[1] + side * ny * h)
    a, b = math.degrees(math.atan2(p[1] - c[1], p[0] - c[0])), math.degrees(math.atan2(q[1] - c[1], q[0] - c[0]))
    return (c[0], c[1], r, a, b) if bulge > 0 else (c[0], c[1], r, b, a)


def shapes(blocks, items, depth=0):
    """The runs of points and the arcs of a list of entities, inserts followed."""
    runs, arcs = [], []
    for kind, tags in items:
        if kind == "LINE":
            runs.append([(first(tags, "10"), first(tags, "20")), (first(tags, "11"), first(tags, "21"))])
        elif kind == "ARC":
            arcs.append((first(tags, "10"), first(tags, "20"), first(tags, "40"), first(tags, "50"), first(tags, "51")))
        elif kind == "CIRCLE":
            arcs.append((first(tags, "10"), first(tags, "20"), first(tags, "40"), 0.0, 360.0))
        elif kind == "LWPOLYLINE":
            xs, ys = every(tags, "10"), every(tags, "20")
            points = list(zip(xs, ys))
            bulges, at = [0.0] * len(points), -1
            for code, value in tags:
                if code == "10":
                    at += 1
                elif code == "42" and at >= 0:
                    bulges[at] = float(value)
            closed = int(first(tags, "70", 0)) & 1
            if closed and points:
                points.append(points[0])
            run = []
            for k in range(len(points) - 1):
                p, q, bulge = points[k], points[k + 1], bulges[k] if k < len(bulges) else 0.0
                if abs(bulge) > 1e-9 and (arc := bulge_arc(p, q, bulge)):
                    if len(run) > 1:
                        runs.append(run)
                    run = []
                    arcs.append(arc)
                else:
                    run = run or [p]
                    run.append(q)
            if len(run) > 1:
                runs.append(run)
        elif kind == "SPLINE":
            xs, ys = every(tags, "11") or every(tags, "10"), every(tags, "21") or every(tags, "20")
            if len(xs) > 1:
                runs.append(list(zip(xs, ys)))
        elif kind == "ELLIPSE":
            cx, cy, mx, my, ratio = first(tags, "10"), first(tags, "20"), first(tags, "11"), first(tags, "21"), first(tags, "40", 1.0)
            a0, a1 = first(tags, "41", 0.0), first(tags, "42", 2 * math.pi)
            major, turn = math.hypot(mx, my), math.atan2(my, mx)
            steps = 24
            run = []
            for k in range(steps + 1):
                t = a0 + (a1 - a0) * k / steps
                x, y = major * math.cos(t), major * ratio * math.sin(t)
                run.append((cx + x * math.cos(turn) - y * math.sin(turn), cy + x * math.sin(turn) + y * math.cos(turn)))
            runs.append(run)
        elif kind == "INSERT" and depth < 4:
            name = next((v for c, v in tags if c == "2"), None)
            if name not in blocks:
                continue
            ox, oy = first(tags, "10"), first(tags, "20")
            sx, sy, turn = first(tags, "41", 1.0), first(tags, "42", 1.0), math.radians(first(tags, "50", 0.0))
            inner_runs, inner_arcs = shapes(blocks, blocks[name], depth + 1)
            def place(p):
                x, y = p[0] * sx, p[1] * sy
                return (ox + x * math.cos(turn) - y * math.sin(turn), oy + x * math.sin(turn) + y * math.cos(turn))
            runs.extend([place(p) for p in run] for run in inner_runs)
            for cx, cy, r, a, b in inner_arcs:
                # (an arc keeps its shape under one scale only; a mirrored one swaps its ends)
                if abs(abs(sx) - abs(sy)) > 1e-9:
                    continue
                c = place((cx, cy))
                a, b = (a, b) if sx * sy > 0 else (180 - b, 180 - a) if sx < 0 else (-b, -a)
                arcs.append((c[0], c[1], r * abs(sx), a + math.degrees(turn), b + math.degrees(turn)))
    return runs, arcs


def tidy(value: float):
    """A tenth of a millimetre, a whole number written as one."""
    value = round(value, 1)
    return int(value) if value == int(value) else value


def chained(lines):
    """Runs joined where one ends where the next begins, so a drawn outline is one run."""
    out = []
    for line in lines:
        if out and out[-1][-2:] == line[:2]:
            out[-1].extend(line[2:])
        else:
            out.append(list(line))
    return out


def main() -> None:
    root, commit = Path(sys.argv[1]), sys.argv[2]
    most = int(sys.argv[3]) if len(sys.argv) > 3 else 400
    catalogue = json.loads(Path(CATALOGUE).read_text(encoding="utf-8"))
    drawings, left_out, empty = {}, [], []
    for series in catalogue["series"]:
        for c in series["components"]:
            path = root / "DXF Library" / "components" / MAKER / series["name"] / c["dxf"]
            blocks, top = blocks_and_inserts(path)
            runs, arcs = shapes(blocks, top)
            if not runs and not arcs:
                empty.append(c["id"])
                continue
            xs = [p[0] for run in runs for p in run] + [a[0] - a[2] for a in arcs]
            ys = [p[1] for run in runs for p in run] + [a[1] - a[2] for a in arcs]
            x0, y0 = min(xs), min(ys)
            lines = chained([[tidy(v) for p in run for v in (p[0] - x0, p[1] - y0)] for run in runs])
            rounds = [[tidy(a[0] - x0), tidy(a[1] - y0), tidy(a[2]), tidy(a[3] % 360), tidy(a[4] % 360 or 360.0)] for a in arcs]
            if sum(len(line) for line in lines) + 5 * len(rounds) > most:
                left_out.append(c["id"])
                continue
            drawings[c["id"]] = {"lines": lines, "arcs": rounds} if rounds else {"lines": lines}
    out = {
        "source": f"Project Ocondat of the OpenAEC Foundation (github.com/OpenAEC-Foundation/project-ocondat), commit {commit}: DXF Library/components/{MAKER}",
        "licence": catalogue["licence"],
        "units": "millimetres; a line is x, y, x, y, ...; an arc is centre x, centre y, radius, from and to in degrees anticlockwise",
        "most": most,
        "left_out": len(left_out),
        "empty": len(empty),
        "drawings": drawings,
    }
    text = json.dumps(out, ensure_ascii=False, separators=(",", ":"))
    # one drawing a line, so a change of Ocondat shows as the lines that changed
    text = text.replace('"drawings":{', '"drawings":{\n').replace('},"gen-', '},\n"gen-')
    Path(OUT).write_text(text + "\n", encoding="utf-8")
    print(f"{len(drawings)} drawings, {len(left_out)} left out (more than {most} numbers), {len(empty)} empty, {len(text) // 1024} kB")


if __name__ == "__main__":
    main()
