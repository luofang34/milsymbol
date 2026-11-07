from drawing_config import WORK, ROOT, SOURCES
import pymupdf as fitz, importlib.util,json,csv,collections,sys
from pathlib import Path
spec=importlib.util.spec_from_file_location('paths',str(Path(__file__).with_name('drawing_paths.py')));paths=importlib.util.module_from_spec(spec);spec.loader.exec_module(paths)
sources={'mil2525e1':SOURCES['mil2525e1'],'app6e2':SOURCES['app6e2']}
matched=json.load(open(str(WORK / 'matched.json')))

def rgb(c):
 if c=='none' or c is None:return None
 if c=='black':return (0,0,0)
 if c=='white':return (255,255,255)
 if c.startswith('#') and len(c)==7:return tuple(int(c[i:i+2],16) for i in [1,3,5])
 raise ValueError(c)
def blue(c):return c and c[2]>c[0]+40 and c[2]>c[1]+40

def blue_bounds(e):
 if 'image' in e:
  im=paths.Image.open(paths.io.BytesIO(paths.base64.b64decode(e['image'].split(',')[1]))).convert('RGB');a=paths.np.array(im).astype(int)
  yy,xx=paths.np.nonzero((a[:,:,2]>a[:,:,0]+40)&(a[:,:,2]>a[:,:,1]+40))
  if not len(xx):return None
  m=paths.Transform(*e['matrix']);return fitz.Rect(*m.transformPoint((int(xx.min()),int(yy.min()))),*m.transformPoint((int(xx.max()),int(yy.max()))))
 if blue(rgb(e['fill'])) or blue(rgb(e['stroke'])):return fitz.Rect(e['bbox'])
 return None

def convert(page,row,elements):
 d=row['drawing'];top,bottom=d['y'];metoc=row['symbol_set'] in ['45','46']
 white_filled='white filled' in ' '.join(d['text'].lower().split())
 possible=[e for e in elements if 220<(e['bbox'][0]+e['bbox'][2])/2<430 and e['bbox'][1]>top-.5 and e['bbox'][3]<bottom+.5]
 bb=[b for e in possible if (b:=blue_bounds(e)) is not None and b.height>30 and b.width>30]
 if bb:octagon=max(bb,key=lambda b:b.width*b.height)
 else:
  left,right=180,430
  pix=page.get_pixmap(matrix=fitz.Matrix(4,4),clip=fitz.Rect(left,top+1,right,bottom-1),alpha=False)
  a=paths.np.frombuffer(pix.samples,dtype=paths.np.uint8).reshape(pix.height,pix.width,pix.n).astype(int)
  yy,xx=paths.np.nonzero((a[:,:,2]>a[:,:,0]+50)&(a[:,:,2]>a[:,:,1]+50))
  if not len(xx):raise ValueError('no octagon')
  octagon=fitz.Rect((pix.x+xx.min())/4,(pix.y+yy.min())/4,(pix.x+xx.max())/4,(pix.y+yy.max())/4)

 cx,cy=(octagon.x0+octagon.x1)/2,(octagon.y0+octagon.y1)/2
 borders=[e['rect'].x0 for e in page.get_drawings() if e['rect'].width<1.6 and e['rect'].y0<=top+4 and e['rect'].y1>=bottom-4]
 left=max([x for x in borders if x<cx],default=octagon.x0-20)+1.5
 right=min([x for x in borders if x>cx],default=octagon.x1+20)-1.5
 possible=[e for e in elements if e['bbox'][0]>left and e['bbox'][2]<right and e['bbox'][1]>top-.5 and e['bbox'][3]<bottom+.5]
 scale=100/octagon.width;normal=paths.Transform(scale,0,0,scale,100-cx*scale,100-cy*scale)
 result=[]
 for e in possible:
  if 'image' in e:
   im=paths.Image.open(paths.io.BytesIO(paths.base64.b64decode(e['image'].split(',')[1])))
   if white_filled:
    white=paths.white_interiors(im)
    if white:result.append({'d':paths.transform_path(white,normal.transform(paths.Transform(*e['matrix']))),'fill':'#ffffff','stroke':None,'width':0})
   d=paths.image_paths(im);fill='icon';stroke=None
  else:
   if blue(rgb(e['fill'])) or blue(rgb(e['stroke'])):continue
   fill,stroke=e['fill'],e['stroke']
   c=rgb(fill) or rgb(stroke)
   if c and 112<min(c)<250:continue
   if fill in ['white','#ffffff'] and not stroke and not white_filled:continue
   d=e['d'];fill='icon' if rgb(fill) and max(rgb(fill))<112 else fill
   stroke='icon' if rgb(stroke) and max(rgb(stroke))<112 else stroke
  if not d:continue
  transformed=paths.transform_path(d,normal.transform(paths.Transform(*e['matrix'])))
  result.append({'d':transformed,'fill':fill,'stroke':stroke,'width':round(e['width']*scale,6)})
 if not result:raise ValueError('no graphic')
 layout='Main' if 'Icon Type: Main' in row['drawing']['text'] else 'FullFrame' if 'Icon Type: Full Frame' in row['drawing']['text'] else 'Fixed'
 return {'layout':layout,'source': {'page':row['drawing']['page'],'table':row['drawing']['table'],'printed':row['drawing']['printed'],'octagon':list(octagon)},'paths':result}

required=set(json.load(open(ROOT / 'tools/standards/drawing-keys.json')))
out={};fail=[]
for name,rows in matched.items():
 doc=fitz.open(sources[name]);cache={}
 for row in rows:
  key=f"{row['symbol_set']} {row['kind']} {row['code']}"
  # Actual missing/changed symbols; meteorological and unframed graphics use a separate extractor.
  if name+' '+key not in required:continue
  if not row['drawing']:fail.append([name,key,'no diagram']);continue
  num=row['drawing']['page'];page=doc[num-1]
  if num not in cache:cache[num]=paths.read(page.get_svg_image(text_as_path=True))
  try:out[name+' '+key]=convert(page,row,cache[num]);print('OK',name,key,len(out[name+' '+key]['paths']),flush=True)
  except Exception as e:fail.append([name,key,str(e)]);print('FAIL',name,key,e,flush=True)
Path(str(WORK / 'icons.json')).write_text(json.dumps(out))
Path(str(WORK / 'icons-fail.json')).write_text(json.dumps(fail,indent=2))
print('DONE',len(out),'failed',len(fail))
