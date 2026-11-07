from drawing_config import WORK, ROOT, SOURCES
import importlib.util,json,re
from pathlib import Path
import pymupdf as fitz
spec=importlib.util.spec_from_file_location('paths',str(Path(__file__).with_name('drawing_paths.py')));paths=importlib.util.module_from_spec(spec);spec.loader.exec_module(paths)
doc=fitz.open(SOURCES['mil2525e1'])
rows=json.load(open(str(WORK / 'matched.json')))['mil2525e1'];out={};fail=[];cache={}
root=Path(str(WORK / 'metoc'));root.mkdir(exist_ok=True)
inputs=json.load(open(ROOT / 'tools/standards/drawing-inputs.json'))
for row in rows:
 if row['symbol_set'] not in ['45','46'] or row['scope']!='point':continue
 key=f"mil2525e1 {row['symbol_set']} {row['kind']} {row['code']}";r=row['drawing'];page=doc[r['page']-1];top,bottom=r['y']
 if r['page'] not in cache:cache[r['page']]=paths.read(page.get_svg_image(text_as_path=True))
 elements=cache[r['page']]
 heading=[w for w in page.get_text('words') if w[4]=='ICON' and 220<w[0]<420 and w[1]<top]
 if not heading:fail.append((key,'no column'));continue
 word=max(heading,key=lambda w:w[1]);cx=(word[0]+word[2])/2;cy=(top+bottom)/2
 borders=[d['rect'].x0 for d in page.get_drawings() if d['rect'].width<1.6 and d['rect'].y0<=top+4 and d['rect'].y1>=bottom-4]
 left=max([x for x in borders if x<cx],default=cx-65)+1.5;right=min([x for x in borders if x>cx],default=cx+65)-1.5
 possible=[e for e in elements if e['bbox'][0]>left and e['bbox'][2]<right and e['bbox'][1]>top-.5 and e['bbox'][3]<bottom+.5]
 shapes=[e for e in possible if 'image' in e or e.get('fill') not in ['#ffffff','white'] or e.get('stroke')]
 if not shapes:fail.append((key,'no graphic'));continue
 images=[e for e in shapes if 'image' in e]
 reference=fitz.Rect(images[0]['bbox']) if len(images)==1 else fitz.Rect(min(e['bbox'][0] for e in shapes),min(e['bbox'][1] for e in shapes),max(e['bbox'][2] for e in shapes),max(e['bbox'][3] for e in shapes))
 factor=100/max(reference.width,reference.height);normal=paths.Transform(factor,0,0,factor,100-(reference.x0+reference.x1)/2*factor,100-(reference.y0+reference.y1)/2*factor)
 color='#000000';m=re.search(r'Color:\s*(\w+)',r['text']);rgb=re.search(r'RGB\s+(\d+)\s*,\s*(\d+)\s*,\s*(\d+)',r['text'])
 if rgb:color='#'+''.join(f'{int(v):02x}' for v in rgb.groups())
 elif m and m[1].lower() in ['blue','red','green','brown','gray','grey','magenta']:color=m[1].lower()
 output=[]
 for e in shapes:
  if 'image' in e:
   im=paths.Image.open(paths.io.BytesIO(paths.base64.b64decode(e['image'].split(',')[1]))).convert('RGB');a=paths.np.array(im)
   spec=inputs.get(row['symbol_set']+' '+row['code'],{})
   if 'erase' in spec:
    assert im.size==(170,170),(key,im.size)
    im.paste('white',tuple(spec['erase']))
   layers=paths.image_layers(im,color)
   for d,fill in layers:
    output.append({'d':paths.transform_path(d,normal.transform(paths.Transform(*e['matrix']))),'fill':fill,'stroke':None,'width':0})
   im.save(root/(row['symbol_set']+'-'+row['code']+'.png'))
   continue
  else:
   d=e['d'];fill=None if e['fill']=='none' else color;stroke=color if e['stroke'] else None
  if d:output.append({'d':paths.transform_path(d,normal.transform(paths.Transform(*e['matrix']))),'fill':fill,'stroke':stroke,'width':round(e['width']*factor,6)})
 if not output and row['symbol_set']+' '+row['code'] not in inputs:fail.append((key,'empty'));continue
 out[key]={'source':{'page':r['page'],'printed':r['printed'],'table':r['table'],'reference':list(reference)},'paths':output,'input':inputs.get(row['symbol_set']+' '+row['code'])}
 print('OK',key,len(output),color,flush=True)
Path(str(WORK / 'metoc-icons.json')).write_text(json.dumps(out));Path(str(WORK / 'metoc-fail.json')).write_text(json.dumps(fail,indent=2));print('DONE',len(out),'failed',len(fail))
