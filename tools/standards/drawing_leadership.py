"""Extract the four leadership chevrons from MIL Figure 28 (printed p. 47)."""
from drawing_config import SOURCES, WORK
import json
import pymupdf as fitz

page = fitz.open(SOURCES['mil2525e1'])[63]
drawings = [d for d in page.get_drawings() if 510 < d['rect'].y0 < d['rect'].y1 < 575]
frames = [d for d in drawings if d['fill'] and d['fill'] != (0, 0, 0)]
frames.sort(key=lambda d: d['rect'].x0)
chevrons = [d for d in drawings if len(d['items']) == 2 and all(i[0] == 'l' for i in d['items'])]
chevrons.sort(key=lambda d: d['rect'].x0)
assert len(frames) == len(chevrons) == 4
out = []
for name, frame, chevron in zip(['Friend', 'Hostile', 'Neutral', 'Unknown'], frames, chevrons):
    box = frame['rect']
    points = [chevron['items'][0][1], chevron['items'][0][2], chevron['items'][1][2]]
    normalized = [[round((p.x-box.x0)/box.width, 9), round((p.y-box.y0)/box.height, 9)] for p in points]
    out.append({'affiliation': name, 'points': normalized, 'document': 'mil2525e1', 'page': 47, 'figure': '28'})
(WORK / 'leadership.json').write_text(json.dumps(out, indent=2))
