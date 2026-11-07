"""Single-point control graphics; explicit image crops remove template labels."""

import json
from collections import deque
from pathlib import Path

import pymupdf as fitz
from PIL import ImageFilter

import drawing_paths as paths
from drawing_config import ROOT, SOURCES, WORK


def border_without_colour_labels(mask):
    """Close the narrow white RED lettering without closing the shield interior."""
    ink = paths.Image.fromarray(paths.np.where(mask, 255, 0).astype("uint8"))
    closed = ink.filter(ImageFilter.MaxFilter(5)).filter(ImageFilter.MinFilter(5))
    return largest_component(paths.np.array(closed) > 0)


def image_graphic(image, spec):
    if "crop" in spec:
        image = image.crop(spec["crop"])
    data = paths.np.array(image.convert("RGB")).astype(int)
    if spec["table"] == "8-23":
        blue = (data[:, :, 2] > data[:, :, 0] + 20) & (data[:, :, 0] < 160)
        layers = [(blue, "#0000ff")]
        if spec.get("red_border"):
            black = (paths.np.max(data, axis=2) < 110) & ~blue
            layers.insert(0, (border_without_colour_labels(black), "#ff0000"))
        return [(paths.image_paths(paths.Image.fromarray(paths.np.where(mask, 0, 255).astype("uint8"))), color)
                for mask, color in layers]
    return paths.image_layers(image, "icon")


def largest_component(mask):
    """Keep the border, excluding the source's detached anchor-point arrow."""
    seen = set()
    largest = []
    height, width = mask.shape
    for y, x in zip(*paths.np.nonzero(mask)):
        if (y, x) in seen:
            continue
        queue = deque([(y, x)])
        seen.add((y, x))
        component = []
        while queue:
            yy, xx = queue.popleft()
            component.append((yy, xx))
            for ny, nx in [(yy-1, xx), (yy+1, xx), (yy, xx-1), (yy, xx+1)]:
                if 0 <= ny < height and 0 <= nx < width and mask[ny, nx] and (ny, nx) not in seen:
                    seen.add((ny, nx))
                    queue.append((ny, nx))
        if len(component) > len(largest):
            largest = component
    output = paths.np.zeros(mask.shape, dtype=bool)
    for y, x in largest:
        output[y, x] = True
    return output


matched = json.loads((WORK / "matched.json").read_text())
specs = json.loads((ROOT / "tools/standards/drawing-controls.json").read_text())
output = {}
for key, spec in specs.items():
    document, symbol_set, kind, code = key.split()
    row = next(r for r in matched[document]
               if (r["symbol_set"], r["kind"], r["code"]) == (symbol_set, kind, code))
    drawing = row["drawing"]
    assert drawing["printed"] == spec["page"], key
    page = fitz.open(SOURCES[document])[drawing["page"] - 1]
    elements = paths.read(page.get_svg_image(text_as_path=True))
    shapes = []
    if "rect" in spec:
        left, top, right, bottom = spec["rect"]
        for e in elements:
            x1, y1, x2, y2 = e["bbox"]
            if x1 > left and x2 < right and y1 > top and y2 < bottom:
                assert "image" not in e
                shapes.append({"d": paths.transform_path(e["d"], paths.Transform(*e["matrix"])),
                               "fill": "icon" if e["fill"] != "none" else None,
                               "stroke": "icon" if e["stroke"] else None, "width": e["width"]})
    else:
        top, bottom = drawing["y"]
        candidates = [e for e in elements if "image" in e and e["bbox"][1] > top
                      and e["bbox"][3] < bottom]
        candidates.sort(key=lambda e: e["bbox"][0])
        e = candidates[-1] if spec["column"] == "example" else candidates[0]
        image = paths.Image.open(paths.io.BytesIO(paths.base64.b64decode(e["image"].split(",")[1])))
        shapes = [{"d": d, "fill": color, "stroke": None, "width": 0}
                  for d, color in image_graphic(image, spec)]
    assert shapes, key
    boxes = [paths.bounds(s["d"], paths.IDENTITY) for s in shapes]
    x1, y1 = min(b[0] for b in boxes), min(b[1] for b in boxes)
    x2, y2 = max(b[2] for b in boxes), max(b[3] for b in boxes)
    scale = 100 / max(x2-x1, y2-y1)
    anchor_y = y2 if spec["anchor"] == "bottom" else (y1+y2)/2
    normal = paths.Transform(scale, 0, 0, scale, 100-(x1+x2)/2*scale, 100-anchor_y*scale)
    for shape in shapes:
        shape["d"] = paths.transform_path(shape["d"], normal)
        shape["width"] *= scale
    output[key] = {"source": {"document": document, "printed": spec["page"], "table": spec["table"]},
                   "paths": shapes}
(WORK / "controls-icons.json").write_text(json.dumps(output))
print(f"Converted {len(output)} single-point control graphics")
